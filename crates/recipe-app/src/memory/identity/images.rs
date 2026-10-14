//! Row images of `app_user` (design D5, ADR-0029). The image never holds `password_hash`.

use crate::memory::images::{object, optional_time, time_value};
use crate::ports::{RowImage, UserRow};
use serde_json::Value;

/// Return the image of an `app_user` row without the column `password_hash`.
pub(super) fn user_image(row: &UserRow) -> RowImage {
    RowImage::new(object(vec![
        ("id", Value::String(row.id.to_string())),
        ("login", Value::String(row.login.to_string())),
        ("role", Value::String(row.role.as_sql().to_owned())),
        ("is_active", Value::Bool(row.is_active)),
        ("session_epoch", Value::from(row.session_epoch)),
        (
            "must_change_password",
            Value::Bool(row.must_change_password),
        ),
        ("deactivated_at", optional_time(row.deactivated_at)),
        ("created_at", time_value(row.created_at)),
        ("updated_at", time_value(row.updated_at)),
    ]))
}
