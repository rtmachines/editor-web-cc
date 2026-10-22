//! `POST .../parameters/{key}/delete`: the removal of a parameter (`api.md` section 2.6,
//! design D7, ADR-0040 (c) and (f)).
//!
//! An HTMX request with `HX-Target: content` gets the content of the version page with the toast
//! and `HX-Push-Url` with the address of the version. Any other HTMX request gets the parameter
//! table and the toast. A request without HTMX gets `303` to the version page.

use askama::Template;
use axum::body::{Body, to_bytes};
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, Query, State};
use axum::http::header::CONTENT_LENGTH;
use axum::http::{HeaderName, Uri};
use axum::response::{Html, Response};
use recipe_app::auth::ParameterFormPurpose;
use recipe_app::error::AppError;
use recipe_app::recipe::{DraftOutput, RemoveParameter, RemoveParameterInput};
use recipe_domain::audit::RequestId;
use recipe_domain::recipe::ParameterKey;

use super::page::read_parameter;
use super::path_parameter;
use super::reply::{Ctx, Target, load, missing_key};
use super::table::table_body;
use super::translation_routes::{RowFields, sent_row_version};
use crate::error::WebError;
use crate::pages::draft_reply::{conflict, success, with_push_url};
use crate::pages::path_values::Page;
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::versions::page::version;
use crate::state::WebState;

/// The value of `HX-Target` that asks for the content of a page (ADR-0037 (w)).
const CONTENT: &str = "content";
/// The HTMX header that fires an event after the swap.
const HX_TRIGGER: &str = "hx-trigger";
/// The first page of a list.
const FIRST_PAGE: Page = Page {
    number: 1,
    index: 0,
};
/// The attribute of the status region of the layout (`base.html`, `design-system.md` 10.7).
const TOAST_REGION: &str = "id=\"toast\"";

/// The toast of the removal of a parameter (`design-system.md` section 10.7).
#[derive(Template)]
#[template(path = "parameter-removed-toast.html")]
struct RemovedToast {
    /// The sentence `parameter-deleted` with the tag.
    text: String,
    /// True for the table answer: the toast and the empty error panel go out of band.
    oob: bool,
}

/// Put the toast `toast` into the empty status region of the content answer `response`.
///
/// HTMX swaps an out-of-band element before the content, so the content must hold the toast.
/// Askama escapes each value of the page, so the first `id="toast"` is the region of the layout.
async fn with_toast_region(
    response: Response,
    toast: &str,
    request_id: RequestId,
) -> Result<Response, WebError> {
    let defect = WebError::Internal { request_id };
    let (mut parts, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.map_err(|_failed| defect)?;
    let page =
        String::from_utf8(bytes.to_vec()).map_err(|_failed| WebError::Internal { request_id })?;
    let Some((head, rest)) = page.split_once(TOAST_REGION) else {
        return Err(WebError::Internal { request_id });
    };
    let Some((tag, tail)) = rest.split_once('>') else {
        return Err(WebError::Internal { request_id });
    };
    parts.headers.remove(CONTENT_LENGTH);
    let body = format!("{head}{TOAST_REGION}{tag}>{toast}{tail}");
    Ok(Response::from_parts(parts, Body::from(body)))
}

/// Copy the header `HX-Trigger` of the success answer of `output` into `response` (design D10).
///
/// The function solves one problem: the version page answers a `Response` and not a body, so
/// `success` cannot wrap it. One success answer gives the header, so its form stays in one place.
fn with_trigger(
    mut response: Response,
    output: &DraftOutput,
    request_id: RequestId,
) -> Result<Response, WebError> {
    let marker = success(output, Html(String::new()), request_id)?;
    if let Some(value) = marker.headers().get(HX_TRIGGER) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(HX_TRIGGER), value.clone());
    }
    Ok(response)
}

/// Answer the content of the version page with the toast and the address of the version in the
/// history.
async fn version_content(
    state: WebState,
    request: PageRequest,
    (target, output, toast): (&Target, DraftOutput, String),
) -> Result<Response, WebError> {
    let request_id = request.request_id();
    let href = target.version_href();
    let values = Ok(Path((
        target.reference.to_string(),
        target.number.to_string(),
    )));
    let query = Query::try_from_uri(&Uri::from_static("/"));
    let response = version(State(state), values, query, request).await?;
    let toast = toast_html(toast, false, request_id)?;
    let response = with_toast_region(response, &toast, request_id).await?;
    let response = with_trigger(response, &output, request_id)?;
    with_push_url(response, &href, request_id)
}

/// `POST .../parameters/{key}/delete`: remove the parameter with its limits and translations.
pub(crate) async fn remove(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<RowFields>, FormRejection>,
) -> Result<Response, WebError> {
    let target = path_parameter(path)?;
    let request_id = request.request_id();
    let key = target.key.ok_or(WebError::Internal { request_id })?;
    let ctx = Ctx {
        state: &state,
        request: &request,
        target: &target,
    };
    // The form query checks the permission of `RemoveParameter` before the fields are read (ADR-0046).
    load(ctx, ParameterFormPurpose::RemoveParameter, None, None).await?;
    let Form(fields) = form?;
    let toast = removed_text(ctx, key).await;
    let input = RemoveParameterInput {
        reference: target.reference.clone(),
        number: target.number,
        expected_row_version: sent_row_version(&fields.row_version)?,
        key,
    };
    let result = RemoveParameter::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    let output = match result {
        Ok(output) => output,
        Err(error) if matches!(error, AppError::ConcurrentEdit { .. }) => {
            return Err(conflict(&state, &request, error).await);
        }
        Err(error) => return Err(WebError::App(missing_key(error))),
    };
    if !request.is_htmx() {
        return see_other_path(&target.version_href(), request_id, []);
    }
    if request.hx_target() == Some(CONTENT) {
        return version_content(state.clone(), request, (&target, output, toast)).await;
    }
    let mut body = table_body(ctx, None, FIRST_PAGE).await?;
    body.0.push_str(&toast_html(toast, true, request_id)?);
    success(&output, body, request_id)
}

/// Render the toast `text`, out of band after the table with `oob`, or for the status region.
fn toast_html(text: String, oob: bool, request_id: RequestId) -> Result<String, WebError> {
    Ok(render(&RemovedToast { text, oob }, request_id)?.0)
}

/// Return the sentence of the toast of the removal: `parameter-deleted` with the tag.
///
/// The tag comes from a read before the removal, because the form sends only the key. When the
/// read fails, the sentence names the key, and the command gives its own error. A request without
/// HTMX gets a redirect and no toast, so it reads nothing.
async fn removed_text(ctx: Ctx<'_>, key: ParameterKey) -> String {
    if !ctx.request.is_htmx() {
        return String::new();
    }
    let tag = match read_parameter(ctx).await {
        Ok(view) => view.parameter.tag.to_string(),
        Err(_unread) => key.to_string(),
    };
    ctx.texts().with_arg("parameter-deleted", "tag", tag)
}
