//! `GET /recipes/{ref}/history`: the timeline of one recipe (`api.md` section 2.9, ADR-0051 (g)).
//!
//! The page shows the entries with the aggregate id of the recipe, newest first, without states.
//! A role with `AuditView` or `AuditViewRecipes` gets a link to the entry page in each row. The
//! Operator gets no link. A deleted recipe shows its history to the Administrator only.

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use recipe_app::audit::{GetRecipeHistory, RecipeHistory, RecipeHistoryInput};
use recipe_app::auth::Operation;
use recipe_app::ports::AuditEntryId;
use serde::Deserialize;

use super::audit::rows::AuditRowView;
use super::path_values::{cursor, query_values};
use super::recipes::path_reference;
use super::render::render;
use super::request::PageRequest;
use super::texts::Texts;
use crate::error::WebError;
use crate::layout::{Layout, NavTarget};
use crate::routes::{RouteEntry, operation_row};
use crate::state::WebState;

/// Return the route row of the history page.
pub(crate) fn rows() -> Vec<RouteEntry> {
    vec![operation_row(
        Method::GET,
        "/recipes/{ref}/history",
        Operation::GetRecipeHistory,
        get(history),
    )]
}

/// The query string of the history page. A key other than `before` gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryQuery {
    before: Option<String>,
}

/// The page `history.html`.
#[derive(Template)]
#[template(path = "history.html")]
struct HistoryPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    deleted: bool,
    recipe_text: String,
    recipe_href: String,
    /// The link to the audit list of the whole recipe, for a role with an audit permission.
    whole_href: Option<String>,
    caption: String,
    empty: String,
    rows: Vec<AuditRowView>,
    pager_label: String,
    newest: String,
    older: String,
}

impl<'a> HistoryPage<'a> {
    /// Build the page from the history of a recipe.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(t: Texts<'a>, request: &PageRequest, history: &RecipeHistory) -> Result<Self, WebError> {
        let role = request.auth()?.user().role();
        let linked = Operation::ListAuditEntries.requirement().allows(role);
        let recipe = &history.recipe;
        let reference = recipe.reference();
        let recipe_text = t.with_arg("recipe-title", "reference", reference);
        let layout = Layout::for_session(
            t,
            request,
            t.with_arg("history-title", "reference", reference),
            Some(NavTarget::Recipes),
        )?;
        let path = format!("/recipes/{reference}/history");
        let older = history
            .page
            .rows
            .last()
            .filter(|_row| history.page.has_older)
            .map(|row| format!("{path}?before={}", row.id))
            .unwrap_or_default();
        Ok(Self {
            t,
            deleted: recipe.is_deleted(),
            recipe_text,
            recipe_href: format!("/recipes/{reference}"),
            whole_href: linked.then(|| format!("/audit?aggregate={}", recipe.id().as_uuid())),
            caption: t.get("history-caption"),
            empty: t.get("history-empty"),
            rows: history
                .page
                .rows
                .iter()
                .map(|row| AuditRowView::new(t, row, linked))
                .collect(),
            pager_label: t.get("pager-label"),
            newest: path,
            older,
            layout,
        })
    }
}

/// `GET /recipes/{ref}/history`: show one page of the history of the recipe.
async fn history(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<HistoryQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let reference = path_reference(path)?;
    let before = cursor(query_values(query)?.before.as_deref())?.map(AuditEntryId::from);
    let history = GetRecipeHistory::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            RecipeHistoryInput { reference, before },
        )
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let page = HistoryPage::new(t, &request, &history)?;
    Ok(render(&page, request.request_id())?.into_response())
}

#[cfg(test)]
mod tests;
