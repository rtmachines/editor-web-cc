//! Row images of `recipe` and `recipe_version` (design D9, ADR-0029).
//!
//! The keys are the column names of `data-model.md` sections 3.9 and 3.10.

use serde_json::Value;

use super::rows::{RecipeRow, VersionRow};
use crate::memory::images::{object, optional_time, time_value};
use crate::ports::RowImage;

pub(super) fn text(value: impl ToString) -> Value {
    Value::String(value.to_string())
}

pub(super) fn optional_text<T: ToString>(value: Option<T>) -> Value {
    value.map_or(Value::Null, text)
}

/// Return the image of a `recipe` row.
pub(super) fn recipe_image(row: &RecipeRow) -> RowImage {
    RowImage::new(object(vec![
        ("id", text(row.id)),
        ("machine_id", text(row.machine)),
        ("plc_platform", text(row.platform)),
        ("reference_number", text(&row.reference)),
        ("name", text(&row.name)),
        ("deleted_at", optional_time(row.deleted_at)),
        ("created_by", text(row.created_by)),
        ("created_at", time_value(row.created_at)),
        ("updated_at", time_value(row.updated_at)),
    ]))
}

/// Return the image of a `recipe_version` row.
pub(super) fn version_image(row: &VersionRow) -> RowImage {
    RowImage::new(object(vec![
        ("id", text(row.id)),
        ("recipe_id", text(row.recipe)),
        ("plc_platform", text(row.platform)),
        ("number", Value::from(row.number)),
        ("status", text(row.status)),
        ("row_version", Value::from(row.row_version)),
        ("based_on_version_id", optional_text(row.based_on)),
        ("change_note", text(row.change_note.as_str())),
        ("content_sha256", optional_text(row.content_sha256)),
        ("created_by", text(row.created_by)),
        ("created_at", time_value(row.created_at)),
        ("updated_at", time_value(row.updated_at)),
        ("approved_by", optional_text(row.approved_by)),
        ("approved_at", optional_time(row.approved_at)),
        ("retired_at", optional_time(row.retired_at)),
        ("deleted_at", optional_time(row.deleted_at)),
    ]))
}
