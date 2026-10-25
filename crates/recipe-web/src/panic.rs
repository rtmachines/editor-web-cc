//! The two panic layers of the router (design D5, ADR-0057).
//!
//! A panic in application code is a bug. These layers contain it, so the process keeps serving
//! (`reliability.md` section 2.4 rule 3). Neither handler reads, logs, or shows the panic payload.

use std::any::Any;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use recipe_domain::error_codes::ErrorCode;
use tower_http::catch_panic::CatchPanicLayer;

/// The handler type of both panic layers. It gets only the panic payload.
pub(crate) type PanicHandler = fn(Box<dyn Any + Send + 'static>) -> Response;

/// The English text of the row `E-WEB-010` of `reliability.md` section 1.2 without the reference.
const INTERNAL_TEXT: &str = "An internal error occurred. Contact the administrator.";

/// The marker that the inner panic layer puts into the response extensions.
///
/// The type solves one problem: the handler of the inner layer gets only the panic payload. It
/// does not know the request id, the language, the HTMX header, or the path. `render_errors` knows
/// them, so it turns the marker into `WebError::Internal` (ADR-0057 (a)).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PanicCaught;

/// Return the inner panic layer. It sits directly inside `render_errors` (ADR-0057 (a)).
pub(crate) fn inner_layer() -> CatchPanicLayer<PanicHandler> {
    let handler: PanicHandler = mark_panic;
    CatchPanicLayer::custom(handler)
}

/// Return the backstop panic layer. It sits directly inside the layer `request_id` (ADR-0057 (c)).
pub(crate) fn backstop_layer() -> CatchPanicLayer<PanicHandler> {
    let handler: PanicHandler = answer_backstop;
    CatchPanicLayer::custom(handler)
}

/// Answer a panic inside `render_errors` with a `500` and the marker. The payload is not read.
///
/// The handler writes no line, because the mapping of `render_errors` writes the line of
/// `E-WEB-010` (`reliability.md` section 1.1).
fn mark_panic(_payload: Box<dyn Any + Send + 'static>) -> Response {
    let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
    response.extensions_mut().insert(PanicCaught);
    response
}

/// Answer a panic of an outer layer with the plain text of `E-WEB-010` (ADR-0057 (d), (g)).
///
/// `render_errors` does not run for this answer, so the handler writes the `ERROR` line itself.
/// The line holds no request id, because the handler gets only the payload.
fn answer_backstop(_payload: Box<dyn Any + Send + 'static>) -> Response {
    let code = ErrorCode::WEB_010;
    tracing::error!(error_code = %code, chain = INTERNAL_TEXT, "request failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("{code} {INTERNAL_TEXT}"),
    )
        .into_response()
}

#[cfg(test)]
mod tests;
