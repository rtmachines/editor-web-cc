//! The three machine routes of the transfer client (`api.md` section 2.7, ADR-0109 (g), (h), (j),
//! (w) to (ab)).
//!
//! All rows use HTTP Basic of an `ExportClient` account (`Access::Basic`). The layer
//! `basic_layer` has already checked the header when the handler runs, so the handler takes the
//! account from the extractor `BasicUser`. The use case checks the permission, the machine, and
//! the hash, and it writes the audit entry of the document. Every error goes to the error layer,
//! which answers JSON on these paths (ADR-0053).

use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use recipe_app::auth::Operation;
use recipe_app::error::{AppError, Missing};
use recipe_app::export::{
    ExportApprovedInput, ExportApprovedVersion, ExportScope, ExportableRecipesInput,
    ListExportableRecipes,
};
use recipe_domain::plant::MachineCode;
use recipe_domain::recipe::ReferenceNumber;

use super::export::{file_answer, named_file_answer, writer};
use super::path_values::{machine_code, path_code, path_code_reference_unit, reference};
use crate::basic_auth::BasicUser;
use crate::error::WebError;
use crate::routes::{RouteEntry, basic_row};
use crate::state::WebState;

/// Return the route rows of the three machine routes.
///
/// Both rows accept only `GET`. The access table gives a `HEAD` request on these paths the answer
/// of an unlisted method (ADR-0054).
pub(crate) fn rows() -> Vec<RouteEntry> {
    vec![
        basic_row(
            Method::GET,
            "/export/v1/machines/{code}/recipes",
            Operation::ListExportableRecipes,
            get(list),
        ),
        basic_row(
            Method::GET,
            "/export/v1/machines/{code}/recipes/{ref}/approved",
            Operation::ExportApprovedVersion,
            get(approved),
        ),
        basic_row(
            Method::GET,
            "/export/v2/machines/{code}/recipes/{ref}/approved/units/{path}",
            Operation::ExportApprovedVersion,
            get(approved_unit),
        ),
    ]
}

/// Read the route `/export/v1/machines/{code}/recipes/{ref}/approved`. The code comes first.
///
/// # Errors
///
/// Returns `404 E-WEB-005` for a bad code, a bad reference number, or a rejection.
fn path_code_reference(
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<(MachineCode, ReferenceNumber), WebError> {
    let Path((code, text)) = path.map_err(|_rejection| {
        WebError::App(AppError::NotFound {
            entity: Missing::MalformedMachine,
        })
    })?;
    Ok((machine_code(&code)?, reference(&text)?))
}

/// `GET /export/v1/machines/{code}/recipes`: send the recipes of the machine that have an
/// approved version.
async fn list(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    account: BasicUser,
) -> Result<Response, WebError> {
    let machine = path_code(path)?;
    let recipes = ListExportableRecipes::new(state.identity().app().clone())
        .execute(
            account.user(),
            account.meta(),
            ExportableRecipesInput { machine },
        )
        .await?;
    let body = recipe_export::write_list(&recipes).map_err(AppError::from)?;
    let headers = [
        (CONTENT_TYPE, HeaderValue::from_static("application/json")),
        (CACHE_CONTROL, HeaderValue::from_static("no-store")),
    ];
    Ok((StatusCode::OK, headers, body).into_response())
}

/// `GET /export/v1/machines/{code}/recipes/{ref}/approved`: send the approved version of a recipe
/// of the machine as a file.
async fn approved(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    account: BasicUser,
) -> Result<Response, WebError> {
    let (machine, reference) = path_code_reference(path)?;
    let input = ExportApprovedInput {
        reference,
        machine: Some(machine),
        scope: ExportScope::Version,
    };
    let exported = ExportApprovedVersion::new(state.identity().app().clone(), writer())
        .execute(account.user(), account.meta(), input)
        .await?;
    file_answer(exported, account.meta().request_id())
}

/// `GET /export/v2/machines/{code}/recipes/{ref}/approved/units/{path}`: send the v2 document of
/// one unit and its descendant units as a file.
///
/// The use case checks the whole version before it filters the scope. It answers `E-EXP-005` for a
/// scope without a parameter. The file name holds the unit path of the request.
async fn approved_unit(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    account: BasicUser,
) -> Result<Response, WebError> {
    let (machine, reference, unit) = path_code_reference_unit(path)?;
    let input = ExportApprovedInput {
        reference,
        machine: Some(machine),
        scope: ExportScope::Unit(unit.clone()),
    };
    let exported = ExportApprovedVersion::new(state.identity().app().clone(), writer())
        .execute(account.user(), account.meta(), input)
        .await?;
    let name = recipe_export::unit_file_name(&exported.reference, exported.number, &unit);
    named_file_answer(exported, &name, account.meta().request_id())
}

#[cfg(test)]
mod read_only_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod unit_tests;
