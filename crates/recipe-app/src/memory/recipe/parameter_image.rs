//! Row image of `parameter` with its child rows (design D9, ADR-0029).
//!
//! The keys are the column names of `data-model.md` sections 3.12 to 3.14. The image holds
//! the limit rows in `limits` and the translation rows in `translations`.

use serde_json::{Number, Value};

use super::images::{optional_text, text};
use super::rows::{LimitRow, ParameterRow, TranslationRow};
use crate::memory::images::{object, time_value};
use crate::ports::RowImage;

/// Return a `NUMERIC` bound as `to_jsonb` gives it: a JSON number with the digits of the column.
///
/// The function solves one problem: the database image holds a number, and the contract suite
/// compares both adapters. A text that is no JSON number cannot occur, and falls back to text.
fn numeric_value<T: ToString>(bound: Option<T>) -> Value {
    bound.map_or(Value::Null, |bound| {
        let digits = bound.to_string();
        digits
            .parse::<Number>()
            .map_or_else(|_| Value::String(digits), Value::Number)
    })
}

fn limit_row_image(row: &ParameterRow, limit: &LimitRow) -> Value {
    let stored = &limit.limit;
    object(vec![
        ("parameter_id", text(row.id)),
        ("recipe_version_id", text(row.version)),
        ("leaf_path", text(&stored.leaf_path)),
        ("min_value", numeric_value(stored.min)),
        ("max_value", numeric_value(stored.max)),
        (
            "max_length",
            stored.max_length.map_or(Value::Null, Value::from),
        ),
        ("created_at", time_value(limit.created_at)),
    ])
}

/// Return the image of one `parameter_translation` row (`data-model.md` section 3.14).
pub(crate) fn translation_row_image(row: &ParameterRow, translation: &TranslationRow) -> Value {
    let stored = &translation.translation;
    object(vec![
        ("parameter_id", text(row.id)),
        ("recipe_version_id", text(row.version)),
        ("lang", text(&stored.lang)),
        ("name", text(&stored.name)),
        (
            "description",
            text(stored.description.as_ref().map_or("", |d| d.as_str())),
        ),
        ("created_at", time_value(translation.created_at)),
    ])
}

/// Return the image of a `parameter` row with its limit rows and translation rows.
pub(crate) fn parameter_image(row: &ParameterRow) -> RowImage {
    let limits = row
        .limits
        .values()
        .map(|limit| limit_row_image(row, limit))
        .collect();
    let translations = row
        .translations
        .values()
        .map(|translation| translation_row_image(row, translation))
        .collect();
    RowImage::new(object(vec![
        ("id", text(row.id)),
        ("recipe_version_id", text(row.version)),
        ("plc_platform", text(row.platform)),
        ("parameter_key", text(row.key)),
        ("unit_id", text(row.unit)),
        ("tag", text(&row.tag)),
        ("name_en", text(&row.name_en)),
        ("description_en", text(row.description_en.as_str())),
        ("plc_type_id", text(row.plc_type)),
        ("s7_address", optional_text(row.s7_address)),
        ("value", row.value.as_json().clone()),
        ("ordinal", Value::from(row.ordinal.value())),
        ("created_at", time_value(row.created_at)),
        ("limits", Value::Array(limits)),
        ("translations", Value::Array(translations)),
    ]))
}
