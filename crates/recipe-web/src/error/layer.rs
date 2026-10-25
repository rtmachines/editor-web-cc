//! The layer that turns a `WebError` into its answer (design D11, D13).

use std::sync::Arc;

use super::Retry;
use super::WebError;
use super::answer::{ErrorContext, answer};
use super::json::json_answer;
use crate::export_path::is_export_path;
use crate::i18n::Lang;
use crate::panic::PanicCaught;
use crate::pipeline::{HX_REQUEST, request_id_of};
use crate::state::WebState;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use recipe_domain::audit::RequestId;

/// Take the error that an inner layer or a handler stored in `response`.
///
/// The marker of the inner panic layer becomes `WebError::Internal` with the request id
/// (ADR-0057 (a)). A response without an error or a marker gives `None`.
fn take_error(response: &mut Response, request_id: RequestId) -> Option<Arc<WebError>> {
    let extensions = response.extensions_mut();
    if extensions.remove::<PanicCaught>().is_some() {
        return Some(Arc::new(WebError::Internal { request_id }));
    }
    extensions.remove::<Arc<WebError>>()
}

/// Render the error of a handler.
///
/// A handler returns `Err(WebError)`. Its `IntoResponse` stores the error in the response
/// extensions, because only this layer knows the request id, the language, the HTMX header, and
/// the path. The layer reads them before the handler runs. Then it calls the JSON mapping on a
/// path of an export route (ADR-0053) and the HTML mapping on every other path. A panic that
/// the inner panic layer caught gets the same answer as `WebError::Internal`.
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
    let export = is_export_path(request.uri().path());
    let request_id = request_id_of(&request);
    let mut response = next.run(request).await;
    let Some(error) = take_error(&mut response, request_id) else {
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
    if export {
        json_answer(&error, &context)
    } else {
        answer(&error, &context)
    }
}
