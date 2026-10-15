//! `POST .../units/{path}/rename` and `POST .../units/{path}/archive` (`api.md` section 2.2,
//! design D6, ADR-0037 x).
//!
//! A valid HTMX rename gets the new name for `#unit-name` and updates `#toast` and `#error-panel`
//! out of band. A rename without HTMX gets `303` to the unit page. A valid archive always gets
//! `303` to the machine page with the field `archived`, which shows the message there.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::plant::{ArchiveUnit, ArchiveUnitInput, RenameUnit, RenameUnitInput};
use recipe_domain::text::Text;
use serde::Deserialize;

use crate::error::{FieldReason, WebError};
use crate::pages::path_values::path_code_unit;
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

/// The answer `unit-renamed.html`: the new name and two out-of-band updates.
#[derive(Template)]
#[template(path = "unit-renamed.html")]
struct UnitRenamedFragment {
    name: String,
    message: String,
}

/// `POST /machines/{code}/units/{path}/rename`: rename the unit.
pub(super) async fn rename(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<NameFields>, FormRejection>,
) -> Result<Response, WebError> {
    let (code, unit) = path_code_unit(path)?;
    let Form(fields) = form?;
    let name = Text::parse(&fields.name).map_err(|_reason| WebError::BadRequest {
        field: "unit_name",
        reason: FieldReason::Name,
    })?;
    RenameUnit::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            RenameUnitInput {
                code,
                path: unit.clone(),
                name: name.clone(),
            },
        )
        .await?;
    if !request.is_htmx() {
        let location = format!("/machines/{code}/units/{unit}");
        return see_other_path(&location, request.request_id(), []);
    }
    let t = Texts::new(state.bundles(), request.lang());
    let fragment = UnitRenamedFragment {
        name: name.to_string(),
        message: t.with_arg("unit-renamed", "path", &unit),
    };
    Ok(render(&fragment, request.request_id())?.into_response())
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
