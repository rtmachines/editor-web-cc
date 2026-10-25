//! The request timeout and the body deadline of the router (design D5, ADR-0058 (g) to (j),
//! ADR-0072).
//!
//! A request without a bound holds a connection and a pool slot without limit. The request
//! timeout bounds the whole request. The body deadline bounds the time that the client needs to
//! send the body.

use std::error::Error;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use recipe_app::error::{AppError, TimeoutStage};
use tower_http::timeout::{RequestBodyDeadlineLayer, TimeoutError};

use crate::error::WebError;
use crate::export_path::is_export_path;
use crate::settings::WebSettings;
use crate::state::WebState;

/// Return the error of a request that ran out of time: `E-WEB-009`.
pub(crate) const fn timed_out() -> WebError {
    WebError::App(AppError::Timeout {
        stage: TimeoutStage::Request,
    })
}

/// Give the rest of the stack at most `http.request_timeout` to answer.
///
/// The function solves one problem: a slow handler must not hold a connection and a pool slot
/// without limit. On the limit the future of the rest of the stack drops, and the request gets
/// `E-WEB-009` from `render_errors` (ADR-0058 (g), (h)).
///
/// An export path has no request timeout (ADR-0072 (a)). An export keeps its own limits
/// (ADR-0072 (b)). So this layer never hides a committed `VersionExported` entry (ADR-0072 (c)).
pub(crate) async fn request_timeout(
    State(state): State<WebState>,
    request: Request,
    next: Next,
) -> Result<Response, WebError> {
    if is_export_path(request.uri().path()) {
        return Ok(next.run(request).await);
    }
    let limit = state.settings().request_timeout;
    tokio::time::timeout(limit, next.run(request))
        .await
        .map_err(|_elapsed| timed_out())
}

/// Return the layer that sets one deadline `http.body_read_timeout` for the whole request body.
///
/// The layer solves one problem: a client that sends one byte at a time must not hold the request
/// open. `RequestBodyTimeoutLayer` starts its timer again on each frame, so it does not bound that
/// client (ADR-0058 (i)).
pub(crate) fn body_deadline(settings: &WebSettings) -> RequestBodyDeadlineLayer {
    RequestBodyDeadlineLayer::new(settings.body_read_timeout)
}

/// Tell whether `error` or one of its sources is the timeout error of a request body.
///
/// The function solves one problem: the reader of a body wraps the error of the body, so the
/// timeout error is not the first error of the chain (ADR-0058 (j)).
pub(crate) fn is_body_timeout(error: &(dyn Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(error) = current {
        if error.is::<TimeoutError>() {
            return true;
        }
        current = error.source();
    }
    false
}

#[cfg(test)]
mod tests;
