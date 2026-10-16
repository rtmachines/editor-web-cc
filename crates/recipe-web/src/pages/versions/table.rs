//! The version table of a recipe: the view of `versions-table.html` (design D7, ADR-0043 (a)).
//!
//! The recipe page includes the table, and `GET /recipes/{ref}/versions` answers it alone. The
//! pager links go to that route, so a boosted link swaps only `#versions-table`.

use askama::Template;
use recipe_app::ports::VersionSummary;
use recipe_app::recipe::VersionList;
use recipe_domain::recipe::ReferenceNumber;

use crate::pages::path_values::Page;
use crate::pages::recipes::{pager_links, state_badge};
use crate::pages::render::{instant, page_url};
use crate::pages::texts::Texts;

/// One row of the version table.
pub(crate) struct VersionRow {
    /// The address of the version page.
    pub(crate) href: String,
    /// The number of the version.
    pub(crate) number: u32,
    /// The kind of the badge of the state: `draft`, `approved`, or `retired`.
    pub(crate) kind: &'static str,
    /// The word of the state in the UI language.
    pub(crate) word: String,
    /// The change note of the version.
    pub(crate) note: String,
    /// The instant of the creation in the format of `design-system.md` section 5.
    pub(crate) created: String,
    /// The instant of the approval, or an empty text for a version that was never approved.
    pub(crate) approved: String,
}

/// The data of the version table: one page of rows and the pager.
pub(crate) struct VersionTable {
    /// The rows of the page, the newest number first.
    pub(crate) rows: Vec<VersionRow>,
    /// The address of the previous page, or an empty text.
    pub(crate) previous_url: String,
    /// The address of the next page, or an empty text.
    pub(crate) next_url: String,
    /// The number of the page in the URL.
    pub(crate) current: u32,
    /// True for a page after the last page.
    pub(crate) after_end: bool,
    /// The address of the first page.
    pub(crate) first_url: String,
}

impl VersionTable {
    /// Build the table of the recipe `reference` from one page of versions.
    pub(crate) fn new(
        t: Texts<'_>,
        reference: &ReferenceNumber,
        list: &VersionList,
        page: Page,
    ) -> Self {
        let path = format!("/recipes/{reference}/versions");
        let after_end = list.rows.is_empty() && page.index > 0;
        let (previous_url, next_url) = pager_links(&path, (page, list.has_next, after_end), &[]);
        Self {
            rows: list
                .rows
                .iter()
                .map(|row| version_row(t, &path, row))
                .collect(),
            previous_url,
            next_url,
            current: page.number,
            after_end,
            first_url: page_url(&path, &[], 1),
        }
    }
}

/// Build the row of one version. `path` is the address of the version list of the recipe.
fn version_row(t: Texts<'_>, path: &str, summary: &VersionSummary) -> VersionRow {
    let (kind, key) = state_badge(summary.state);
    VersionRow {
        href: format!("{path}/{}", summary.number),
        number: summary.number,
        kind,
        word: t.get(key),
        note: summary.change_note.to_string(),
        created: instant(&summary.created_at),
        approved: summary
            .approved_at
            .as_ref()
            .map(instant)
            .unwrap_or_default(),
    }
}

/// The fragment `versions-table.html`: the table and its pager, without the layout.
#[derive(Template)]
#[template(path = "versions-table.html")]
pub(crate) struct VersionsTableFragment<'a> {
    /// The texts of the UI language.
    pub(crate) t: Texts<'a>,
    /// One page of the version table with its pager.
    pub(crate) versions: VersionTable,
}
