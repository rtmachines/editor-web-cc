//! `GET .../parameters/{key}`: the page of a parameter (`api.md` section 2.6, design D7 and D8,
//! ADR-0039 (i), ADR-0043 (k)).
//!
//! The page shows the definition and the leaf grid. A live draft of a live recipe and a role that
//! holds `ParameterEdit` get the edit form. Every other reader gets the read-only grid.

use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::Operation;
use recipe_app::recipe::{GetParameter, ParameterRef, ParameterView};
use recipe_domain::recipe::VersionState;
use serde::Deserialize;

use super::form_view::{DefinitionTexts, GridTexts, ParameterFormView, Place, Shown, grid_of};
use super::grid::GridView;
use super::path_parameter;
use super::reply::{Ctx, Target, load, version_text};
use super::summary::{ParameterPage, RemoveView, SummaryView};
use super::translations::{RowSource, TranslationsBlock, translation_rows};
use crate::error::{FormErrors, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::query_values;
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::state::WebState;

/// The query string of the routes of one parameter. It has no key, so any key gives
/// `422 E-WEB-008`, and no query reaches the field `next` of the language switch.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NoQuery {}

/// Refuse a query string with a key (`api.md` section 1).
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the field `query` for any key.
pub(crate) fn no_query(query: Result<Query<NoQuery>, QueryRejection>) -> Result<(), WebError> {
    query_values(query).map(|NoQuery {}| ())
}

/// Run `GetParameter` for the parameter of `target`.
///
/// # Errors
///
/// Returns the error of the query, for example `404 E-WEB-005` for a key that the version lacks.
pub(crate) async fn read_parameter(ctx: Ctx<'_>) -> Result<ParameterView, WebError> {
    let target = ctx.target;
    let key = target.key.ok_or(WebError::Internal {
        request_id: ctx.request.request_id(),
    })?;
    let input = ParameterRef {
        reference: target.reference.clone(),
        number: target.number,
        key,
    };
    Ok(GetParameter::new(ctx.state.identity().app().clone())
        .execute(ctx.request.auth()?.user(), &ctx.request.meta(), input)
        .await?)
}

/// Return true when the reader gets the edit form: a live draft of a live recipe and `ParameterEdit`.
fn is_editable(request: &PageRequest, view: &ParameterView) -> Result<bool, WebError> {
    let role = request.auth()?.user().role();
    Ok(view.detail.state == VersionState::LiveDraft
        && !view.recipe.is_deleted()
        && Operation::UpdateParameterDefinition
            .requirement()
            .allows(role))
}

/// Build the grid of the stored value and the stored limits of `view` (A43).
///
/// # Errors
///
/// Returns `E-WEB-010` when the stored value does not match its tree, a defect of the storage.
fn stored_grid(ctx: Ctx<'_>, view: &ParameterView) -> Result<GridView, WebError> {
    let texts = GridTexts::stored(view).map_err(|_fault| WebError::Internal {
        request_id: ctx.request.request_id(),
    })?;
    let tag = view.parameter.tag.to_string();
    let chunk = ctx.state.settings().form_array_chunk;
    Ok(grid_of(ctx.texts(), texts, chunk, &tag))
}

/// Build what the edit form shows for the stored parameter: the stored texts and the grid.
///
/// The edit form of the page and the clean form after a save use it, so both show the canonical
/// texts of the store (ADR-0041 (h), A43).
///
/// # Errors
///
/// Returns the errors of [`stored_grid`].
pub(crate) fn stored_shown(ctx: Ctx<'_>, view: &ParameterView) -> Result<Shown, WebError> {
    Ok(Shown {
        texts: DefinitionTexts::stored(view),
        grid: Some(stored_grid(ctx, view)?),
        value_type: view.type_text.clone(),
        notice: String::new(),
    })
}

/// Build the edit form of the stored parameter, or the read-only grid for a reader without it.
async fn form_or_grid(
    ctx: Ctx<'_>,
    view: &ParameterView,
) -> Result<(Option<ParameterFormView>, Option<GridView>), WebError> {
    if !is_editable(ctx.request, view)? {
        return Ok((None, Some(stored_grid(ctx, view)?)));
    }
    let form = load(ctx, ctx.target.form_purpose(), None, None).await?;
    let shown = stored_shown(ctx, view)?;
    let csrf = ctx.request.csrf_token();
    let place = (shown, Place::Fragment);
    let edit = ParameterFormView::new(ctx.texts(), &form, place, csrf, FormErrors::default());
    Ok((Some(edit), None))
}

/// `GET .../parameters/{key}`: show the definition, the leaf grid, and the edit form of a draft.
pub(crate) async fn parameter(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    query: Result<Query<NoQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let target: Target = path_parameter(path)?;
    no_query(query)?;
    let ctx = Ctx {
        state: &state,
        request: &request,
        target: &target,
    };
    parameter_page(ctx).await
}

/// Render the page of the parameter of `ctx`: the full page, or the content of `main`.
///
/// # Errors
///
/// Returns the error of the queries, or `E-WEB-010` when the template cannot render.
pub(crate) async fn parameter_page(ctx: Ctx<'_>) -> Result<Response, WebError> {
    let view = read_parameter(ctx).await?;
    let (form, values) = form_or_grid(ctx, &view).await?;
    let editable = form.is_some();
    let t = ctx.texts();
    let title = t.with_arg("parameter-title", "tag", &view.parameter.tag);
    let layout = Layout::for_session(t, ctx.request, title, Some(NavTarget::Recipes))?;
    let page = ParameterPage {
        layout,
        t,
        summary: Some(SummaryView::stored(&view)),
        form,
        values,
        translations: Some(translations_of(ctx, &view, editable)),
        remove: editable.then(|| remove_of(ctx, &view)),
        version_href: ctx.target.version_href(),
        version_text: version_text(ctx),
    };
    Ok(render(&page, ctx.request.request_id())?.into_response())
}

/// Return the address of the parameter page of `ctx`.
fn parameter_href(ctx: Ctx<'_>, view: &ParameterView) -> String {
    format!(
        "{}/parameters/{}",
        ctx.target.version_href(),
        view.parameter.key
    )
}

/// Build the translation rows of the stored parameter (ADR-0043 (f)).
fn translations_of(ctx: Ctx<'_>, view: &ParameterView, editable: bool) -> TranslationsBlock {
    let source = RowSource {
        parameter_href: parameter_href(ctx, view),
        csrf_token: ctx.request.csrf_token(),
        row_version: view.detail.row_version.to_string(),
        editable,
    };
    let langs = &ctx.state.settings().translation_langs;
    TranslationsBlock {
        name_en: view.parameter.name_en.to_string(),
        description_en: view.parameter.description_en.to_string(),
        rows: translation_rows(&source, langs, &view.parameter.translations),
        toast: String::new(),
    }
}

/// Build the form of the removal of the stored parameter (ADR-0040 (c) and (f)).
fn remove_of(ctx: Ctx<'_>, view: &ParameterView) -> RemoveView {
    RemoveView {
        action: format!("{}/delete", parameter_href(ctx, view)),
        tag: view.parameter.tag.to_string(),
        csrf_token: ctx.request.csrf_token(),
        row_version: view.detail.row_version.to_string(),
    }
}
