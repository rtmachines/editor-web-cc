//! `GET /machines/{code}/connection/edit`: the form of the PLC connection of a machine
//! (`api.md` section 2.2, ADR-0154, design D8).
//!
//! The form shows only the fields of the platform of the machine. The password input never has a
//! `value`, because the query `GetPlcConnectionForm` holds no password (R31, A47).

use askama::Template;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::Operation;
use recipe_app::plant::{
    GetPlcConnectionForm, PlcConnectionForm, PlcConnectionFormInput, SetPlcConnectionOutput,
};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::{CommMode, MachineCode, PlcConnection, PlcPlatform};

use crate::error::{FieldError, FormErrors, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::draft_reply::refused;
use crate::pages::path_values::path_code;
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The communication modes that a platform with an IEC connection permits, in the order of the
/// list. The domain rule is `PlcConnection::for_platform` (`E-PLT-025`). An S7 platform has none.
const fn modes_of(platform: PlcPlatform) -> &'static [CommMode] {
    match platform {
        PlcPlatform::CodesysIec => &[CommMode::OpcUa, CommMode::CodesysV3, CommMode::ModbusTcp],
        PlcPlatform::TwinCat3 => &[CommMode::Ads, CommMode::OpcUa, CommMode::ModbusTcp],
        PlcPlatform::S7_300 | PlcPlatform::S7_1200 | PlcPlatform::S7_1500 => &[],
    }
}

/// Return the key of the text of a communication mode in the bundles.
pub(super) const fn mode_key(mode: CommMode) -> &'static str {
    match mode {
        CommMode::Ads => "connection-mode-ads",
        CommMode::OpcUa => "connection-mode-opcua",
        CommMode::CodesysV3 => "connection-mode-codesysv3",
        CommMode::ModbusTcp => "connection-mode-modbustcp",
    }
}

/// One option of the list of communication modes.
struct ModeOption {
    value: &'static str,
    text: String,
}

/// The data of the connection form: the shape of the platform, the stored values, and the texts.
///
/// The type solves one problem: one template shows the S7 shape and the IEC shape. It has no
/// field for a password, so the template cannot show one.
struct ConnectionFormView {
    action: String,
    csrf_token: String,
    row_version: i64,
    is_s7: bool,
    host: String,
    rack: String,
    slot: String,
    port: String,
    address: String,
    modes: Vec<ModeOption>,
    mode: String,
    login: String,
    /// The hint of the password input. It tells whether the record holds a password.
    password_hint: String,
    /// True when the refused request held a password. The form then asks to type it again.
    password_reenter: bool,
    /// True when read-only mode blocks the form, so the notice stands in its place (ADR-0113 (d)).
    blocked: bool,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    oob: bool,
    /// The error lines under the inputs. A form without errors has none.
    errors: FormErrors,
}

/// The texts of the inputs of the form: the stored record, or the texts that a refused request
/// sent. A machine without a record gives empty texts. The type has no password.
#[derive(Default)]
pub(super) struct InputTexts {
    pub(super) host: String,
    pub(super) rack: String,
    pub(super) slot: String,
    pub(super) port: String,
    pub(super) address: String,
    pub(super) mode: String,
    pub(super) login: String,
}

impl InputTexts {
    /// Turn the stored record into the texts of the inputs.
    fn of(connection: Option<&PlcConnection>) -> Self {
        match connection {
            None => Self::default(),
            Some(PlcConnection::S7 { host, rack, slot }) => Self {
                host: host.to_string(),
                rack: rack.value().to_string(),
                slot: slot.value().to_string(),
                ..Self::default()
            },
            Some(PlcConnection::Iec {
                host,
                port,
                address,
                mode,
                login,
            }) => Self {
                host: host.to_string(),
                port: port.value().to_string(),
                address: address.to_string(),
                mode: mode.as_str().to_owned(),
                login: login.as_ref().map(ToString::to_string).unwrap_or_default(),
                ..Self::default()
            },
        }
    }
}

impl ConnectionFormView {
    /// Build the form from the answer of the query.
    fn new(t: Texts<'_>, form: &PlcConnectionForm, csrf_token: String, blocked: bool) -> Self {
        let stored = InputTexts::of(form.connection.as_ref());
        // The empty first option is selected, so the user chooses the mode on a first save.
        let modes = std::iter::once(ModeOption {
            value: "",
            text: t.get("connection-mode-choose"),
        })
        .chain(modes_of(form.platform).iter().map(|mode| ModeOption {
            value: mode.as_str(),
            text: t.get(mode_key(*mode)),
        }))
        .collect();
        let password_key = if form.password_set {
            "connection-password-stored"
        } else {
            "connection-password-none"
        };
        Self {
            action: format!("/machines/{}/connection", form.code),
            csrf_token,
            row_version: form.row_version,
            is_s7: form.platform.is_s7(),
            host: stored.host,
            rack: stored.rack,
            slot: stored.slot,
            port: stored.port,
            address: stored.address,
            modes,
            mode: stored.mode,
            login: stored.login,
            password_hint: t.get(password_key),
            password_reenter: false,
            blocked,
            oob: false,
            errors: FormErrors::default(),
        }
    }

    /// Return the form of a refused request: the sent texts, the sent `row_version`, and the
    /// error lines, swapped out of band (ADR-0041 (e)). The password input stays empty. The flag
    /// `password_reenter` is true when the request held a password, so the form shows the hint
    /// that asks to type it again (design D8). The form never holds the password itself.
    fn sent(
        self,
        texts: InputTexts,
        row_version: i64,
        password_reenter: bool,
        errors: FormErrors,
    ) -> Self {
        Self {
            host: texts.host,
            rack: texts.rack,
            slot: texts.slot,
            port: texts.port,
            address: texts.address,
            mode: texts.mode,
            login: texts.login,
            row_version,
            password_reenter,
            oob: true,
            errors,
            ..self
        }
    }

    /// Build the form of a saved request from the output of the command: the saved record, the new
    /// `row_version`, and no error line, swapped out of band (ADR-0041 (h)). The password input
    /// stays empty.
    fn saved(t: Texts<'_>, output: &SetPlcConnectionOutput, csrf_token: String) -> Self {
        Self {
            oob: true,
            ..Self::new(t, &output.form, csrf_token, false)
        }
    }
}

/// The inputs of the S7 shape of the form, by their `id`.
const S7_INPUTS: [&str; 3] = ["host", "rack", "slot"];
/// The inputs of the IEC shape of the form, by their `id`.
const IEC_INPUTS: [&str; 6] = ["host", "port", "address", "comm_mode", "login", "password"];

/// Return true when the form of the platform shape has an input with the `id` `field`.
///
/// A line of an input that the form lacks has no target, so the summary shows it without a link.
pub(super) fn shows_input(is_s7: bool, field: &str) -> bool {
    let inputs: &[&str] = if is_s7 { &S7_INPUTS } else { &IEC_INPUTS };
    inputs.contains(&field)
}

/// The page `machine-connection.html`.
#[derive(Template)]
#[template(path = "machine-connection.html")]
struct ConnectionPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    machine_href: String,
    machine_link: String,
    form: ConnectionFormView,
}

/// The block `form` of `machine-connection.html` alone: the form that an error answer swaps out of
/// band.
#[derive(Template)]
#[template(path = "machine-connection.html", block = "form")]
struct ConnectionFormBlock<'a> {
    t: Texts<'a>,
    form: ConnectionFormView,
}

/// Run `GetPlcConnectionForm`. The query checks `MachineManage` first (ADR-0046 (a)).
pub(super) async fn connection_form(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
) -> Result<PlcConnectionForm, WebError> {
    Ok(GetPlcConnectionForm::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            PlcConnectionFormInput { code },
        )
        .await?)
}

/// Build the error of a refused save: every line and the form with the sent texts (ADR-0041 (e)).
///
/// `lines` builds the lines from the shape of the platform, because a line of an input that the
/// form lacks has no link. The error of `GetPlcConnectionForm` wins, for example `404 E-WEB-005`.
/// The sent `row_version` stays in the form, so the next save compares the token that the page
/// showed. The flag `password_reenter` tells that the request held a password, which the form
/// does not keep (design D8).
pub(super) async fn refuse(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    sent: (InputTexts, i64, bool),
    lines: impl FnOnce(bool) -> NonEmpty<FieldError>,
) -> WebError {
    let form = match connection_form(state, request, code).await {
        Ok(form) => form,
        Err(error) => return error,
    };
    let (texts, row_version, password_reenter) = sent;
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines(form.platform.is_s7()), |errors| {
        ConnectionFormBlock {
            t,
            form: ConnectionFormView::new(t, &form, request.csrf_token(), false).sent(
                texts,
                row_version,
                password_reenter,
                errors,
            ),
        }
    })
}

/// Return the form of a saved request as markup that swaps out of band (ADR-0041 (h)).
///
/// The form comes from the output of `SetPlcConnection`, so it needs no second read after the
/// commit and it shows the state that the command saved. It holds the saved record, the new
/// `row_version`, an empty password input with the current password state, and no error line. So a
/// line of an earlier refusal does not stay, and a typed password does not stay in the input.
pub(super) fn saved_form(
    state: &WebState,
    request: &PageRequest,
    output: &SetPlcConnectionOutput,
) -> Result<String, WebError> {
    let t = Texts::new(state.bundles(), request.lang());
    let block = ConnectionFormBlock {
        t,
        form: ConnectionFormView::saved(t, output, request.csrf_token()),
    };
    block.render().map_err(|_failed| WebError::Internal {
        request_id: request.request_id(),
    })
}

/// `GET /machines/{code}/connection/edit`: show the connection form of the live machine.
///
/// The query checks `MachineManage` first, so a role without the right gets `403 E-WEB-002` and
/// no form (ADR-0046 (a)).
pub(crate) async fn edit_form(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let form = connection_form(&state, &request, code).await?;
    let t = Texts::new(state.bundles(), request.lang());
    let title = t.with_arg("connection-title", "code", code);
    let layout = Layout::for_session(t, &request, title, Some(NavTarget::Machines))?;
    let blocked = !request.may_run(Operation::SetPlcConnection)?;
    let page = ConnectionPage {
        machine_href: format!("/machines/{code}"),
        machine_link: t.with_arg("connection-machine-link", "code", code),
        form: ConnectionFormView::new(t, &form, layout.csrf_token.clone(), blocked),
        layout,
        t,
    };
    Ok(render(&page, request.request_id())?.into_response())
}
