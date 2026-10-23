//! The JSON answer of an error on an export path (ADR-0053, `api.md` section 5.3).

use axum::http::HeaderValue;
use axum::http::header::{CONTENT_TYPE, RETRY_AFTER};
use axum::response::{IntoResponse, Response};
use recipe_export::error_body;

use super::WebError;
use super::answer::{ErrorContext, args_of, logged, retry_after_of, sentence_of, session_redirect};

/// The media type of the JSON error body (ADR-0053 (a)).
const JSON: &str = "application/json";

/// Map an error on an export path to its answer and write its one log line.
///
/// The body is the JSON object of ADR-0053 (a) that `recipe-export` writes. The status is the
/// status of the HTML answer. `message` is the sentence of the code in the UI language without
/// the reference. `E-WEB-004` and `E-IDN-007` keep their `303` redirects, also for an HTMX request
/// (ADR-0053 (d)). The answer has no `HX-*` header. `Retry-After` stays on `503` and `429`.
pub(crate) fn json_answer(error: &WebError, ctx: &ErrorContext<'_>) -> Response {
    let (shown, status) = logged(error, ctx);
    if let Some(response) = session_redirect(shown, false) {
        return response;
    }
    let message = sentence_of(error, shown, &args_of(error, ctx), ctx);
    let body = error_body(shown, &message, ctx.request_id);
    let mut response = (
        status,
        [(CONTENT_TYPE, HeaderValue::from_static(JSON))],
        body,
    )
        .into_response();
    if let Some(seconds) = retry_after_of(error, status, ctx) {
        response
            .headers_mut()
            .insert(RETRY_AFTER, HeaderValue::from(seconds));
    }
    response
}
