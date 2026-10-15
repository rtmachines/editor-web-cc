//! `GET /machines/{code}/units`: the whole unit tree (`api.md` section 2.2, design D6 and D7).
//!
//! An HTMX request for another target than `main` gets the fragment `unit-tree.html`. Any other
//! request gets the page `unit-tree-page.html`: the full page, or the content of `main` for an
//! HTMX request with `HX-Target: content` (ADR-0037 w).

use askama::Template;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::plant::{GetUnitTree, UnitTreeInput};

use super::tree::{Scope, UnitTree};
use crate::error::WebError;
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::path_code;
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The fragment `unit-tree.html`: the tree and nothing around it.
#[derive(Template)]
#[template(path = "unit-tree.html")]
struct UnitTreeFragment<'a> {
    t: Texts<'a>,
    tree: UnitTree,
}

/// The page `unit-tree-page.html`: the layout, the heading, the tree, and a link to the machine.
#[derive(Template)]
#[template(path = "unit-tree-page.html")]
struct UnitTreePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    tree: UnitTree,
    code: String,
    machine_href: String,
}

/// `GET /machines/{code}/units`: show the live units of the live machine with this code.
pub(crate) async fn show(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let auth = request.auth()?;
    let machine = GetUnitTree::new(state.identity().app().clone())
        .execute(auth.user(), &request.meta(), UnitTreeInput { code })
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let tree = UnitTree::build(&machine, Scope::Live, auth.user().role(), None, true);
    let title = t.with_arg("unit-tree-title", "code", code);
    let layout = Layout::for_session(t, &request, title, Some(NavTarget::Machines))?;
    if request.is_htmx() && !layout.is_content() {
        let fragment = UnitTreeFragment { t, tree };
        return Ok(render(&fragment, request.request_id())?.into_response());
    }
    let page = UnitTreePage {
        layout,
        t,
        tree,
        code: code.to_string(),
        machine_href: format!("/machines/{code}"),
    };
    Ok(render(&page, request.request_id())?.into_response())
}
