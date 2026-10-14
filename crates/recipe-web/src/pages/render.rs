//! Helpers that every page handler shares.

use askama::Template;
use axum::http::header::{LOCATION, SET_COOKIE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use recipe_domain::audit::RequestId;

use crate::error::WebError;

/// Render a template. A failure is a defect of the server and gives `E-WEB-010`.
///
/// # Errors
///
/// Returns `WebError::Internal` when the template cannot render.
pub(crate) fn render<T: Template>(
    page: &T,
    request_id: RequestId,
) -> Result<Html<String>, WebError> {
    page.render()
        .map(Html)
        .map_err(|_failed| WebError::Internal { request_id })
}

/// Build a `303` answer with the `Location` header and the given `Set-Cookie` values.
pub(crate) fn see_other(
    location: &'static str,
    cookies: impl IntoIterator<Item = HeaderValue>,
) -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    let headers = response.headers_mut();
    headers.insert(LOCATION, HeaderValue::from_static(location));
    for cookie in cookies {
        headers.append(SET_COOKIE, cookie);
    }
    response
}
