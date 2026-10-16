//! The body limit of each route (design D11 step 5, `api.md` section 7.6).
//!
//! The module solves one problem: a parameter value can be 4 MiB, a translation 64 KiB, and every
//! other form 256 KiB. One limit for all routes would either refuse a large value or let a small
//! form carry 4 MiB. The layer and the buffer of the CSRF check pick the limit of the matched
//! route, so both refuse the same size with `413 E-WEB-003`.

use axum::body::HttpBody as _;
use axum::extract::{MatchedPath, Request, State};
use axum::http::Method;
use axum::http::header::CONTENT_LENGTH;
use axum::middleware::Next;
use axum::response::Response;

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
