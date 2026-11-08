//! The cell builders of the diff rows.
//!
//! The module solves one problem: every part of a diff row needs the same cells. It builds the
//! text cell, the side cell, the name cell, the limit cell, and the translation cell. `diff_view`
//! keeps the page, the sections, and the rows.

use std::collections::BTreeMap;

use recipe_domain::plant::{PlcTypeId, UnitId, UnitPath};
use recipe_domain::recipe::{DefinitionChange, DiffBounds, DiffPart, LangCode, Translation};
use recipe_domain::text::Text;

use super::{Cell, Pair, RowContext};
use crate::i18n::Lang;
use crate::pages::texts::Texts;
use crate::pages::versions::parameter_table::shown_name;

/// Return a cell with a text and a class.
pub(super) fn text_cell(class: &'static str, text: impl ToString) -> Cell {
    Cell {
        class,
        text: text.to_string(),
        lang: String::new(),
        marker: None,
        href: String::new(),
        none: false,
        pairs: Vec::new(),
        lines: Vec::new(),
        lines_id: String::new(),
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
pub(super) fn part_cells(ctx: &RowContext<'_, '_>, part: &DiffPart) -> Vec<Cell> {
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
            translated_names,
            unit,
            plc_type,
            scalar,
            leaves,
        } => vec![
            name_cell(t, ctx.translation, translated_names, name_en),
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

/// Return the name cell of an added or a removed parameter (ADR-0152 (e) to (g)).
///
/// The problem: this cell must follow the one rule of the shown name, as the parameter table does.
/// A translation of the translation language shows without a marker. Without one, the UI language
/// `pl` shows `name_en` in a `lang="en"` span with the marker outside it. The UI language `en`
/// shows `name_en` as plain text, so the `td` carries no `lang`.
fn name_cell(
    t: Texts<'_>,
    translation: Option<&LangCode>,
    translated_names: &BTreeMap<LangCode, Text>,
    name_en: &Text,
) -> Cell {
    let translated = translation.and_then(|code| translated_names.get(code));
    let shown = shown_name(t, translated, name_en);
    Cell {
        marker: shown.marker,
        ..text_cell("", shown.text)
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
            "text-lines",
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
            lines: false,
            none: false,
        },
        Pair {
            label: t.get("diff-field-tr-description"),
            lines: description.is_some(),
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
