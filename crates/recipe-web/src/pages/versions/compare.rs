//! `GET /recipes/{ref}/compare`: the redirect of the compare form to the diff page (`api.md`
//! section 2.5, ADR-0108 (e), design D3).
//!
//! The route solves one problem: the compare form of the version page has two lists, `n` and `m`,
//! and the user may pick them in any order. The diff page needs the higher number first
//! (ADR-0050 (l)). The route reads no data and runs no use case. The diff route checks the
//! permission and the versions one request later.

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::response::Response;
use recipe_app::error::Missing;
use serde::Deserialize;

use crate::error::WebError;
use crate::pages::path_values::{query_values, reference, version_query};
use crate::pages::render::see_other_path;
use crate::pages::request::PageRequest;

/// The query string of the compare route. A key other than `n` and `m` gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompareQuery {
    n: Option<String>,
    m: Option<String>,
}

/// `GET /recipes/{ref}/compare`: answer `303` to the diff page of the two versions.
///
/// The higher number is the new side and the lower number is the old side.
///
/// # Errors
///
/// Returns `404 E-WEB-005` for a bad `{ref}`. Returns `422 E-WEB-008` for a missing key, an
/// unknown key, or a value that is not a version number.
pub(crate) async fn compare(
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<CompareQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let Path(text) = path.map_err(|_rejection| {
        WebError::App(recipe_app::error::AppError::NotFound {
            entity: Missing::MalformedRecipe,
        })
    })?;
    let reference = reference(&text)?;
    let query = query_values(query)?;
    let n = version_query("n", query.n.as_deref())?;
    let m = version_query("m", query.m.as_deref())?;
    let (new, old) = (n.max(m), n.min(m));
    let location = format!("/recipes/{reference}/versions/{new}/diff/{old}");
    see_other_path(&location, request.request_id(), [])
}
