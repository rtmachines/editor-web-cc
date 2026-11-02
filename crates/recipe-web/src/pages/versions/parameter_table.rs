//! The parameter table of a version: the view of `parameter-table.html` (design D7, ADR-0043 (j)).
//!
//! The version page includes the table. `GET .../parameters` of the parameter area answers
//! `parameter-table.html` alone with a struct that holds `t` and `table`. `AddParameter` answers
//! one row of it with [`ParameterRowFragment`] (wave 5).

use std::collections::BTreeSet;

use askama::Template;
use recipe_app::ports::{ParameterRow, UnitCount};
use recipe_app::recipe::ParameterList;
use recipe_domain::plant::{BaseExpr, PlcPlatform, Primitive, TypeExpr};
use recipe_domain::recipe::ParameterKey;

use crate::pages::path_values::Page;
use crate::pages::recipes::pager_links;
use crate::pages::render::page_url;
use crate::pages::texts::Texts;

/// One row of the parameter table.
pub(crate) struct ParameterRowView {
    /// The address of the parameter page.
    pub(crate) href: String,
    /// The tag of the parameter.
    pub(crate) tag: String,
    /// The English name of the parameter.
    pub(crate) name: String,
    /// The type text, for example `ARRAY[1..8] OF INT`.
    pub(crate) type_text: String,
    /// The cell of the column Value (ADR-0108 (a)).
    pub(crate) value: ValueView,
    /// The path of the unit that holds the parameter.
    pub(crate) unit: String,
    /// The S7 address, or an empty text on a platform without addresses.
    pub(crate) address: String,
    /// True when the type of the parameter uses an older struct revision (ADR-0043 (j)).
    pub(crate) outdated: bool,
}

/// The cell of the column Value: the text of the value or a fixed word.
pub(crate) struct ValueView {
    /// The text of the leaf cut to [`VALUE_CHARS`], or the word of an empty text or a composite.
    pub(crate) text: String,
    /// True when `text` is a fixed word and not a value. The cell then has the muted class.
    pub(crate) is_word: bool,
}

impl ValueView {
    /// Build the cell from the leaf text of a primitive parameter, or `None` for an array or a
    /// struct (ADR-0108 (a)).
    ///
    /// An empty text shows `parameters-value-empty`. `None` shows `parameters-value-composite`.
    /// A BOOL value shows the word of `grid-true` or `grid-false`, as the leaf grid does
    /// (ADR-0039). `type_text` is the type text of the parameter, for example `BOOL`.
    pub(crate) fn new(t: Texts<'_>, type_text: &str, leaf_text: Option<&str>) -> Self {
        match leaf_text {
            None => Self {
                text: t.get("parameters-value-composite"),
                is_word: true,
            },
            Some("") => Self {
                text: t.get("parameters-value-empty"),
                is_word: true,
            },
            Some(text) => Self {
                text: bool_word(t, type_text, text).unwrap_or_else(|| cut_value(text)),
                is_word: false,
            },
        }
    }
}

/// Return the word of the leaf grid for a BOOL value, or `None` for any other type or text.
///
/// The grid shows `TRUE` and `FALSE` for a BOOL leaf. A STRING with the text `true` keeps its text.
fn bool_word(t: Texts<'_>, type_text: &str, leaf_text: &str) -> Option<String> {
    let is_bool = TypeExpr::parse(type_text).is_ok_and(|expr| {
        expr.arrays().is_empty() && matches!(expr.base(), BaseExpr::Primitive(Primitive::Bool))
    });
    if !is_bool {
        return None;
    }
    match leaf_text {
        "true" => Some(t.get("grid-true")),
        "false" => Some(t.get("grid-false")),
        _ => None,
    }
}

/// Cut `text` to [`VALUE_CHARS`] characters and end a longer text with `…`.
///
/// The cut counts `char` values and never splits a multi-byte character. It uses no string slice.
pub(crate) fn cut_value(text: &str) -> String {
    let mut chars = text.chars();
    let mut kept: String = chars.by_ref().take(VALUE_CHARS).collect();
    if chars.next().is_some() {
        kept.push('…');
    }
    kept
}

/// One link to the parameters of one unit (ADR-0043 (j)).
pub(crate) struct UnitLink {
    /// The address of the version page with the unit.
    pub(crate) href: String,
    /// The unit path and the count of its parameters in the UI language.
    pub(crate) text: String,
    /// True for the unit that the table shows. The link then has `aria-current="page"`.
    pub(crate) current: bool,
}

/// The data of the parameter table: one page of rows, the unit links, and the pager.
pub(crate) struct ParameterTable {
    /// The rows of the page in the order unit path, ordinal, and tag.
    pub(crate) rows: Vec<ParameterRowView>,
    /// True when the platform of the machine has S7 addresses, so the table has their column.
    pub(crate) has_address: bool,
    /// The number of the columns of the table.
    pub(crate) columns: usize,
    /// The links to the units. Empty when the table shows every unit.
    pub(crate) units: Vec<UnitLink>,
    /// The address of the previous page, or an empty text.
    pub(crate) previous_url: String,
    /// The address of the next page, or an empty text.
    pub(crate) next_url: String,
    /// The number of the page in the URL.
    pub(crate) current: u32,
    /// True for a page after the last page.
    pub(crate) after_end: bool,
    /// The address of the first page of the shown unit.
    pub(crate) first_url: String,
}

/// Return true for a platform whose parameters have an S7 address (A31).
pub(crate) const fn has_address(platform: PlcPlatform) -> bool {
    platform.is_s7()
}

/// The columns without the address: tag, name, type, value, unit, and state.
const BASE_COLUMNS: usize = 6;

/// The setting `table.value_chars` of `reliability.md` section 7.11, in characters.
pub(crate) const VALUE_CHARS: usize = 64;

/// Return the count of the columns of the table: the base columns and the address column.
pub(crate) fn column_count(has_address: bool) -> usize {
    BASE_COLUMNS.saturating_add(usize::from(has_address))
}

impl ParameterTable {
    /// Build the table from one page of `ListParameters` and the keys of the outdated parameters.
    ///
    /// `base` is the address of the version page, for example `/recipes/R-100/versions/3`.
    pub(crate) fn new(
        t: Texts<'_>,
        base: &str,
        list: &ParameterList,
        outdated: &BTreeSet<ParameterKey>,
        page: Page,
    ) -> Self {
        let has_address = has_address(list.machine.platform());
        let unit = list.unit.as_ref().map(ToString::to_string);
        let pairs: Vec<(&str, &str)> = unit.iter().map(|path| ("unit", path.as_str())).collect();
        let after_end = list.rows.is_empty() && page.index > 0;
        let (previous_url, next_url) = pager_links(base, (page, list.has_next, after_end), &pairs);
        Self {
            rows: list
                .rows
                .iter()
                .map(|row| row_view(t, base, list, row, outdated))
                .collect(),
            has_address,
            columns: column_count(has_address),
            units: unit
                .as_deref()
                .map(|shown| unit_links(t, base, &list.units, shown))
                .unwrap_or_default(),
            previous_url,
            next_url,
            current: page.number,
            after_end,
            first_url: page_url(base, &pairs, 1),
        }
    }
}

/// Build the view of one row.
fn row_view(
    t: Texts<'_>,
    base: &str,
    list: &ParameterList,
    row: &ParameterRow,
    outdated: &BTreeSet<ParameterKey>,
) -> ParameterRowView {
    let type_text = list
        .type_texts
        .get(&row.plc_type)
        .cloned()
        .unwrap_or_default();
    let leaf_text = list.value_texts.get(&row.key).map(String::as_str);
    ParameterRowView {
        href: format!("{base}/parameters/{}", row.key),
        tag: row.tag.to_string(),
        name: row.name_en.to_string(),
        value: ValueView::new(t, &type_text, leaf_text),
        type_text,
        unit: row.unit_path.to_string(),
        address: row
            .s7_address
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        outdated: outdated.contains(&row.key),
    }
}

/// Build one link per unit with parameters. The link of the shown unit is the current link.
fn unit_links(t: Texts<'_>, base: &str, units: &[UnitCount], shown: &str) -> Vec<UnitLink> {
    units
        .iter()
        .map(|count| {
            let path = count.path.to_string();
            let mut args = fluent::FluentArgs::new();
            args.set("path", path.clone());
            args.set("count", count.count);
            // A unit path holds only `U`, digits, and dots, so it needs no percent-encoding.
            UnitLink {
                href: format!("{base}?unit={path}"),
                text: t.with("parameters-unit-link", &args),
                current: path == shown,
            }
        })
        .collect()
}

/// The fragment `parameter-row.html`: the row of a new parameter and the out-of-band updates.
///
/// The answer of `AddParameter` (wave 5) appends the row to `#parameter-rows`. It also clears
/// `#parameter-empty`, `#parameter-form`, and `#error-panel` and shows the toast (design D7,
/// ADR-0043 (i)).
#[derive(Template)]
#[template(path = "parameter-row.html")]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the answer of AddParameter of wave 5 renders it")
)]
pub(crate) struct ParameterRowFragment<'a> {
    /// The texts of the UI language.
    pub(crate) t: Texts<'a>,
    /// The row of the new parameter. Its tag link gets `autofocus`.
    pub(crate) row: ParameterRowView,
    /// True when the platform has S7 addresses, so the row has the address cell.
    pub(crate) has_address: bool,
    /// The success sentence for `#toast`, for example the text of `parameter-added`.
    pub(crate) message: String,
}
