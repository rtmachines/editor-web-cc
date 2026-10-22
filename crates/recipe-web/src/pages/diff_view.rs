//! The shared view of the diff sections (ADR-0050 (d), (e), (k), `design-system.md` section 10.13).
//!
//! The module solves one problem: the approval preview and the diff page of two versions show the
//! same summary and the same sections. Both pages build a [`DiffView`] and call the macros of
//! `components/diff.html`. The header, the blockers, and the approve form stay out of this module.
//! Each value and each bound shows as the text of the domain: canonical decimal text or an IEC
//! literal (A43).

use std::collections::BTreeMap;

use recipe_app::recipe::{DiffPage, DiffSection};
use recipe_domain::plant::{PlcTypeId, UnitId, UnitPath};
use recipe_domain::recipe::{
    DefinitionChange, DiffBounds, DiffKind, DiffPart, DiffRow, ParameterKey, Translation,
};

use crate::error::{FieldReason, WebError};
use crate::i18n::Lang;
use crate::pages::render::{page_url, query_url};
use crate::pages::texts::Texts;

/// Read the query value `kind`: the section of a diff page (ADR-0050 (e)).
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the field `kind` and `FieldReason::Query` for an unknown section.
/// A diff page has no input `kind`, so the error is `WebError::BadQuery` and its line has no link.
pub(crate) fn kind(text: Option<&str>) -> Result<Option<DiffKind>, WebError> {
    text.map(|text| {
        DiffKind::parse(text).map_err(|_reason| WebError::BadQuery {
            field: "kind",
            reason: FieldReason::Query,
        })
    })
    .transpose()
}

/// The addresses of a diff page and of the versions that it compares.
pub(crate) struct DiffLinks<'a> {
    /// The address of the page without a query, for example `/recipes/R-500/versions/2/approve`.
    pub(crate) path: &'a str,
    /// The query pairs that each link to another page of the diff keeps, for example `rv`.
    pub(crate) keep: &'a [(&'a str, &'a str)],
    /// The address of the versions of the recipe, for example `/recipes/R-500/versions`.
    pub(crate) versions: &'a str,
    /// The number of the old side, or `None` when the diff has no old side.
    pub(crate) old: Option<u32>,
    /// The number of the new side.
    pub(crate) new: u32,
}

impl<'a> DiffLinks<'a> {
    /// Return the query pairs of the section `kind`: `kind` first, then the kept pairs.
    fn kind_query(&self, kind: DiffKind) -> Vec<(&'a str, &'a str)> {
        std::iter::once(("kind", kind.as_str()))
            .chain(self.keep.iter().copied())
            .collect()
    }

    /// Return the address of the first page of the section `kind`.
    fn section_url(&self, kind: DiffKind) -> String {
        query_url(self.path, &self.kind_query(kind))
    }

    /// Return the address of the first page without a section. It keeps the pairs of `keep`.
    fn first_url(&self) -> String {
        query_url(self.path, self.keep)
    }

    /// Return the address of the page `number` (from 1) of the section `kind`.
    fn page_of(&self, kind: DiffKind, number: u32) -> String {
        page_url(self.path, &self.kind_query(kind), number)
    }

    /// Return the address of the page of the parameter `key` in the version `number`.
    fn parameter_url(&self, number: u32, key: ParameterKey) -> String {
        format!("{}/{number}/parameters/{key}", self.versions)
    }
}

/// The state of a section: the class suffix of the badge and its word (ADR-0050 (k)).
pub(crate) struct DiffState {
    /// The suffix of the class `badge-`, which also names the icon, for example `diff-limit`.
    pub(crate) badge: &'static str,
    /// The word of the state, for example "Limit changed".
    pub(crate) word: String,
}

/// Return the state of the section `kind` in the language of `t`.
fn state_of(t: Texts<'_>, kind: DiffKind) -> DiffState {
    let (badge, key) = match kind {
        DiffKind::Limits => ("diff-limit", "diff-state-limit"),
        DiffKind::Values => ("diff-changed", "diff-state-value"),
        DiffKind::Definitions => ("diff-changed", "diff-state-definition"),
        DiffKind::Translations => ("diff-changed", "diff-state-translation"),
        DiffKind::Added => ("diff-added", "diff-state-added"),
        DiffKind::Removed => ("diff-removed", "diff-state-removed"),
    };
    DiffState {
        badge,
        word: t.get(key),
    }
}

/// Return the name of the section `kind`. The removed section names the old version.
fn section_name(t: Texts<'_>, kind: DiffKind, old: Option<u32>) -> String {
    let key = format!("diff-section-{}", kind.as_str());
    match kind {
        DiffKind::Removed => t.with_arg(
            &key,
            "old",
            old.map_or_else(String::new, |number| number.to_string()),
        ),
        _other => t.get(&key),
    }
}

/// One item of the summary list (`design-system.md` section 10.13).
pub(crate) struct SummaryItem {
    /// The state of the section.
    pub(crate) state: DiffState,
    /// The name of the section. It is the text of the link.
    pub(crate) name: String,
    /// The anchor of the section on this page, or the address of its first page.
    pub(crate) href: String,
    /// The count of the parameters of the section.
    pub(crate) parameters: String,
    /// The count of the rows. Empty for a section that counts no rows.
    pub(crate) rows: String,
}

/// One link to the pages of a section that the bound of the page cuts (ADR-0050 (e)).
pub(crate) struct CutLink {
    /// The address of the first page of the section.
    pub(crate) href: String,
    /// The text `diff-pager-label` with the name of the section.
    pub(crate) label: String,
}

/// The pager of one section (`design-system.md` section 10.12).
pub(crate) struct PagerView {
    /// The accessible name `diff-pager-label` with the name of the section.
    pub(crate) label: String,
    /// The address of the previous page. Empty on the first page.
    pub(crate) previous: String,
    /// The address of the next page. Empty on the last page.
    pub(crate) next: String,
    /// The number of the current page, from 1.
    pub(crate) current: u32,
}

/// The notice of a page that shows no section, with the link to the first page.
///
/// The type solves one problem: two pages show a notice in place of the sections. These are a page
/// after the last page and the first page of a section without an entry. Both end in a link to the
/// first page, so the template needs one shape.
pub(crate) struct PageNotice {
    /// The text of the notice: `list-page-after-end` or `diff-kind-empty`.
    pub(crate) text: String,
    /// The address of the page that the link `list-first-page` opens.
    pub(crate) first: String,
}

/// One header cell of a section table.
pub(crate) struct HeadCell {
    /// The text of the column.
    pub(crate) text: String,
    /// `true` for a column of values or limits, which aligns to the right.
    pub(crate) value: bool,
}

/// One cell of a section table.
pub(crate) struct Cell {
    /// The class of the cell, for example `data` or `diff-value`. Empty for no class.
    pub(crate) class: &'static str,
    /// The text of the cell.
    pub(crate) text: String,
    /// The language tag of a content text: the tag of its translation row, or `en` for `name_en`
    /// and `description_en`. Empty for a UI text and for a text without a language.
    pub(crate) lang: String,
    /// The address of the link of the cell. Empty for no link.
    pub(crate) href: String,
    /// `true` when the side is empty and the text is the word `diff-none`.
    pub(crate) none: bool,
    /// The labeled texts of a side with more than one field, such as a translation. The template
    /// shows them as a description list in place of `text`.
    pub(crate) pairs: Vec<Pair>,
}

/// One labeled text of a cell, for example the name of a translation.
pub(crate) struct Pair {
    /// The label, for example "Name".
    pub(crate) label: String,
    /// The text, or the word `diff-none` for an empty field.
    pub(crate) text: String,
    /// The language tag of the text. Empty for the word `diff-none`, which is in the UI language.
    pub(crate) lang: String,
    /// `true` when the field is empty and the text is the word `diff-none`.
    pub(crate) none: bool,
}

/// A link with its text.
pub(crate) struct LinkView {
    /// The address of the link.
    pub(crate) href: String,
    /// The text of the link.
    pub(crate) text: String,
}

/// The row of the rows that a section does not show (ADR-0050 (e)).
pub(crate) struct MoreRows {
    /// The text `diff-more-rows` with the count.
    pub(crate) text: String,
    /// The links to the parameter page in each version.
    pub(crate) links: Vec<LinkView>,
}

/// One row of a section table. The template adds the state cell to a row with cells.
pub(crate) struct RowView {
    /// The cells of the row.
    pub(crate) cells: Vec<Cell>,
    /// The count of the rows that the section does not show, or `None`.
    pub(crate) more: Option<MoreRows>,
}

/// One section with its table (`design-system.md` section 10.13).
pub(crate) struct SectionView {
    /// The `id` of the `section` element, for example `diff-limits`.
    pub(crate) id: String,
    /// The `id` of the caption, for example `diff-limits-caption`.
    pub(crate) caption_id: String,
    /// The class of the frame. The limits frame has the border of `--color-error-edge`.
    pub(crate) frame: &'static str,
    /// The state of the section.
    pub(crate) state: DiffState,
    /// The name of the section.
    pub(crate) name: String,
    /// The caption `diff-caption` with the name and the count.
    pub(crate) caption: String,
    /// The header cells.
    pub(crate) head: Vec<HeadCell>,
    /// The rows.
    pub(crate) rows: Vec<RowView>,
}

/// The view of one diff page: the summary, the sections, and the links to the other pages.
pub(crate) struct DiffView {
    /// The summary items of the sections with a count.
    pub(crate) summary: Vec<SummaryItem>,
    /// The text `diff-unchanged` with the count.
    pub(crate) unchanged: String,
    /// The sections of this page.
    pub(crate) sections: Vec<SectionView>,
    /// The links of the sections that the bound of the page cuts. A page with `kind` has none.
    pub(crate) cut: Vec<CutLink>,
    /// The pager of the requested section, or `None` without `kind`, under a notice, and under the
    /// text `diff-empty`.
    pub(crate) pager: Option<PagerView>,
    /// The notice of a page without a section, or `None`.
    pub(crate) notice: Option<PageNotice>,
    /// The text `diff-empty` when the two versions have the same parameters, or `None`.
    pub(crate) empty: Option<String>,
}

/// Return `true` for a page after the last page: it has a number above 1 and shows no section.
fn is_after_end(page: &DiffPage) -> bool {
    page.sections.is_empty() && page.page > 0
}

/// Return the text `diff-empty` when the diff has an old side and no change in any section.
///
/// A page after the last page has no section too, so the function returns `None` for it. A diff
/// without an old side has no old number for the text, so the function returns `None` for it as
/// well.
fn empty_text(t: Texts<'_>, page: &DiffPage, links: &DiffLinks<'_>) -> Option<String> {
    let unchanged = DiffKind::ALL
        .into_iter()
        .all(|kind| page.summary.parameters(kind) == 0);
    let old = links.old.filter(|_old| unchanged && !is_after_end(page))?;
    let mut args = fluent::FluentArgs::new();
    args.set("new", links.new.to_string());
    args.set("old", old.to_string());
    Some(t.with("diff-empty", &args))
}

/// Return the notice of a page that shows no section, or `None` for any other page.
///
/// A page after the last page gets `list-page-after-end` and a link to the first page of its
/// section or of the whole diff. The first page with `kind` of a section without an entry gets
/// `diff-kind-empty` and a link to the first page of the whole diff. A diff with the text
/// `diff-empty` gets no notice, because that text already says that nothing changed (design D4).
fn page_notice(
    t: Texts<'_>,
    page: &DiffPage,
    links: &DiffLinks<'_>,
    empty_shown: bool,
) -> Option<PageNotice> {
    if is_after_end(page) {
        return Some(PageNotice {
            text: t.with_arg("list-page-after-end", "page", page.page.saturating_add(1)),
            first: page
                .kind
                .map_or_else(|| links.first_url(), |kind| links.section_url(kind)),
        });
    }
    let kind = page
        .kind
        .filter(|_kind| page.sections.is_empty() && !empty_shown)?;
    Some(PageNotice {
        text: t.with_arg(
            "diff-kind-empty",
            "section",
            section_name(t, kind, links.old),
        ),
        first: links.first_url(),
    })
}

impl DiffView {
    /// Build the view of one diff page.
    pub(crate) fn new(t: Texts<'_>, page: &DiffPage, links: &DiffLinks<'_>) -> Self {
        let rows = RowContext { t, page, links };
        let empty = empty_text(t, page, links);
        let notice = page_notice(t, page, links, empty.is_some());
        // A diff without any change shows `diff-empty` and no pager, with or without `kind`
        // (design D4).
        let has_pager = notice.is_none() && empty.is_none();
        Self {
            empty,
            summary: summary_items(t, page, links),
            unchanged: t.with_arg("diff-unchanged", "count", page.summary.unchanged),
            sections: page
                .sections
                .iter()
                .map(|section| section_view(&rows, section))
                .collect(),
            // A page with `kind` shows no cut links, because its pager leads to the other pages
            // of the section (design D4).
            cut: page
                .cut
                .iter()
                .filter(|_kind| page.kind.is_none())
                .map(|kind| CutLink {
                    href: links.section_url(*kind),
                    label: pager_label(t, *kind, links.old),
                })
                .collect(),
            pager: page
                .kind
                .filter(|_kind| has_pager)
                .map(|kind| pager_view(t, page, links, kind)),
            notice,
        }
    }
}

/// Return the text `diff-pager-label` with the name of the section `kind`.
fn pager_label(t: Texts<'_>, kind: DiffKind, old: Option<u32>) -> String {
    t.with_arg("diff-pager-label", "section", section_name(t, kind, old))
}

/// Return the pager of the section `kind`.
fn pager_view(t: Texts<'_>, page: &DiffPage, links: &DiffLinks<'_>, kind: DiffKind) -> PagerView {
    let current = page.page.saturating_add(1);
    let previous = if page.page > 0 {
        links.page_of(kind, page.page)
    } else {
        String::new()
    };
    let next = page
        .has_next
        .then(|| current.checked_add(1))
        .flatten()
        .map(|number| links.page_of(kind, number))
        .unwrap_or_default();
    PagerView {
        label: pager_label(t, kind, links.old),
        previous,
        next,
        current,
    }
}

/// Return the summary items of the sections with a count, in the order of [`DiffKind::ALL`].
fn summary_items(t: Texts<'_>, page: &DiffPage, links: &DiffLinks<'_>) -> Vec<SummaryItem> {
    let summary = &page.summary;
    DiffKind::ALL
        .into_iter()
        .filter(|kind| summary.parameters(*kind) > 0)
        .map(|kind| {
            let shown = page.sections.iter().any(|section| section.kind == kind);
            let rows = match kind {
                DiffKind::Limits => Some(summary.limit_rows),
                DiffKind::Definitions => Some(summary.definition_rows),
                DiffKind::Translations => Some(summary.translation_rows),
                _other => None,
            };
            SummaryItem {
                state: state_of(t, kind),
                name: section_name(t, kind, links.old),
                href: if shown {
                    format!("#diff-{}", kind.as_str())
                } else {
                    links.section_url(kind)
                },
                parameters: t.with_arg("diff-count-parameters", "count", summary.parameters(kind)),
                rows: rows
                    .map(|count| t.with_arg("diff-count-rows", "count", count))
                    .unwrap_or_default(),
            }
        })
        .collect()
}

/// What the rows of a section need: the texts, the page, and the addresses.
struct RowContext<'r, 'a> {
    t: Texts<'r>,
    page: &'r DiffPage,
    links: &'r DiffLinks<'a>,
}

/// Return the header cells of the section `kind`.
fn head_of(t: Texts<'_>, kind: DiffKind, links: &DiffLinks<'_>) -> Vec<HeadCell> {
    let old = links.old.map_or_else(
        || t.get("diff-none"),
        |number| t.with_arg("diff-col-version", "number", number),
    );
    let new = t.with_arg("diff-col-version", "number", links.new);
    let limit = t.with_arg("diff-col-limit", "number", links.new);
    let columns: Vec<(String, bool)> = match kind {
        DiffKind::Limits => vec![(t.get("diff-col-path"), false), (old, true), (new, true)],
        DiffKind::Values => vec![
            (t.get("diff-col-path"), false),
            (old, true),
            (new, true),
            (limit, true),
        ],
        DiffKind::Definitions => vec![(t.get("diff-col-field"), false), (old, false), (new, false)],
        DiffKind::Translations => vec![(t.get("diff-col-lang"), false), (old, false), (new, false)],
        DiffKind::Added | DiffKind::Removed => vec![
            (t.get("diff-col-name"), false),
            (t.get("diff-col-unit"), false),
            (t.get("diff-col-type"), false),
            (t.get("diff-col-value"), true),
        ],
    };
    std::iter::once((t.get("diff-col-parameter"), false))
        .chain(columns)
        .chain(std::iter::once((t.get("diff-col-state"), false)))
        .map(|(text, value)| HeadCell { text, value })
        .collect()
}

/// Return the view of one section of the page.
fn section_view(ctx: &RowContext<'_, '_>, section: &DiffSection) -> SectionView {
    let t = ctx.t;
    let kind = section.kind;
    let name = section_name(t, kind, ctx.links.old);
    let id = format!("diff-{}", kind.as_str());
    let mut caption_args = fluent::FluentArgs::new();
    caption_args.set("section", name.clone());
    caption_args.set("count", ctx.page.summary.parameters(kind).to_string());
    SectionView {
        caption_id: format!("{id}-caption"),
        id,
        frame: if kind == DiffKind::Limits {
            "table-wrap diff-limits"
        } else {
            "table-wrap"
        },
        state: state_of(t, kind),
        caption: t.with("diff-caption", &caption_args),
        name,
        head: head_of(t, kind, ctx.links),
        rows: section_rows(ctx, section),
    }
}

/// Return the rows of a section. Only the first row of a parameter links its tag.
///
/// The link opens the page of the parameter in a version that holds it (see [`link_number`]).
fn section_rows(ctx: &RowContext<'_, '_>, section: &DiffSection) -> Vec<RowView> {
    let mut linked: Option<ParameterKey> = None;
    section
        .rows
        .iter()
        .map(|row| match row.part() {
            DiffPart::Omitted { count } => more_row(ctx, row, *count),
            part => {
                let first = linked.replace(row.key()) != Some(row.key());
                let href = if first {
                    let number = link_number(row, ctx.links);
                    ctx.links.parameter_url(number, row.key())
                } else {
                    String::new()
                };
                let tag = Cell {
                    href,
                    ..text_cell("data", row.tag())
                };
                RowView {
                    cells: std::iter::once(tag).chain(part_cells(ctx, part)).collect(),
                    more: None,
                }
            }
        })
        .collect()
}

/// Return the version whose parameter page a row links to.
///
/// The presence of the parameter decides, not the part of the row. A limit row that only the old
/// side holds can come first (ADR-0050 (d)). So the link opens the old version only when the new
/// version does not hold the parameter.
fn link_number(row: &DiffRow, links: &DiffLinks<'_>) -> u32 {
    match links.old {
        Some(old) if !row.is_in_new_version() => old,
        _new => links.new,
    }
}

/// Return the row of the rows that a section does not show, with the links to both versions.
fn more_row(ctx: &RowContext<'_, '_>, row: &DiffRow, count: usize) -> RowView {
    let t = ctx.t;
    let links = ctx
        .links
        .old
        .into_iter()
        .chain(std::iter::once(ctx.links.new))
        .map(|number| LinkView {
            href: ctx.links.parameter_url(number, row.key()),
            text: t.with_arg("diff-open-version", "number", number),
        })
        .collect();
    RowView {
        cells: Vec::new(),
        more: Some(MoreRows {
            text: t.with_arg("diff-more-rows", "count", count),
            links,
        }),
    }
}

/// Return a cell with a text and a class.
fn text_cell(class: &'static str, text: impl ToString) -> Cell {
    Cell {
        class,
        text: text.to_string(),
        lang: String::new(),
        href: String::new(),
        none: false,
        pairs: Vec::new(),
    }
}

/// Return `cell` with the language tag `lang` on its text.
///
/// The tag comes from a validated language code or from `Lang::En`, never from free text.
fn in_language(lang: &str, cell: Cell) -> Cell {
    Cell {
        lang: lang.to_owned(),
        ..cell
    }
}

/// Return the cell of one side: its text, or the word `diff-none` for an empty side.
fn side_cell(t: Texts<'_>, class: &'static str, text: Option<String>) -> Cell {
    text.map_or_else(
        || Cell {
            none: true,
            ..text_cell(class, t.get("diff-none"))
        },
        |text| text_cell(class, text),
    )
}

/// Return the cells of the one row of a part, without the tag cell (design D4).
///
/// An omitted part has no cells. `section_rows` shows it as the row of the rows that it omits.
fn part_cells(ctx: &RowContext<'_, '_>, part: &DiffPart) -> Vec<Cell> {
    let t = ctx.t;
    match part {
        DiffPart::Limit { path, old, new } => vec![
            text_cell("data", path),
            limit_cell(t, old.as_ref()),
            limit_cell(t, new.as_ref()),
        ],
        DiffPart::Value {
            path,
            old,
            new,
            limit,
        } => vec![
            text_cell("data", path),
            side_cell(t, "diff-value", old.clone()),
            side_cell(t, "diff-value", new.clone()),
            limit_cell(t, limit.as_ref()),
        ],
        DiffPart::Definition(change) => definition_cells(ctx, change),
        DiffPart::Translation { lang, old, new } => vec![
            text_cell("data", lang),
            translation_cell(t, old.as_ref()),
            translation_cell(t, new.as_ref()),
        ],
        DiffPart::Parameter {
            name_en,
            unit,
            plc_type,
            scalar,
            leaves,
        } => vec![
            in_language(Lang::En.as_str(), text_cell("", name_en)),
            text_cell("data", unit_text(&ctx.page.unit_paths, *unit)),
            text_cell("data", type_text(&ctx.page.type_texts, *plc_type)),
            text_cell(
                "diff-value",
                scalar
                    .clone()
                    .unwrap_or_else(|| t.with_arg("diff-leaf-count", "count", leaves)),
            ),
        ],
        DiffPart::Omitted { .. } => Vec::new(),
    }
}

/// Return the cell of one limit as one text: the range, one bound, or the maximum length.
///
/// A limit row of the limits section shows each side with this cell, so each bound of both sides
/// shows, also after a change of the type (ADR-0050 (c), design D4). A value row shows the limit of
/// the new version that covers its leaf.
fn limit_cell(t: Texts<'_>, limit: Option<&DiffBounds>) -> Cell {
    let text = limit.and_then(
        |bounds| match (&bounds.min, &bounds.max, &bounds.max_length) {
            (_min, _max, Some(length)) => {
                Some(t.with_arg("diff-limit-max-length", "max_length", length))
            }
            (Some(min), Some(max), None) => {
                let mut args = fluent::FluentArgs::new();
                args.set("min", min.clone());
                args.set("max", max.clone());
                Some(t.with("diff-limit-range", &args))
            }
            (Some(min), None, None) => Some(t.with_arg("diff-limit-min-only", "min", min)),
            (None, Some(max), None) => Some(t.with_arg("diff-limit-max-only", "max", max)),
            (None, None, None) => None,
        },
    );
    side_cell(t, "diff-value", text)
}

/// Return the path of a unit, or its identifier when the page holds no path.
fn unit_text(paths: &BTreeMap<UnitId, UnitPath>, unit: UnitId) -> String {
    paths
        .get(&unit)
        .map_or_else(|| unit.to_string(), ToString::to_string)
}

/// Return the text of a type, or its identifier when the page holds no text.
fn type_text(texts: &BTreeMap<PlcTypeId, String>, plc_type: PlcTypeId) -> String {
    texts
        .get(&plc_type)
        .cloned()
        .unwrap_or_else(|| plc_type.to_string())
}

/// Return the cells of one changed field of a definition, without the tag cell.
fn definition_cells(ctx: &RowContext<'_, '_>, change: &DefinitionChange) -> Vec<Cell> {
    let page = ctx.page;
    let (key, class, old, new) = match change {
        DefinitionChange::Tag { old, new } => {
            ("diff-field-tag", "data", old.to_string(), new.to_string())
        }
        DefinitionChange::Unit { old, new } => (
            "diff-field-unit",
            "data",
            unit_text(&page.unit_paths, *old),
            unit_text(&page.unit_paths, *new),
        ),
        DefinitionChange::Type { old, new } => (
            "diff-field-type",
            "data",
            type_text(&page.type_texts, *old),
            type_text(&page.type_texts, *new),
        ),
        DefinitionChange::S7Address { old, new } => {
            let t = ctx.t;
            return vec![
                text_cell("", t.get("diff-field-s7")),
                side_cell(t, "data", old.as_ref().map(ToString::to_string)),
                side_cell(t, "data", new.as_ref().map(ToString::to_string)),
            ];
        }
        DefinitionChange::Ordinal { old, new } => (
            "diff-field-ordinal",
            "num",
            old.value().to_string(),
            new.value().to_string(),
        ),
        DefinitionChange::NameEn { old, new } => {
            ("diff-field-name-en", "", old.to_string(), new.to_string())
        }
        DefinitionChange::DescriptionEn { old, new } => (
            "diff-field-description-en",
            "",
            old.to_string(),
            new.to_string(),
        ),
    };
    let lang = text_language(change);
    vec![
        text_cell("", ctx.t.get(key)),
        in_language(lang, text_cell(class, old)),
        in_language(lang, text_cell(class, new)),
    ]
}

/// Return the language tag of the texts of a changed definition field.
///
/// Only the English name and the English description are text of a language. Every other field is
/// a tag, a path, a type, an address, or a number, which has no language.
fn text_language(change: &DefinitionChange) -> &'static str {
    match change {
        DefinitionChange::NameEn { .. } | DefinitionChange::DescriptionEn { .. } => {
            Lang::En.as_str()
        }
        _other => "",
    }
}

/// Return the cell of one side of a translation row: the name and the description (design D4).
///
/// A side without the translation shows the word `diff-none`. An empty description shows the
/// same word under its label.
fn translation_cell(t: Texts<'_>, side: Option<&Translation>) -> Cell {
    let Some(translation) = side else {
        return side_cell(t, "", None);
    };
    let lang = translation.lang.as_str();
    let description = translation.description.as_ref().map(ToString::to_string);
    let pairs = vec![
        Pair {
            label: t.get("diff-field-tr-name"),
            text: translation.name.to_string(),
            lang: lang.to_owned(),
            none: false,
        },
        Pair {
            label: t.get("diff-field-tr-description"),
            none: description.is_none(),
            lang: description
                .as_ref()
                .map_or_else(String::new, |_text| lang.to_owned()),
            text: description.unwrap_or_else(|| t.get("diff-none")),
        },
    ];
    Cell {
        pairs,
        ..text_cell("", "")
    }
}
