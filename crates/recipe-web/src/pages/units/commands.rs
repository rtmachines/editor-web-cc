//! `POST .../units/{path}/rename` and `POST .../units/{path}/archive` (`api.md` section 2.2,
//! design D6, ADR-0037 x).
//!
//! A valid HTMX rename gets the new name for `#unit-name` and updates `#toast` and `#error-panel`
//! out of band. It also gets the clean form out of band (ADR-0041 (h)). A rename without HTMX gets
//! `303` to the unit page. A bad name gets `422 E-WEB-008` with the form and the line under its
//! input. A domain error of `RenameUnit` gets the form with only its line. The form query
//! `GetUnitForm` checks the permission before the field check (ADR-0013). A valid archive always
//! gets `303` to the machine page with the field `archived`.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::UnitFormPurpose;
use recipe_app::plant::{ArchiveUnit, ArchiveUnitInput, RenameUnit, RenameUnitInput};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::{MachineCode, UnitPath};
use recipe_domain::text::Text;
use serde::Deserialize;

use super::add::unit_form;
use crate::error::{FieldError, FieldReason, FormErrors, WebError};
use crate::pages::draft_reply::{refusal_lines, refused, with_form};
use crate::pages::path_values::path_code_unit;
use crate::pages::recipes::fields::FieldLines;
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::pages::users::TokenFields;
use crate::state::WebState;

/// The fields of the form of `POST .../units/{path}/rename`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NameFields {
    name: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// The data of the rename form. The unit page and each answer of the rename show it.
pub(super) struct RenameForm {
    /// The address of the route, for example `/machines/M7/units/U1.2/rename`.
    pub(super) action: String,
    /// The name in the input: the current name, the saved name, or the sent text.
    pub(super) name: String,
    /// The CSRF token of the session.
    pub(super) csrf_token: String,
    /// True when the page or the answer shows the form. The unit page sets it by permission.
    pub(super) shown: bool,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    pub(super) oob: bool,
    /// The error lines under the input.
    pub(super) errors: FormErrors,
}

impl RenameForm {
    /// Build the form of the unit `unit` of the machine `code` with `name`, shown and clean.
    pub(super) fn new(
        (code, unit): (MachineCode, &UnitPath),
        name: String,
        csrf_token: String,
    ) -> Self {
        Self {
            action: format!("/machines/{code}/units/{unit}/rename"),
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

/// The block `rename_form` of `unit.html` alone: the form that an answer swaps out of band.
#[derive(Template)]
#[template(path = "unit.html", block = "rename_form")]
struct RenameFormBlock<'a> {
    t: Texts<'a>,
    form: RenameForm,
}

/// The answer `unit-renamed.html`: the new name and two out-of-band updates.
#[derive(Template)]
#[template(path = "unit-renamed.html")]
struct UnitRenamedFragment {
    name: String,
    message: String,
}

/// Build the error with the line of the name and the form with the sent text.
fn refused_name(
    state: &WebState,
    request: &PageRequest,
    target: (MachineCode, &UnitPath),
    (sent, lines): (String, NonEmpty<FieldError>),
) -> WebError {
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| RenameFormBlock {
        t,
        form: RenameForm::new(target, sent, request.csrf_token()).out_of_band(errors),
    })
}

/// Answer a valid HTMX rename: the name, the toast, the empty error panel, and the clean form.
fn renamed(
    state: &WebState,
    request: &PageRequest,
    (code, unit): (MachineCode, &UnitPath),
    name: &Text,
) -> Result<Response, WebError> {
    let t = Texts::new(state.bundles(), request.lang());
    let fragment = UnitRenamedFragment {
        name: name.to_string(),
        message: t.with_arg("unit-renamed", "path", unit),
    };
    let form = RenameFormBlock {
        t,
        form: RenameForm::new((code, unit), name.to_string(), request.csrf_token())
            .out_of_band(FormErrors::default()),
    };
    let body = render(&fragment, request.request_id())?;
    Ok(with_form(body, &form, request.request_id())?.into_response())
}

/// `POST /machines/{code}/units/{path}/rename`: rename the unit.
pub(super) async fn rename(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<NameFields>, FormRejection>,
) -> Result<Response, WebError> {
    let (code, unit) = path_code_unit(path)?;
    // The form query checks the permission of `RenameUnit` before the field check (ADR-0046).
    unit_form(&state, &request, (code, UnitFormPurpose::RenameUnit)).await?;
    let Form(fields) = form?;
    let mut lines = FieldLines::default();
    let parsed = lines.field(Text::parse(&fields.name), "unit_name", FieldReason::Name);
    let target = (code, &unit);
    let name = match lines.finish(parsed) {
        Ok(name) => name,
        Err(lines) => return Err(refused_name(&state, &request, target, (fields.name, lines))),
    };
    let input = RenameUnitInput {
        code,
        path: unit.clone(),
        name: name.clone(),
    };
    let result = RenameUnit::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    if let Err(error) = result {
        // No error of `RenameUnit` names the name input, so its line stands in the summary.
        return Err(match refusal_lines(error, |_refusal| None) {
            Ok(lines) => refused_name(&state, &request, target, (fields.name, lines)),
            Err(other) => other,
        });
    }
    if !request.is_htmx() {
        let location = format!("/machines/{code}/units/{unit}");
        return see_other_path(&location, request.request_id(), []);
    }
    renamed(&state, &request, (code, &unit), &name)
}

/// `POST /machines/{code}/units/{path}/archive`: archive the unit and its live sub-units.
pub(super) async fn archive(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<TokenFields>, FormRejection>,
) -> Result<Response, WebError> {
    let (code, unit) = path_code_unit(path)?;
    let Form(_fields) = form?;
    ArchiveUnit::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            ArchiveUnitInput {
                code,
                path: unit.clone(),
            },
        )
        .await?;
    let location = format!("/machines/{code}?archived={unit}");
    see_other_path(&location, request.request_id(), [])
}
