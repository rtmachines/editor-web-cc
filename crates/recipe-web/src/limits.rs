//! The body limit of each route (design D11 step 5, `api.md` section 7.6).
//!
//! The module solves one problem: an edit form of a parameter can be 16 MiB, a translation 64 KiB,
//! and every other form 256 KiB. One limit for all routes would either refuse a large value or
//! let a small form carry 16 MiB. The layer and the buffer of the CSRF check pick the limit of
//! the matched route, so both refuse the same size with `413 E-WEB-003`.

use std::sync::Arc;

use axum::body::HttpBody as _;
use axum::extract::{MatchedPath, Request, State};
use axum::http::Method;
use axum::http::header::CONTENT_LENGTH;
use axum::middleware::Next;
use axum::response::Response;
use recipe_app::error::{AppError, TimeoutStage};

use crate::error::{ByteLimit, WebError};
use crate::settings::WebSettings;
use crate::state::WebState;

/// The path patterns of the four routes that carry a parameter value (`api.md` section 7.6).
const VALUE_ROUTES: [&str; 4] = [
    "/recipes/{ref}/versions/{n}/parameters",
    "/recipes/{ref}/versions/{n}/parameters/{key}/definition",
    "/recipes/{ref}/versions/{n}/parameters/{key}/value",
    "/recipes/{ref}/versions/{n}/rebind",
];
/// The path pattern of the route that writes a translation.
const TRANSLATION_ROUTE: &str = "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}";

/// Return the size that the request announces: the larger of `Content-Length` and the body hint.
fn announced_size(request: &Request) -> u64 {
    let declared = request
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|text| text.parse::<u64>().ok())
        .unwrap_or(0);
    let hinted = request.body().size_hint().upper().unwrap_or(0);
    declared.max(hinted)
}

/// Return the body limit of the route with the path pattern `pattern`.
///
/// Only a `POST` carries a body. The four value routes get `http.body_limit.value` and the
/// translation route gets `http.body_limit.translation`. Every other route, and a request that
/// matched no route, gets `http.body_limit.form`.
pub(crate) fn route_limit(settings: &WebSettings, method: &Method, pattern: &str) -> usize {
    if *method != Method::POST {
        settings.body_limit_form
    } else if VALUE_ROUTES.contains(&pattern) {
        settings.body_limit_value
    } else if pattern == TRANSLATION_ROUTE {
        settings.body_limit_translation
    } else {
        settings.body_limit_form
    }
}

/// Return the error of a wait for a value body permit: `E-WEB-009`.
///
/// A closed semaphore never happens, because nothing closes it. It gives the same answer.
const fn timed_out() -> WebError {
    WebError::App(AppError::Timeout {
        stage: TimeoutStage::ValueBodyPermit,
    })
}

/// Tell whether a request takes a value body permit: a `POST` on one of the four value routes.
pub(crate) fn needs_value_body_permit(method: &Method, pattern: &str) -> bool {
    *method == Method::POST && VALUE_ROUTES.contains(&pattern)
}

/// Take a value body permit for a `POST` on a value route and hold it until the answer.
///
/// The function solves one problem: each request on a value route buffers a body of up to 16 MiB,
/// so parallel requests need a bound (ADR-0211 (m) to (o)). The layer runs inside the request
/// timeout and outside the body deadline. A waiting request keeps its full body deadline. The wait
/// lasts at most `http.body_read_timeout`. Then the layer returns `E-WEB-009` and reads no body.
/// The permit drops when the handler returns its answer, also an error answer or a panic.
pub(crate) async fn value_body_permit(
    State(state): State<WebState>,
    request: Request,
    next: Next,
) -> Result<Response, WebError> {
    let pattern = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("", MatchedPath::as_str);
    if !needs_value_body_permit(request.method(), pattern) {
        return Ok(next.run(request).await);
    }
    let waiting = Arc::clone(state.value_body_permits()).acquire_owned();
    let permit = tokio::time::timeout(state.settings().body_read_timeout, waiting)
        .await
        .map_err(|_elapsed| timed_out())?
        .map_err(|_closed| timed_out())?;
    let response = next.run(request).await;
    drop(permit);
    Ok(response)
}

/// Return the largest body limit of all routes. The router uses it as the limit of the extractors.
pub(crate) fn largest_limit(settings: &WebSettings) -> usize {
    settings
        .body_limit_form
        .max(settings.body_limit_value)
        .max(settings.body_limit_translation)
}

/// Return the body limit of the route that matched `request`.
pub(crate) fn request_limit(settings: &WebSettings, request: &Request) -> usize {
    let pattern = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("", MatchedPath::as_str);
    route_limit(settings, request.method(), pattern)
}

/// Reject a request whose announced body is above the limit of its route with `E-WEB-003`.
///
/// The layer reads no body. A body that announces nothing, such as a chunked body, is checked again
/// in the buffer of the CSRF check (`to_bytes` with the same limit).
pub(crate) async fn body_limit(
    State(state): State<WebState>,
    request: Request,
    next: Next,
) -> Result<Response, WebError> {
    let limit = request_limit(state.settings(), &request);
    let allowed = u64::try_from(limit).unwrap_or(u64::MAX);
    if announced_size(&request) > allowed {
        return Err(WebError::PayloadTooLarge {
            limit: ByteLimit::from(limit),
        });
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
pub(crate) mod route_tests;
