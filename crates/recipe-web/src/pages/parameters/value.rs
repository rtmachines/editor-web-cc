//! `POST .../{key}/value`, `GET .../{key}/limits`, and `POST .../{key}/limits` (`api.md` section
//! 2.6, design D7, ADR-0039 (i)).
//!
//! The routes keep their fragments: the value route answers the read-only grid, and the limit
//! routes answer the read-only limit rows. The page holds no form for them, so an error answer
//! lists its lines in `#error-panel` and swaps no form.

use axum::extract::rejection::{PathRejection, QueryRejection, RawFormRejection};
use axum::extract::{Path, Query, RawForm, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::ParameterFormPurpose;
use recipe_app::error::AppError;
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::{
    DraftOutput, GetLimits, ParameterRef, ReplaceLimits, ReplaceLimitsInput, SetParameterValue,
    SetParameterValueInput,
};
use recipe_domain::recipe::ValueInput;

use super::definition::row_version;
use super::fields::{FieldSet, SentFields, read_form};
use super::form_view::{GridTexts, grid_of};
use super::grid::{GridLimitsFragment, GridValuesFragment, LimitTexts};
use super::page::{NoQuery, no_query, parameter_page, read_parameter};
use super::path_parameter;
use super::reply::{Ctx, Target, command_lines, load, missing_key};
use crate::error::{FieldError, WebError};
use crate::pages::draft_reply::{conflict, success};
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::state::WebState;

/// Build the error of lines without a form: the page has no form of the route.
fn lines_error(lines: NonEmpty<FieldError>) -> WebError {
    WebError::Fields { lines, form: None }
}

/// Read the hidden field `row_version` of a fragment route.
fn sent_row_version(sent: &SentFields) -> Result<i64, WebError> {
    row_version(sent).map_err(|line| lines_error(NonEmpty::new(line)))
}

/// Turn the error of a command of a fragment route into its answer (design D9, D10).
async fn command_failed(ctx: Ctx<'_>, error: AppError) -> WebError {
    if matches!(error, AppError::ConcurrentEdit { .. }) {
        return conflict(ctx.state, ctx.request, error).await;
    }
    match command_lines(&error, &|_path| None) {
        Some(lines) => lines_error(lines),
        None => WebError::App(missing_key(error)),
    }
}

/// Return the key of the parameter of the URL.
fn key_of(ctx: Ctx<'_>) -> Result<recipe_domain::recipe::ParameterKey, WebError> {
    ctx.target.key.ok_or(WebError::Internal {
        request_id: ctx.request.request_id(),
    })
}

/// Read the parameter again and answer its read-only grid with the new `row_version`.
async fn values_answer(ctx: Ctx<'_>, output: DraftOutput) -> Result<Response, WebError> {
    let view = read_parameter(ctx).await?;
    let request_id = ctx.request.request_id();
    let texts = GridTexts::stored(&view).map_err(|_fault| WebError::Internal { request_id })?;
    let t = ctx.texts();
    let tag = view.parameter.tag.to_string();
    let grid = grid_of(t, texts, ctx.state.settings().form_array_chunk, &tag);
    let body = render(&GridValuesFragment { t, view: &grid }, request_id)?;
    success(&output, body, request_id)
}

/// Read the limits with `GetLimits` and render the rows. A time bound shows as an IEC literal (A43).
///
/// `api.md` section 2.6 and ADR-0046 (d) give the query `GetLimits` to the limits fragment.
async fn limits_body(ctx: Ctx<'_>) -> Result<axum::response::Html<String>, WebError> {
    let input = ParameterRef {
        reference: ctx.target.reference.clone(),
        number: ctx.target.number,
        key: key_of(ctx)?,
    };
    let view = GetLimits::new(ctx.state.identity().app().clone())
        .execute(ctx.request.auth()?.user(), &ctx.request.meta(), input)
        .await?;
    let rows = LimitTexts::from_stored(&view.tree, &view.limits);
    let fragment = GridLimitsFragment {
        t: ctx.texts(),
        rows: &rows,
        root_label: view.tag.to_string(),
    };
    render(&fragment, ctx.request.request_id())
}

/// Build the context of a fragment route.
const fn ctx_of<'a>(state: &'a WebState, request: &'a PageRequest, target: &'a Target) -> Ctx<'a> {
    Ctx {
        state,
        request,
        target,
    }
}

/// `POST .../{key}/value`: set the value of the parameter (`SetParameterValue`).
pub(crate) async fn set_value(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<RawForm, RawFormRejection>,
) -> Result<Response, WebError> {
    let target = path_parameter(path)?;
    let ctx = ctx_of(&state, &request, &target);
    // The form query checks `ParameterEdit`, refuses a deleted recipe for every role, requires a
    // live draft, and gives `E-RCP-051` for a key that the draft lacks, before any field is read
    // (ADR-0013).
    load(ctx, ParameterFormPurpose::SetParameterValue, None, None).await?;
    let sent = read_form(form, FieldSet::Value)?;
    let expected_row_version = sent_row_version(&sent)?;
    let view = read_parameter(ctx).await?;
    let input = SetParameterValueInput {
        reference: target.reference.clone(),
        number: target.number,
        expected_row_version,
        key: key_of(ctx)?,
        value: ValueInput::Texts(sent.leaf_texts(&view.tree)?),
    };
    let result = SetParameterValue::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    match result {
        Ok(output) => values_answer(ctx, output).await,
        Err(error) => Err(command_failed(ctx, error).await),
    }
}

/// `GET .../{key}/limits`: show the limit rows of the parameter.
///
/// A request without HTMX gets the page of the parameter, which shows the limits in its grid.
pub(crate) async fn limits(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    query: Result<Query<NoQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let target = path_parameter(path)?;
    no_query(query)?;
    let ctx = ctx_of(&state, &request, &target);
    if !request.is_htmx() {
        return parameter_page(ctx).await;
    }
    Ok(limits_body(ctx).await?.into_response())
}

/// `POST .../{key}/limits`: replace the limit rows of the parameter (`ReplaceLimits`).
pub(crate) async fn replace_limits(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<RawForm, RawFormRejection>,
) -> Result<Response, WebError> {
    let target = path_parameter(path)?;
    let ctx = ctx_of(&state, &request, &target);
    // The form query checks `ParameterEdit` before the fields are read (ADR-0013).
    load(ctx, ParameterFormPurpose::ReplaceLimits, None, None).await?;
    let sent = read_form(form, FieldSet::Limits)?;
    let expected_row_version = sent_row_version(&sent)?;
    let limits = sent
        .limit_list()
        .map_err(|lines| match NonEmpty::try_from(lines) {
            Ok(lines) => lines_error(lines),
            Err(_empty) => WebError::Internal {
                request_id: request.request_id(),
            },
        })?;
    let input = ReplaceLimitsInput {
        reference: target.reference.clone(),
        number: target.number,
        expected_row_version,
        key: key_of(ctx)?,
        limits,
    };
    let result = ReplaceLimits::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    match result {
        Ok(output) => {
            let body = limits_body(ctx).await?;
            success(&output, body, request.request_id())
        }
        Err(error) => Err(command_failed(ctx, error).await),
    }
}
