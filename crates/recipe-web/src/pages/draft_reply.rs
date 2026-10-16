//! The answers of a form on a draft: the form with errors and the clean form (design D9, D10).
//!
//! Each form handler calls one function here, so every form answers in one way (ADR-0041).

use askama::Template;
use axum::http::{HeaderName, HeaderValue};
use axum::response::{Html, IntoResponse, Response};
use recipe_app::error::AppError;
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::{DraftOutput, GetVersion, GetVersionInput};
use recipe_domain::audit::RequestId;
use recipe_domain::recipe::RecipeError;

use super::render::instant;
use super::{PageRequest, Texts};
use crate::error::{ConflictView, FieldError, FormErrors, RenderedForm, WebError};
use crate::layout::SessionFrame;
use crate::state::WebState;

/// The HTMX header that fires an event in the browser after the swap.
const HX_TRIGGER: HeaderName = HeaderName::from_static("hx-trigger");

/// Build the error of a form with every line and the form with the sent input (ADR-0041 (e)).
///
/// `form` builds the form template, which owns the error lines of its inputs. The caller fills the
/// template with the sent texts and the sent `row_version`. The template must start with
/// `<form id="...` and hold the literal `hx-swap-oob="true"`. A form that fails this check gives
/// `E-WEB-010`, because the answer would lose the input.
///
/// For a request without HTMX, the form also holds the session parts of the layout. The error
/// layer then shows the summary and the form on a full page of the session (`api.md` 5.3).
pub(crate) fn refused<T, F>(
    state: &WebState,
    request: &PageRequest,
    lines: NonEmpty<FieldError>,
    form: F,
) -> WebError
where
    T: Template,
    F: FnOnce(FormErrors) -> T,
{
    let errors = FormErrors::new(lines.iter(), state.bundles(), request.lang());
    let frame = (!request.is_htmx())
        .then(|| SessionFrame::of(Texts::new(state.bundles(), request.lang()), request))
        .flatten();
    match RenderedForm::from_template(&form(errors), request.request_id()) {
        Ok(rendered) => WebError::Fields {
            lines,
            form: Some(rendered.with_frame(frame)),
        },
        Err(defect) => defect,
    }
}

/// Turn the refusal of a use case into the line of its form (ADR-0041 (e)).
///
/// A domain error of the Plant Model or of Recipe Management becomes one line. `target` gives the
/// `id` of the input that the error names, or `None`, so the line stands only in the summary. The
/// form of the answer then shows only this line, and no line of an earlier answer stays.
///
/// # Errors
///
/// Returns every other error as it is: an access error, a missing row, a conflict, a storage
/// failure, and a failed snapshot check (`E-RCP-020`), which lists its own items.
pub(crate) fn refusal_lines(
    error: AppError,
    target: impl FnOnce(&AppError) -> Option<String>,
) -> Result<NonEmpty<FieldError>, WebError> {
    let place = target(&error);
    match domain_line(&error, place) {
        Some(line) => Ok(NonEmpty::new(line)),
        None => Err(WebError::from(error)),
    }
}

/// Return the line of a domain error of a refused form, with the input `target` or none.
///
/// A domain error of the Plant Model or of Recipe Management gets the line, so every error answer
/// of a form swaps the form (ADR-0041 (e)). The snapshot check `E-RCP-020` lists its own items, so
/// it gets `None`, as every other error does.
pub(crate) fn domain_line(error: &AppError, target: Option<String>) -> Option<FieldError> {
    match error {
        AppError::Plant(inner) => Some(FieldError::plant(inner, target)),
        AppError::Recipe(RecipeError::ValidationFailed { .. }) => None,
        AppError::Recipe(inner) => Some(FieldError::recipe(inner, target)),
        _other => None,
    }
}

/// Append the clean form to the body of a success answer (ADR-0041 (h)).
///
/// The form holds the saved texts and no error line. It swaps out of band, as in [`refused`].
///
/// # Errors
///
/// Returns `WebError::Internal` (`E-WEB-010`) when the form fails the check of [`refused`].
pub(crate) fn with_form<T: Template>(
    body: Html<String>,
    form: &T,
    request_id: RequestId,
) -> Result<Html<String>, WebError> {
    let rendered = RenderedForm::from_template(form, request_id)?;
    let Html(mut text) = body;
    text.push_str(rendered.as_str());
    Ok(Html(text))
}

/// Build the success answer of a draft command with `HX-Trigger: {"rowVersion": <n>}` (`api.md` 4).
///
/// The value is the new `row_version` of `output`. It is an integer, so no user text enters the
/// header. The script `row-version.js` writes it into every hidden field `row_version`.
///
/// # Errors
///
/// Returns `WebError::Internal` (`E-WEB-010`) when the header value is not valid.
pub(crate) fn success(
    output: &DraftOutput,
    body: Html<String>,
    request_id: RequestId,
) -> Result<Response, WebError> {
    let trigger = HeaderValue::try_from(format!("{{\"rowVersion\": {}}}", output.row_version))
        .map_err(|_invalid| WebError::Internal { request_id })?;
    let mut response = body.into_response();
    response.headers_mut().insert(HX_TRIGGER, trigger);
    Ok(response)
}

/// Turn the error of a draft command into its answer (design D10).
///
/// `AppError::ConcurrentEdit` becomes `WebError::Conflict`. The function reads the version with
/// `GetVersion` as the user of the request, for the current `row_version` and the last change.
/// A failed read still gives the conflict, and the mapping of `WebError` logs the failure. Any
/// other error stays as it is.
pub(crate) async fn conflict(state: &WebState, request: &PageRequest, error: AppError) -> WebError {
    let input = match &error {
        AppError::ConcurrentEdit {
            reference, number, ..
        } => GetVersionInput {
            reference: reference.clone(),
            number: *number,
            rebind_bound: 0,
        },
        _other => return WebError::App(error),
    };
    let view = match request.auth() {
        Ok(session) => {
            let read = GetVersion::new(state.identity().app().clone())
                .execute(session.user(), &request.meta(), input)
                .await;
            match read {
                Ok(version) => ConflictView::Read {
                    row_version: version.detail.row_version,
                    last_change: version
                        .last_change
                        .map(|change| Box::new((change.login, instant(&change.at)))),
                },
                Err(failure) => ConflictView::Unread(Box::new(failure)),
            }
        }
        Err(_no_session) => ConflictView::Unread(Box::new(AppError::Unauthenticated)),
    };
    WebError::Conflict {
        error: Box::new(error),
        view,
    }
}

#[cfg(test)]
mod conflict_tests;
#[cfg(test)]
pub(crate) mod refusal_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod toast_tests;
