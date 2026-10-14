//! The layer that turns a `WebError` into its answer (design D11, D13).

use std::sync::Arc;

use super::Retry;
use super::WebError;
use super::answer::{ErrorContext, answer};
use crate::i18n::Lang;
use crate::pipeline::{HX_REQUEST, request_id_of};
use crate::state::WebState;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

/// Render the error of a handler.
///
/// A handler returns `Err(WebError)`. Its `IntoResponse` stores the error in the response
/// extensions, because only this layer knows the request id, the language, and the HTMX header.
/// The layer reads them before the handler runs, then calls the one mapping function.
pub(crate) async fn render_errors(
    State(state): State<WebState>,
    request: Request,
    next: Next,
) -> Response {
    let lang = Lang::from_headers(request.headers());
    let htmx = request
        .headers()
        .get(HX_REQUEST)
        .is_some_and(|value| value == "true");
    let request_id = request_id_of(&request);
    let mut response = next.run(request).await;
    let Some(error) = response.extensions_mut().remove::<Arc<WebError>>() else {
        return response;
    };
    let retry = response.extensions_mut().remove::<Retry>();
    let context = ErrorContext {
        request_id,
        lang,
        htmx,
        retry,
        state: &state,
    };
    answer(&error, &context)
}
