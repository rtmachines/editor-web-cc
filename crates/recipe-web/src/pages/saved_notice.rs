//! The notice answer of a command that committed and whose read then failed (ADR-0163).

use askama::Template;
use axum::http::{HeaderName, HeaderValue};
use axum::response::{IntoResponse, Response};
use recipe_domain::audit::RequestId;

use super::draft_reply::{with_push_url, with_row_version};
use super::render::{render, see_other_path};
use super::texts::Texts;
use crate::error::{WebError, display_chain};

/// The key of the notice text in both bundles.
const NOTICE_KEY: &str = "saved-notice";
/// The element that receives the notice.
const TOAST: &str = "#toast";
/// The swap mode of the notice.
const INNER_HTML: &str = "innerHTML";
/// The value of `HX-Push-Url` that keeps the address of the browser.
const NO_PUSH: &str = "false";
const HX_RETARGET: HeaderName = HeaderName::from_static("hx-retarget");
const HX_RESWAP: HeaderName = HeaderName::from_static("hx-reswap");

/// The toast and the empty error panel of the notice answer.
#[derive(Template)]
#[template(path = "saved-notice.html")]
struct SavedNotice {
    /// The fixed sentence of the notice.
    text: String,
}

/// Write the one `WARN` line of a committed change whose read failed (ADR-0163 (i)).
///
/// The function solves one problem: the command is committed and the answer is a success, so no
/// error mapping logs the read fault. This line is the one log place of that fault.
///
/// The field `chain` holds the `Display` chain of the fault, as the line of `log_error` does.
fn log_failed_read(fault: &WebError, request_id: RequestId) {
    tracing::warn!(
        error_code = %fault.code(),
        request_id = %request_id,
        chain = %display_chain(fault),
        "read failed after a committed change"
    );
}

/// Answer a request whose command committed and whose read of the answer failed (ADR-0163).
///
/// The function solves one problem: the read error must not hide a change that is saved. An
/// HTMX request gets `200` with the fixed notice in `#toast` and an empty `#error-panel` out of
/// band. The answer sets `HX-Trigger` with `rowVersion` only when `row_version` holds the value
/// that the output of the command gave. A request without HTMX gets `303` to `href`, the page of
/// the changed object. The function writes the one `WARN` line of `fault`.
///
/// # Errors
///
/// Returns `WebError::Internal` (`E-WEB-010`) when the template cannot render or when `href`
/// is not a path of this site.
pub(crate) fn saved_notice(
    htmx: bool,
    request_id: RequestId,
    texts: Texts<'_>,
    fault: &WebError,
    href: &str,
    row_version: Option<i64>,
) -> Result<Response, WebError> {
    log_failed_read(fault, request_id);
    if !htmx {
        return see_other_path(href, request_id, []);
    }
    let notice = SavedNotice {
        text: texts.get(NOTICE_KEY),
    };
    let mut response = render(&notice, request_id)?.into_response();
    let headers = response.headers_mut();
    headers.insert(HX_RETARGET, HeaderValue::from_static(TOAST));
    headers.insert(HX_RESWAP, HeaderValue::from_static(INNER_HTML));
    let response = with_push_url(response, NO_PUSH, request_id)?;
    match row_version {
        Some(value) => with_row_version(response, value, request_id),
        None => Ok(response),
    }
}

#[cfg(test)]
mod tests;
