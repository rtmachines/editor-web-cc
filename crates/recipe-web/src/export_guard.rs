//! The navigation guard of the export routes (ADR-0055, design D7 (f)).
//!
//! The guard solves one problem: the CSRF layer skips `GET`, but an export writes an audit entry.
//! A same-site page could start an export in the name of the user and write a false entry. A
//! boosted link could swap the document into the page as HTML.
//!
//! `routes::assemble` puts the guard on each route row under the export prefix. The layers of the
//! router wrap it, so it runs after the session layer and the gates, and before the handler.

use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;

use crate::csrf::{SAME_ORIGIN, SEC_FETCH_SITE, reject, rejection_event};
use crate::error::WebError;
use crate::pipeline::HX_REQUEST;
use crate::state::WebState;

/// The value of `Sec-Fetch-Site` for a request that the user started, for example a bookmark.
const USER_STARTED: &str = "none";

/// Tell whether the headers let an export run (ADR-0055 (b), (c)).
///
/// A missing `Sec-Fetch-Site` passes. Its value must equal `same-origin` or `none` byte for byte.
/// The header `HX-Request: true` always fails.
fn passes(headers: &HeaderMap) -> bool {
    let site_passes = headers.get(SEC_FETCH_SITE).is_none_or(|value| {
        let site = value.as_bytes();
        site == SAME_ORIGIN.as_bytes() || site == USER_STARTED.as_bytes()
    });
    let htmx = headers.get(HX_REQUEST).is_some_and(|value| value == "true");
    site_passes && !htmx
}

/// Reject an export that a foreign page or an HTMX request starts (ADR-0055 (d)).
///
/// A rejected request gets `403 E-WEB-007` and writes one `csrf_rejected` event. The handler
/// does not run, so the audit log gets no `VersionExported` entry. A failed event write gives the
/// storage error.
pub(crate) async fn export_guard(
    State(state): State<WebState>,
    request: Request,
    next: Next,
) -> Result<Response, WebError> {
    if passes(request.headers()) {
        return Ok(next.run(request).await);
    }
    Err(reject(&state, rejection_event(&request)).await)
}

#[cfg(test)]
mod tests;
