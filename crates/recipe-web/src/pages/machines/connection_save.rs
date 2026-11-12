//! `POST /machines/{code}/connection`: save the PLC connection of a machine (`api.md` section 2.2,
//! ADR-0154, design D6 and D8).
//!
//! A valid HTMX request gets `200` with the toast, the form out of band with the saved record
//! (ADR-0041 (h)), and `HX-Trigger: {"rowVersion": <new>}`. A
//! request without HTMX gets `303` to the machine page. The route checks the permission
//! `MachineManage` before it reads a field, so a role without the right gets `403 E-WEB-002`
//! (ADR-0046 (a)). The use case checks it again as its first statement (ADR-0013).
//!
//! A refused save answers one `422` with a line under each bad input and the form with the sent
//! texts out of band (ADR-0041). The lines come from the texts that do not parse and from the
//! faults of the domain (`E-PLT-022` to `E-PLT-028`). A changed form body gives one `E-WEB-008`
//! line and no form (ADR-0138). The password input never has a value in any form.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{Html, IntoResponse, Response};
use recipe_app::auth::{Operation, authorize};
use recipe_app::error::AppError;
use recipe_app::plant::{SetPlcConnection, SetPlcConnectionInput};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::{MachineCode, PlantError, PlantErrors};

use super::connection::{refuse, saved_form, shows_input};
use super::connection_request::ConnectionRequest;
use crate::error::{FieldError, WebError};
use crate::pages::draft_reply::with_row_version;
use crate::pages::path_values::path_code;
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The answer `machine-connection-saved.html`: the toast and the empty error panel, out of band.
/// The save adds the out-of-band form after it.
#[derive(Template)]
#[template(path = "machine-connection-saved.html")]
struct ConnectionSaved {
    message: String,
}

/// Check the permission of the route before the first field is read (ADR-0046 (a)).
async fn check_permission(state: &WebState, request: &PageRequest) -> Result<(), WebError> {
    authorize(
        Operation::SetPlcConnection,
        request.auth()?.user(),
        &request.meta(),
        state.security(),
    )
    .await?;
    Ok(())
}

/// Return the `id` of the input that a fault of the domain names, or `None` for no input.
///
/// The domain names a field with the `id` of its input. A shape without that input gives `None`,
/// so the line stands in the summary without a link.
fn target_of(error: &PlantError, is_s7: bool) -> Option<String> {
    let field = match error {
        PlantError::ConnectionFieldNotOnPlatform { field, .. }
        | PlantError::ConnectionFieldMissing { field, .. }
        | PlantError::ConnectionFieldNotInMode { field, .. } => *field,
        PlantError::CommModeNotOnPlatform { .. } => "comm_mode",
        PlantError::AddressNotAmsNetId { .. } => "address",
        _other => return None,
    };
    shows_input(is_s7, field).then(|| field.to_owned())
}

/// Return one line for each fault of the domain, in the order of the list (ADR-0041).
fn plant_lines(errors: &PlantErrors, is_s7: bool) -> NonEmpty<FieldError> {
    let line = |error: &PlantError| FieldError::plant(error, target_of(error, is_s7));
    let mut lines = NonEmpty::new(line(errors.first()));
    for error in errors.iter().skip(1) {
        lines.push(line(error));
    }
    lines
}

/// `POST /machines/{code}/connection`: save the connection record of the live machine.
pub(super) async fn save(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<ConnectionRequest>, FormRejection>,
) -> Result<Response, WebError> {
    let code: MachineCode = path_code(path)?;
    check_permission(&state, &request).await?;
    let Form(fields) = form?;
    let expected_row_version = fields.expected_row_version()?;
    let parsed = match fields.fields() {
        Ok(parsed) => parsed,
        Err(lines) => {
            let sent = (
                fields.input_texts(),
                expected_row_version,
                fields.password.is_some(),
            );
            return Err(refuse(&state, &request, code, sent, |_is_s7| lines).await);
        }
    };
    let input = SetPlcConnectionInput {
        code,
        expected_row_version,
        fields: parsed,
    };
    let cipher = state.credential_cipher().ok_or(WebError::Internal {
        request_id: request.request_id(),
    })?;
    let saved = SetPlcConnection::new(state.identity().app().clone(), cipher)
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    let output = match saved {
        Ok(output) => output,
        Err(AppError::Plants(errors)) => {
            let sent = (
                fields.input_texts(),
                expected_row_version,
                fields.password.is_some(),
            );
            let lines = |is_s7| plant_lines(&errors, is_s7);
            return Err(refuse(&state, &request, code, sent, lines).await);
        }
        Err(other) => return Err(other.into()),
    };
    if !request.is_htmx() {
        return see_other_path(&format!("/machines/{code}"), request.request_id(), []);
    }
    let t = Texts::new(state.bundles(), request.lang());
    let toast = render(
        &ConnectionSaved {
            message: t.with_arg("connection-saved", "code", code),
        },
        request.request_id(),
    )?;
    let form = saved_form(&state, &request, &output)?;
    let body = Html(format!("{}{form}", toast.0));
    with_row_version(
        body.into_response(),
        output.form.row_version,
        request.request_id(),
    )
}
