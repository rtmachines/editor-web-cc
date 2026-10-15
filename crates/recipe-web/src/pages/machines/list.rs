//! `GET /machines`: the machine list (`api.md` section 2.2, design D6, ADR-0037 l and x).
//!
//! The URL counts pages from 1 and the query counts them from 0. The search text and the choice to
//! show archived machines stay in the links to the other pages.

use askama::Template;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::Operation;
use recipe_app::plant::{ListMachines, ListMachinesInput, MachineList};
use recipe_app::ports::{MachineFilter, MachineSummary};
use recipe_domain::plant::MachineCode;
use recipe_domain::text::Text;
use serde::Deserialize;

use super::{platform_text, query_values};
use crate::error::{FieldReason, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::{Page, archived_machine, flag, page as read_page};
use crate::pages::render::{page_url, render};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The path of the list. The pager and the search form use it.
const LIST_PATH: &str = "/machines";

/// The raw query string. A key that is not listed here gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListQuery {
    q: Option<String>,
    page: Option<String>,
    show_archived: Option<String>,
    archived: Option<String>,
}

/// The checked query: every value is a value object or a flag.
struct Checked {
    /// The search text as typed, or an empty text. The form shows it again.
    q: String,
    filter: MachineFilter,
    page: Page,
    archived: Option<MachineCode>,
}

impl ListQuery {
    /// Check `q`, `page`, `show_archived`, and `archived`.
    ///
    /// # Errors
    ///
    /// Returns `422 E-WEB-008` for the first value that is not valid:
    /// - a search text that `Text::parse` refuses,
    /// - a page that is not a whole number from 1,
    /// - a flag other than `1`,
    /// - an `archived` that is not a machine code.
    fn check(self) -> Result<Checked, WebError> {
        let q = self.q.unwrap_or_default();
        // A search text of spaces only is no search text (D20, U17). Any other text keeps its rule.
        let q = if q.chars().all(|c| c == ' ') {
            String::new()
        } else {
            q
        };
        let text = if q.is_empty() {
            None
        } else {
            let parsed = Text::parse(&q).map_err(|_reason| WebError::BadRequest {
                field: "machine_q",
                reason: FieldReason::SearchText,
            })?;
            Some(parsed)
        };
        let page = read_page(self.page.as_deref())?;
        let include_archived = flag("show_archived", self.show_archived.as_deref())?;
        let archived = self.archived.as_deref().map(archived_machine).transpose()?;
        Ok(Checked {
            q,
            filter: MachineFilter {
                text,
                include_archived,
            },
            page,
            archived,
        })
    }
}

/// One row of the table.
struct MachineRow {
    code: String,
    href: String,
    name: String,
    platform: String,
    archived: bool,
}

impl MachineRow {
    /// Build the row. A live machine links to its page by code. An archived machine links by
    /// identifier, because a new machine can use its code (A32).
    fn of(t: Texts<'_>, summary: &MachineSummary) -> Self {
        let archived = summary.archived_at.is_some();
        let href = if archived {
            format!("/machines/by-id/{}", summary.id)
        } else {
            format!("/machines/{}", summary.code)
        };
        Self {
            code: summary.code.to_string(),
            href,
            name: summary.name.to_string(),
            platform: platform_text(t, summary.platform),
            archived,
        }
    }
}

/// The page `machines.html`.
#[derive(Template)]
#[template(path = "machines.html")]
struct MachinesPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    q: String,
    show_archived: bool,
    can_create: bool,
    rows: Vec<MachineRow>,
    previous_url: String,
    next_url: String,
    current: u32,
    /// True for a page after the last page: no row, and the page number is above 1 (D20, U16).
    after_end: bool,
    /// The address of the first page. The page `after_end` links to it.
    first_url: String,
}

/// Return the pairs of the query that the links to the other pages keep.
///
/// An empty search text is left out, so `/machines?page=2` stays short.
fn kept_pairs(q: &str, include_archived: bool) -> Vec<(&str, &str)> {
    [
        (!q.is_empty()).then_some(("q", q)),
        include_archived.then_some(("show_archived", "1")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Build the addresses of the previous and the next page. An empty address omits its link.
///
/// A page after the end has no previous link, because the page before it can lie after the end too.
fn pager_links(
    page: Page,
    has_next: bool,
    after_end: bool,
    pairs: &[(&str, &str)],
) -> (String, String) {
    let previous = page
        .number
        .checked_sub(1)
        .filter(|number| *number > 0 && !after_end)
        .map(|number| page_url(LIST_PATH, pairs, number))
        .unwrap_or_default();
    let next = has_next
        .then(|| page.number.checked_add(1))
        .flatten()
        .map(|number| page_url(LIST_PATH, pairs, number))
        .unwrap_or_default();
    (previous, next)
}

impl<'a> MachinesPage<'a> {
    /// Build the page from the checked query and one page of machines.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(
        t: Texts<'a>,
        request: &PageRequest,
        checked: &Checked,
        list: MachineList,
    ) -> Result<Self, WebError> {
        let role = request.auth()?.user().role();
        let mut layout = Layout::for_session(
            t,
            request,
            t.get("machines-title"),
            Some(NavTarget::Machines),
        )?;
        if let Some(code) = list.archived {
            layout = layout.with_toast(t.with_arg("machine-archived-toast", "code", code));
        }
        let pairs = kept_pairs(&checked.q, checked.filter.include_archived);
        let after_end = list.rows.is_empty() && checked.page.index > 0;
        let (previous_url, next_url) = pager_links(checked.page, list.has_next, after_end, &pairs);
        Ok(Self {
            layout,
            t,
            q: checked.q.clone(),
            show_archived: checked.filter.include_archived,
            can_create: Operation::CreateMachine.requirement().allows(role),
            rows: list
                .rows
                .iter()
                .map(|summary| MachineRow::of(t, summary))
                .collect(),
            previous_url,
            next_url,
            current: checked.page.number,
            after_end,
            first_url: page_url(LIST_PATH, &pairs, 1),
        })
    }
}

/// `GET /machines`: list the machines, one page at a time. The query checks the permission first.
pub(crate) async fn list(
    State(state): State<WebState>,
    request: PageRequest,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Response, WebError> {
    let checked = query_values(query)?.check()?;
    let input = ListMachinesInput {
        filter: checked.filter.clone(),
        page: checked.page.index,
        archived: checked.archived,
    };
    let machines = ListMachines::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let page = MachinesPage::new(t, &request, &checked, machines)?;
    Ok(render(&page, request.request_id())?.into_response())
}
