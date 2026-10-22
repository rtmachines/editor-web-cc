//! `GET /recipes/{ref}/versions/{n}/diff/{m}`: the diff page of two versions (`api.md` section 2.5,
//! ADR-0050, design D4).
//!
//! The page shows the changes from version `m` to version `n` with the summary and the sections
//! of the shared view `DiffView`. The result of the query holds the permit of the diff gate. The
//! handler drops it after the page renders (ADR-0050 (g)).

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::error::{AppError, Missing};
use recipe_app::recipe::{DiffChoice, DiffVersions, DiffVersionsInput, VersionDiff};
use recipe_domain::recipe::{DiffKind, ReferenceNumber};
use serde::Deserialize;

use super::diff_view::{DiffLinks, DiffView, kind as read_kind};
use super::path_values::{Page, page as read_page, query_values, reference, version_number};
use super::render::render;
use super::request::PageRequest;
use super::texts::Texts;
use crate::error::WebError;
use crate::layout::{Layout, NavTarget};
use crate::state::WebState;

/// The query string of the diff page. A key other than `kind` and `page` gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiffQuery {
    kind: Option<String>,
    page: Option<String>,
}

/// The checked query of the diff page.
struct Choice {
    /// The section of the diff, or `None` for the page that fills the sections in their order.
    kind: Option<DiffKind>,
    /// The page of the diff.
    page: Page,
}

impl DiffQuery {
    /// Read the section and the page.
    ///
    /// # Errors
    ///
    /// Returns `422 E-WEB-008` for a bad `kind` or `page`.
    fn check(self) -> Result<Choice, WebError> {
        Ok(Choice {
            kind: read_kind(self.kind.as_deref())?,
            page: read_page(self.page.as_deref())?,
        })
    }
}

/// The path values of the route: the recipe, the new side `n`, and the old side `m`.
struct Sides {
    reference: ReferenceNumber,
    new: u32,
    old: u32,
}

/// Read the route `/recipes/{ref}/versions/{n}/diff/{m}`.
///
/// A rejection of axum, for example a value that is not UTF-8, holds no value. So it gives
/// `Missing::MalformedRecipe`, as a malformed reference number does.
///
/// # Errors
///
/// Returns `404 E-WEB-005` for a bad reference number, a bad version number, or a rejection.
fn path_sides(
    path: Result<Path<(String, String, String)>, PathRejection>,
) -> Result<Sides, WebError> {
    let Path((text, new, old)) = path.map_err(|_rejection| {
        WebError::App(AppError::NotFound {
            entity: Missing::MalformedRecipe,
        })
    })?;
    Ok(Sides {
        reference: reference(&text)?,
        new: version_number(&new)?,
        old: version_number(&old)?,
    })
}

/// The page `diff.html`.
#[derive(Template)]
#[template(path = "diff.html")]
struct DiffPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    deleted: bool,
    diff: DiffView,
    /// The empty address and text of the jump link. The page has no form to jump to.
    no_jump: &'static str,
    back_href: String,
    back_text: String,
}

impl<'a> DiffPage<'a> {
    /// Build the page from the result of the query.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(t: Texts<'a>, request: &PageRequest, result: &VersionDiff) -> Result<Self, WebError> {
        let recipe = &result.recipe;
        let (new, old) = (result.new.number(), result.old.number());
        let base = format!("/recipes/{}/versions", recipe.reference());
        let path = format!("{base}/{new}/diff/{old}");
        let links = DiffLinks {
            path: &path,
            keep: &[],
            versions: &base,
            old: Some(old),
            new,
        };
        let mut args = fluent::FluentArgs::new();
        args.set("new", new.to_string());
        args.set("old", old.to_string());
        args.set("reference", recipe.reference().to_string());
        let title = t.with("diff-title", &args);
        Ok(Self {
            layout: Layout::for_session(t, request, title, Some(NavTarget::Recipes))?,
            t,
            deleted: recipe.is_deleted(),
            diff: DiffView::new(t, &result.diff, &links),
            no_jump: "",
            back_text: t.with_arg("diff-back", "number", new),
            back_href: format!("{base}/{new}"),
        })
    }
}

/// `GET /recipes/{ref}/versions/{n}/diff/{m}`: show one page of the diff of two versions.
pub(crate) async fn diff(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    query: Result<Query<DiffQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let sides = path_sides(path)?;
    let choice = query_values(query)?.check()?;
    let input = DiffVersionsInput {
        reference: sides.reference,
        number: sides.new,
        old_number: sides.old,
        choice: DiffChoice {
            kind: choice.kind,
            page: choice.page.index,
        },
    };
    let result = DiffVersions::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let page = DiffPage::new(t, &request, &result)?;
    let html = render(&page, request.request_id())?;
    // The permit of the diff gate lives until the page is text (ADR-0050 (g)).
    drop(page);
    drop(result);
    Ok(html.into_response())
}

#[cfg(test)]
mod bounds_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod kind_tests;
#[cfg(test)]
mod link_tests;
#[cfg(test)]
mod page_tests;
#[cfg(test)]
mod stored_tests;
#[cfg(test)]
mod test_world;
