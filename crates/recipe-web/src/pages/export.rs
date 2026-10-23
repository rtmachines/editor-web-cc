//! The two export routes of `api.md` section 2.7 (design D7 (a) and (c), ADR-0054).
//!
//! Each handler parses the path values, builds its use case with the adapter [`ExportWriter`],
//! and sends the document as a file. The use case checks the permission, the state, and the hash,
//! and it writes the audit entry. Every error goes to the error layer, which answers JSON on these
//! paths (ADR-0053).

use std::sync::Arc;

use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use recipe_app::auth::Operation;
use recipe_app::export::{
    ExportApprovedInput, ExportApprovedVersion, ExportByNumberInput, ExportVersionByNumber,
    ExportedVersion,
};
use recipe_app::ports::DocumentWriter;
use recipe_domain::audit::RequestId;
use recipe_domain::export::{DocumentFacts, ExportError, HashedBody};

use super::recipes::path_reference;
use super::request::PageRequest;
use super::versions::path_version;
use crate::error::WebError;
use crate::routes::{RouteEntry, operation_row};
use crate::state::WebState;

/// Return the route rows of the two export routes.
///
/// Both rows accept only `GET`. The access table gives a `HEAD` request on these paths the answer
/// of an unlisted method (ADR-0054).
pub(crate) fn rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/export/v1/recipes/{ref}/approved",
            Operation::ExportApprovedVersion,
            get(approved),
        ),
        operation_row(
            Method::GET,
            "/export/v1/recipes/{ref}/versions/{n}",
            Operation::ExportVersionByNumber,
            get(by_number),
        ),
    ]
}

/// Writes the export document with `recipe_export::write_document`.
///
/// The adapter solves one problem: the use cases of `recipe-app` need the port `DocumentWriter`,
/// but only `recipe-web` may depend on `recipe-export` (`overview.md` section 5). The adapter adds
/// no logic of its own.
struct ExportWriter;

impl DocumentWriter for ExportWriter {
    fn write(&self, body: &HashedBody, facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        recipe_export::write_document(body, facts)
    }
}

/// Return the writer that the export use cases get for one request.
fn writer() -> Arc<dyn DocumentWriter> {
    Arc::new(ExportWriter)
}

/// `GET /export/v1/recipes/{ref}/approved`: send the approved version of the recipe as a file.
async fn approved(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let reference = path_reference(path)?;
    let input = ExportApprovedInput { reference };
    let exported = ExportApprovedVersion::new(state.identity().app().clone(), writer())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    file_answer(exported, request.request_id())
}

/// `GET /export/v1/recipes/{ref}/versions/{n}`: send an approved or a retired version as a file.
async fn by_number(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let (reference, number) = path_version(path)?;
    let input = ExportByNumberInput { reference, number };
    let exported = ExportVersionByNumber::new(state.identity().app().clone(), writer())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    file_answer(exported, request.request_id())
}

/// Return the value of `Content-Disposition` that names `file_name` as an attachment.
///
/// # Errors
///
/// Returns `WebError::Internal` (`E-WEB-010`) when the value is not a valid header value. The
/// grammar of `ReferenceNumber` makes this branch unreachable for a real file name.
fn content_disposition(file_name: &str, request_id: RequestId) -> Result<HeaderValue, WebError> {
    HeaderValue::from_str(&format!("attachment; filename=\"{file_name}\""))
        .map_err(|_invalid| WebError::Internal { request_id })
}

/// Answer `200` with the document of `exported` and the headers of `api.md` section 6.3.
///
/// The layer of `api.md` section 9 adds `X-Content-Type-Options` and the CSP.
fn file_answer(exported: ExportedVersion, request_id: RequestId) -> Result<Response, WebError> {
    let name = recipe_export::file_name(&exported.reference, exported.number, exported.status);
    let headers = [
        (CONTENT_TYPE, HeaderValue::from_static("application/json")),
        (CONTENT_DISPOSITION, content_disposition(&name, request_id)?),
        (CACHE_CONTROL, HeaderValue::from_static("no-store")),
    ];
    Ok((StatusCode::OK, headers, exported.document).into_response())
}

#[cfg(test)]
mod access_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod tests;
