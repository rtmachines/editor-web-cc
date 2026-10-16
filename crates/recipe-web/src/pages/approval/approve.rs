//! `POST /recipes/{ref}/versions/{n}/approve`: the approval (ADR-0049 (b), (e), and (k), design D3).
//!
//! A success answers `200` with the version page of the approved version and a toast. The toast
//! names the retired version when the approval retired one. A replay with the same `row_version`
//! answers the same page with the toast of `AlreadyApproved` and writes no audit entry.
//!
//! An error keeps the address of the page and shows its lines in `#error-panel`. A stale
//! `row_version` gives the conflict of design D10. `E-RCP-024` also swaps the approve form out of
//! band with its line under the checkbox `allow_outdated` (ADR-0041 (e)).

use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::error::AppError;
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::{
    ApprovalPreviewInput, ApproveVersion, ApproveVersionInput, ApproveVersionOutput, DiffChoice,
    GetApprovalPreview,
};
use recipe_domain::recipe::{ApprovalOutcome, RecipeError, ReferenceNumber};
use serde::Deserialize;

use super::preview::{ApproveForm, ApproveFormBlock};
use crate::error::{FieldError, WebError};
use crate::pages::draft_reply::{conflict, refused, with_push_url, with_row_version};
use crate::pages::path_values::page as read_page;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::pages::versions::page::version_answer;
use crate::pages::versions::path_version;
use crate::state::WebState;

/// The `id` and the name of the checkbox that accepts outdated parameters (A29).
const ALLOW_OUTDATED: &str = "allow_outdated";

/// The fields of the approve form. An unknown field gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApproveFields {
    /// The row version of the review (ADR-0050 (h)).
    row_version: i64,
    /// When `true`, the approval accepts the outdated parameters (A29).
    #[serde(default)]
    allow_outdated: bool,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// Return the toast of the result of the approval of version `number` (ADR-0049 (e)).
fn toast(t: Texts<'_>, number: u32, output: &ApproveVersionOutput) -> String {
    match (&output.outcome, output.retired) {
        (ApprovalOutcome::AlreadyApproved, _retired) => {
            t.with_arg("version-already-approved", "number", number)
        }
        (_approved, Some(retired)) => {
            let mut args = fluent::FluentArgs::new();
            args.set("number", number.to_string());
            args.set("retired", retired.to_string());
            t.with("toast-version-approved-retired", &args)
        }
        (_approved, None) => t.with_arg("toast-version-approved", "number", number),
    }
}

/// Build the answer `E-RCP-024`: the line under the checkbox and the form out of band.
///
/// The form needs the outdated parameters and the notice of the retired version, so the function
/// reads the preview again. The line keeps the sent `row_version`. When the preview read fails,
/// the answer holds the line in the summary only.
async fn refuse_outdated(
    state: &WebState,
    request: &PageRequest,
    (reference, number): (ReferenceNumber, u32),
    (row_version, error): (i64, RecipeError),
) -> WebError {
    let t = Texts::new(state.bundles(), request.lang());
    let line = FieldError::recipe(&error, Some(ALLOW_OUTDATED.to_owned()))
        .with_subject(t.get("field-allow-outdated"));
    let Ok(session) = request.auth() else {
        return WebError::from(AppError::Recipe(error));
    };
    let input = ApprovalPreviewInput {
        reference,
        number,
        choice: DiffChoice {
            kind: None,
            page: 0,
        },
    };
    let read = GetApprovalPreview::new(state.identity().app().clone())
        .execute(session.user(), &request.meta(), input)
        .await;
    let Ok(preview) = read else {
        return WebError::from(AppError::Recipe(error));
    };
    let form = ApproveForm::new(t, &preview, row_version, request.csrf_token());
    // The permit of the diff gate lives until the form is text (ADR-0050 (g)).
    refused(state, request, NonEmpty::new(line), |errors| {
        ApproveFormBlock {
            t,
            form: Some(form.refused(errors)),
        }
    })
}

/// Turn the error of `ApproveVersion` into its answer.
///
/// `ConcurrentEdit` gives the conflict of design D10. `E-RCP-024` gives its line under the
/// checkbox. Every other error keeps its answer of `api.md` section 5, for example one line per
/// missing limit of `E-RCP-016`.
async fn refusal(
    state: &WebState,
    request: &PageRequest,
    target: (ReferenceNumber, u32),
    (row_version, error): (i64, AppError),
) -> WebError {
    match error {
        AppError::ConcurrentEdit { .. } => conflict(state, request, error).await,
        AppError::Recipe(inner @ RecipeError::TypeRevisionOutdated { .. }) => {
            refuse_outdated(state, request, target, (row_version, inner)).await
        }
        other => WebError::from(other),
    }
}

/// `POST /recipes/{ref}/versions/{n}/approve`: approve the draft and show the version page.
pub(crate) async fn approve(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<ApproveFields>, FormRejection>,
) -> Result<Response, WebError> {
    let (reference, number) = path_version(path)?;
    let Form(fields) = form?;
    let input = ApproveVersionInput {
        reference: reference.clone(),
        number,
        expected_row_version: fields.row_version,
        allow_outdated: fields.allow_outdated,
    };
    let result = ApproveVersion::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    let target = (reference.clone(), number);
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            return Err(refusal(&state, &request, target, (fields.row_version, error)).await);
        }
    };
    let request_id = request.request_id();
    let t = Texts::new(state.bundles(), request.lang());
    let text = toast(t, number, &output);
    let page = (None, read_page(None)?);
    let answer = version_answer(&state, &request, target, page, Some(text)).await?;
    let response = answer.html.into_response();
    let response = with_row_version(response, answer.row_version, request_id)?;
    let href = format!("/recipes/{reference}/versions/{number}");
    with_push_url(response, &href, request_id)
}
