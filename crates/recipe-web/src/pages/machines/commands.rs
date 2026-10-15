//! `POST /machines/{code}/rename` and `POST /machines/{code}/archive` (`api.md` section 2.2,
//! design D6, ADR-0037 x).
//!
//! A valid HTMX rename gets the new name for `#machine-name` and updates `#toast` and
//! `#error-panel` out of band. A rename without HTMX gets `303` to the machine page. A valid
//! archive always gets `303` to the list with the field `archived`, which shows the message there.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::plant::{ArchiveMachine, ArchiveMachineInput, RenameMachine, RenameMachineInput};
use recipe_domain::text::Text;
use serde::Deserialize;

use crate::error::{FieldReason, WebError};
use crate::pages::path_values::path_code;
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

/// The answer `machine-renamed.html`: the new name and two out-of-band updates.
#[derive(Template)]
#[template(path = "machine-renamed.html")]
struct MachineRenamedFragment {
    name: String,
    message: String,
}

/// `POST /machines/{code}/rename`: rename the live machine.
pub(super) async fn rename(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<NameFields>, FormRejection>,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let Form(fields) = form?;
    let name = Text::parse(&fields.name).map_err(|_reason| WebError::BadRequest {
        field: "machine_name",
        reason: FieldReason::Name,
    })?;
    RenameMachine::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            RenameMachineInput {
                code,
                name: name.clone(),
            },
        )
        .await?;
    if !request.is_htmx() {
        return see_other_path(&format!("/machines/{code}"), request.request_id(), []);
    }
    let t = Texts::new(state.bundles(), request.lang());
    let fragment = MachineRenamedFragment {
        name: name.to_string(),
        message: t.with_arg("machine-renamed", "code", code),
    };
    Ok(render(&fragment, request.request_id())?.into_response())
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
