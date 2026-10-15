//! The usage page: `GET /machines/{code}/types/{name}/usage` (`api.md` section 2.3, design D6).

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::plant::{GetTypeUsage, TypeRef, TypeUsage, TypeUsageInput, TypeUsageLine};
use serde::Deserialize;

use super::{current_nav, named_revision, query_values, revision_href, type_href};
use crate::error::WebError;
use crate::layout::Layout;
use crate::pages::path_values::{Page, page, path_code_type, revision};
use crate::pages::render::{page_url, render};
use crate::pages::{PageRequest, Texts};
use crate::state::WebState;

/// The query of `GET /machines/{code}/types/{name}/usage`: `rev` and `page`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UsageQuery {
    /// The revision. A missing value means the latest revision.
    rev: Option<String>,
    /// The page of the list. A missing value means the first page.
    page: Option<String>,
}

/// One row of the table: a parameter that uses the revision.
struct UsageRow {
    /// The reference number of the recipe, as data text. The recipe pages come in step P3.3.
    reference: String,
    /// The number of the version.
    number: u32,
    /// The state of the version: `draft`, `approved`, or `retired`. It comes from code.
    kind: &'static str,
    /// The word of the state in the language of the page.
    word: String,
    tag: String,
    /// The type expression of the type of the parameter.
    type_text: String,
}

/// The page `type-usage.html`.
#[derive(Template)]
#[template(path = "type-usage.html")]
struct UsagePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    title: String,
    rows: Vec<UsageRow>,
    /// The address of the previous page, or an empty text on the first page.
    previous: String,
    /// The address of the next page, or an empty text on the last page.
    next: String,
    /// The number of the page, from 1.
    current: u32,
    /// True for a page after the last page: no row, and the page number is above 1 (D20, U16).
    after_end: bool,
    /// The address of the first page. The page `after_end` links to it.
    first_url: String,
    back_href: String,
    back_text: String,
}

/// Turn a usage line into a row of the table.
fn usage_row(t: Texts<'_>, line: &TypeUsageLine) -> UsageRow {
    let kind = line.row.status.as_sql();
    UsageRow {
        reference: line.row.reference.to_string(),
        number: line.row.number,
        kind,
        word: t.get(&format!("status-{kind}")),
        tag: line.row.tag.to_string(),
        type_text: line.type_text.clone(),
    }
}

/// Build the addresses of the previous, the next, and the first page.
///
/// An empty text means that the page has no such neighbor. The `rev` of the request is the
/// revision that the use case resolved, so the links stay on one revision. A page after the end
/// has no previous link, because the page before it can lie after the end too.
fn pager_links(usage: &TypeUsage, current: Page) -> (String, String, String) {
    let path = format!("{}/usage", type_href(usage.machine.code(), &usage.name));
    let revision = usage.revision.to_string();
    let link = |number: Option<u32>| {
        number
            .map(|number| page_url(&path, &[("rev", revision.as_str())], number))
            .unwrap_or_default()
    };
    let after_end = usage.rows.is_empty() && current.index > 0;
    let previous = current
        .number
        .checked_sub(1)
        .filter(|number| *number > 0 && !after_end);
    let next = usage
        .has_next
        .then(|| current.number.checked_add(1))
        .flatten();
    (link(previous), link(next), link(Some(1)))
}

/// `GET /machines/{code}/types/{name}/usage`: list the parameters that use a revision.
pub(super) async fn show(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    query: Result<Query<UsageQuery>, QueryRejection>,
) -> Result<Response, WebError> {
    let (code, name) = path_code_type(path)?;
    let values = query_values(query)?;
    let wanted = values.rev.as_deref().map(revision).transpose()?;
    let current = page(values.page.as_deref())?;
    let input = TypeUsageInput {
        type_ref: TypeRef {
            code,
            name,
            revision: wanted,
        },
        page: current.index,
    };
    let usage = GetTypeUsage::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let title = named_revision(t, "usage-title", &usage.name, usage.revision);
    let layout = Layout::for_session(t, &request, title.clone(), current_nav())?;
    let (previous, next, first_url) = pager_links(&usage, current);
    let after_end = usage.rows.is_empty() && current.index > 0;
    let view = UsagePage {
        layout,
        t,
        title,
        rows: usage.rows.iter().map(|line| usage_row(t, line)).collect(),
        previous,
        next,
        current: current.number,
        after_end,
        first_url,
        back_href: revision_href(code, &usage.name, usage.revision),
        back_text: named_revision(t, "type-title", &usage.name, usage.revision),
    };
    Ok(render(&view, request.request_id())?.into_response())
}
