//! `GET /recipes/{ref}/versions`: one page of the version table (`api.md` section 2.5, design D7).
//!
//! An HTMX request for another target than `main` gets the fragment `versions-table.html`. Any
//! other request gets the recipe page with that page of versions (ADR-0037 (w)).

use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::recipe::{ListVersions, ListVersionsInput};

use super::table::{VersionTable, VersionsTableFragment};
use crate::error::WebError;
use crate::layout::Layout;
use crate::pages::recipes::page::{PageQuery, show};
use crate::pages::recipes::path_reference;
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// `GET /recipes/{ref}/versions`: show one page of the versions, the newest number first.
pub(crate) async fn list(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<PageQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let reference = path_reference(path)?;
    let page = PageQuery::page(query)?;
    let t = Texts::new(state.bundles(), request.lang());
    let layout = Layout::for_session(t, &request, String::new(), None)?;
    if !request.is_htmx() || layout.is_content() {
        return show(&state, &request, reference, page).await;
    }
    let input = ListVersionsInput {
        reference: reference.clone(),
        page: page.index,
    };
    let list = ListVersions::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    let fragment = VersionsTableFragment {
        t,
        versions: VersionTable::new(t, &reference, &list, page),
    };
    Ok(render(&fragment, request.request_id())?.into_response())
}
