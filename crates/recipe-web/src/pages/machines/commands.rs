//! `POST /machines/{code}/rename` and `POST /machines/{code}/archive` (`api.md` section 2.2,
//! design D6, ADR-0037 x).
//!
//! A valid HTMX rename gets the new name for `#machine-name` and updates `#toast` and
//! `#error-panel` out of band. It also gets the clean form out of band (ADR-0041 (h)). A rename
//! without HTMX gets `303` to the machine page. A bad name gets `422 E-WEB-008` with the form and
//! the line under its input. A domain error of `RenameMachine` gets the form with only its line.
//! The form query `GetMachineForm` checks the permission before the field check (ADR-0013). A
//! valid archive always gets `303` to the list with the field `archived`, which shows the message
//! there.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::MachineFormPurpose;
use recipe_app::plant::{ArchiveMachine, ArchiveMachineInput, RenameMachine, RenameMachineInput};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::MachineCode;
use recipe_domain::text::Text;
use serde::Deserialize;

use super::form::machine_form;
use crate::error::{FieldError, FieldReason, FormErrors, WebError};
use crate::pages::draft_reply::{refusal_lines, refused, with_form};
use crate::pages::path_values::path_code;
use crate::pages::recipes::fields::FieldLines;
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::pages::users::TokenFields;
use crate::state::WebState;

/// The fields of the form of `POST /machines/{code}/rename`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NameFields {
    name: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// The data of the rename form. The machine page and each answer of the rename show it.
pub(super) struct RenameForm {
    /// The address of the route, for example `/machines/M7/rename`.
    pub(super) action: String,
    /// The name in the input: the current name, the saved name, or the sent text.
    pub(super) name: String,
    /// The CSRF token of the session.
    pub(super) csrf_token: String,
    /// True when the page or the answer shows the form. The machine page sets it by permission.
    pub(super) shown: bool,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    pub(super) oob: bool,
    /// The error lines under the input.
    pub(super) errors: FormErrors,
}

impl RenameForm {
    /// Build the form of the machine `code` with `name`. The form is shown and has no line.
    pub(super) fn new(code: MachineCode, name: String, csrf_token: String) -> Self {
        Self {
            action: format!("/machines/{code}/rename"),
            name,
            csrf_token,
            shown: true,
            oob: false,
            errors: FormErrors::default(),
        }
    }

    /// Return the form with `shown` set by the permission of the role.
    #[must_use]
    pub(super) fn shown(self, shown: bool) -> Self {
        Self { shown, ..self }
    }

    /// Return the form that an answer swaps out of band, with the error lines `errors`.
    fn out_of_band(self, errors: FormErrors) -> Self {
        Self {
            oob: true,
            errors,
            ..self
        }
    }
}

/// The block `rename_form` of `machine.html` alone: the form that an answer swaps out of band.
#[derive(Template)]
#[template(path = "machine.html", block = "rename_form")]
struct RenameFormBlock<'a> {
    t: Texts<'a>,
    form: RenameForm,
}

/// The answer `machine-renamed.html`: the new name and two out-of-band updates.
#[derive(Template)]
#[template(path = "machine-renamed.html")]
struct MachineRenamedFragment {
    name: String,
    message: String,
}

/// Build the error with the line of the name and the form with the sent text.
fn refused_name(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    (sent, lines): (String, NonEmpty<FieldError>),
) -> WebError {
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| RenameFormBlock {
        t,
        form: RenameForm::new(code, sent, request.csrf_token()).out_of_band(errors),
    })
}

/// Answer a valid HTMX rename: the name, the toast, the empty error panel, and the clean form.
fn renamed(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    name: &Text,
) -> Result<Response, WebError> {
    let t = Texts::new(state.bundles(), request.lang());
    let fragment = MachineRenamedFragment {
        name: name.to_string(),
        message: t.with_arg("machine-renamed", "code", code),
    };
    let form = RenameFormBlock {
        t,
        form: RenameForm::new(code, name.to_string(), request.csrf_token())
            .out_of_band(FormErrors::default()),
    };
    let body = render(&fragment, request.request_id())?;
    Ok(with_form(body, &form, request.request_id())?.into_response())
}

/// `POST /machines/{code}/rename`: rename the live machine.
pub(super) async fn rename(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<NameFields>, FormRejection>,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    // The form query checks the permission of `RenameMachine` before the field check (ADR-0046).
    machine_form(&state, &request, MachineFormPurpose::RenameMachine).await?;
    let Form(fields) = form?;
    let mut lines = FieldLines::default();
    let parsed = lines.field(Text::parse(&fields.name), "machine_name", FieldReason::Name);
    let name = match lines.finish(parsed) {
        Ok(name) => name,
        Err(lines) => return Err(refused_name(&state, &request, code, (fields.name, lines))),
    };
    let input = RenameMachineInput {
        code,
        name: name.clone(),
    };
    let result = RenameMachine::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    if let Err(error) = result {
        // No error of `RenameMachine` names the name input, so its line stands in the summary.
        return Err(match refusal_lines(error, |_refusal| None) {
            Ok(lines) => refused_name(&state, &request, code, (fields.name, lines)),
            Err(other) => other,
        });
    }
    if !request.is_htmx() {
        return see_other_path(&format!("/machines/{code}"), request.request_id(), []);
    }
    renamed(&state, &request, code, &name)
}

/// `POST /machines/{code}/archive`: archive the live machine.
pub(super) async fn archive(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<TokenFields>, FormRejection>,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let Form(_fields) = form?;
    ArchiveMachine::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            ArchiveMachineInput { code },
        )
        .await?;
    see_other_path(
        &format!("/machines?archived={code}"),
        request.request_id(),
        [],
    )
}
