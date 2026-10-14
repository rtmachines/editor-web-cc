//! The body limit (design D11 step 5, `api.md` section 7.6).

use axum::body::HttpBody as _;
use axum::extract::{Request, State};
use axum::http::header::CONTENT_LENGTH;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::{ByteLimit, WebError};
use crate::state::WebState;

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

/// Reject a request whose announced body is above the limit of HTML forms with `E-WEB-003`.
///
/// The layer reads no body. A body that announces nothing is checked again where a layer or an
/// extractor buffers it (`to_bytes` with the same limit).
pub(crate) async fn body_limit(
    State(state): State<WebState>,
    request: Request,
    next: Next,
) -> Result<Response, WebError> {
    let limit = state.settings().body_limit_form;
    let allowed = u64::try_from(limit).unwrap_or(u64::MAX);
    if announced_size(&request) > allowed {
        return Err(WebError::PayloadTooLarge {
            limit: ByteLimit::from(limit),
        });
    }
    Ok(next.run(request).await)
}
