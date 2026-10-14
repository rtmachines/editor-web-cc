//! The recipe list page: `GET /recipes` (`api.md` section 2, design D9, ADR-0036 l).
//!
//! Step P3.1 shows the layout, the heading, and the empty state. Step P3.3 adds the rows.

use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use recipe_app::recipe::ListRecipes;

use super::render::render;
use super::request::PageRequest;
use super::texts::Texts;
use crate::error::WebError;
use crate::layout::{Layout, NavTarget};
use crate::state::WebState;

/// The page `recipes.html`.
#[derive(Template)]
#[template(path = "recipes.html")]
struct RecipesPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
}

/// `GET /recipes`: show the recipe list. The query checks the permission first.
pub(crate) async fn list(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let auth = request.auth()?;
    ListRecipes::new(state.identity().app().clone())
        .execute(auth.user(), &request.meta())
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let layout = Layout::for_session(
        t,
        &request,
        t.get("recipes-title"),
        Some(NavTarget::Recipes),
    )?;
    Ok(render(&RecipesPage { layout, t }, request.request_id())?.into_response())
}

#[cfg(test)]
mod tests;
