//! Comparison of row images across the adapters (design D5 and D13).
//!
//! A case checks the key set, the JSON type of each key, and the content of the values that the
//! case knows. It never compares a timestamp or a generated identifier across adapters.

use std::collections::BTreeMap;

use recipe_app::ports::RowImage;
use serde_json::Value;

/// The JSON type of a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    String,
    Number,
    Bool,
    Null,
    Array,
}

fn kind_of(value: &Value) -> String {
    match value {
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
    .to_owned()
}

fn kind_name(kind: Kind) -> String {
    match kind {
        Kind::String => "string",
        Kind::Number => "number",
        Kind::Bool => "bool",
        Kind::Null => "null",
        Kind::Array => "array",
    }
    .to_owned()
}

/// Assert that the image has exactly the keys of `expected`, each with the JSON type listed.
pub fn assert_shape(image: &RowImage, expected: &[(&str, Kind)]) {
    let actual: BTreeMap<String, String> = image
        .as_value()
        .as_object()
        .expect("the image is a JSON object")
        .iter()
        .map(|(key, value)| (key.clone(), kind_of(value)))
        .collect();
    let wanted: BTreeMap<String, String> = expected
        .iter()
        .map(|(key, kind)| ((*key).to_owned(), kind_name(*kind)))
        .collect();
    assert_eq!(actual, wanted, "{image:?}");
}

/// Return the value of a key. The test fails when the key is missing.
pub fn field<'a>(image: &'a RowImage, key: &str) -> &'a Value {
    image
        .as_value()
        .get(key)
        .ok_or_else(|| format!("the image has no key {key}: {image:?}"))
        .unwrap()
}

/// Return the text of a key. The test fails when the value is not a string.
pub fn text<'a>(image: &'a RowImage, key: &str) -> &'a str {
    field(image, key).as_str().expect("the value is a string")
}

/// Return the image without the keys that hold a time of the adapter.
pub fn without_times(image: &RowImage) -> Value {
    let mut value = image.as_value().clone();
    if let Some(object) = value.as_object_mut() {
        object.remove("created_at");
        object.remove("updated_at");
    }
    value
}

/// The shape of a `machine` image (`data-model.md` section 3.1).
pub fn machine_shape(archived: bool) -> Vec<(&'static str, Kind)> {
    vec![
        ("id", Kind::String),
        ("code", Kind::String),
        ("name", Kind::String),
        ("plc_platform", Kind::String),
        (
            "archived_at",
            if archived { Kind::String } else { Kind::Null },
        ),
        ("created_at", Kind::String),
        ("updated_at", Kind::String),
    ]
}

/// The shape of a `recipe` image (`data-model.md` section 3.9).
pub fn recipe_shape(deleted: bool) -> Vec<(&'static str, Kind)> {
    vec![
        ("id", Kind::String),
        ("machine_id", Kind::String),
        ("plc_platform", Kind::String),
        ("reference_number", Kind::String),
        ("name", Kind::String),
        (
            "deleted_at",
            if deleted { Kind::String } else { Kind::Null },
        ),
        ("created_by", Kind::String),
        ("created_at", Kind::String),
        ("updated_at", Kind::String),
    ]
}

/// The state of a `recipe_version` image for `version_shape`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// A live draft.
    Draft,
    /// A discarded draft.
    Discarded,
    /// An approved version.
    Approved,
    /// A retired version.
    Retired,
}

fn nullable(is_set: bool) -> Kind {
    if is_set { Kind::String } else { Kind::Null }
}

/// The shape of a `recipe_version` image (`data-model.md` section 3.10).
pub fn version_shape(based_on: bool, stage: Stage) -> Vec<(&'static str, Kind)> {
    let signed = matches!(stage, Stage::Approved | Stage::Retired);
    vec![
        ("id", Kind::String),
        ("recipe_id", Kind::String),
        ("plc_platform", Kind::String),
        ("number", Kind::Number),
        ("status", Kind::String),
        ("row_version", Kind::Number),
        ("based_on_version_id", nullable(based_on)),
        ("change_note", Kind::String),
        ("content_sha256", nullable(signed)),
        ("created_by", Kind::String),
        ("created_at", Kind::String),
        ("updated_at", Kind::String),
        ("approved_by", nullable(signed)),
        ("approved_at", nullable(signed)),
        ("retired_at", nullable(stage == Stage::Retired)),
        ("deleted_at", nullable(stage == Stage::Discarded)),
    ]
}

/// The shape of an `app_user` image without `password_hash` (`data-model.md` section 3.5).
pub fn user_shape(deactivated: bool) -> Vec<(&'static str, Kind)> {
    vec![
        ("id", Kind::String),
        ("login", Kind::String),
        ("role", Kind::String),
        ("is_active", Kind::Bool),
        ("session_epoch", Kind::Number),
        ("must_change_password", Kind::Bool),
        ("deactivated_at", nullable(deactivated)),
        ("created_at", Kind::String),
        ("updated_at", Kind::String),
    ]
}

/// The keys whose values differ between two images, without the time of the last change.
///
/// The trigger `app_user_set_updated_at` sets `updated_at` to the time of the transaction, and
/// the adapters differ in the first value of that time. Every other key counts.
pub fn changed_keys(before: &RowImage, after: &RowImage) -> Vec<String> {
    let (Some(old), Some(new)) = (before.as_value().as_object(), after.as_value().as_object())
    else {
        return vec!["not an object".to_owned()];
    };
    let mut keys: Vec<String> = new
        .iter()
        .filter(|(key, value)| key.as_str() != "updated_at" && old.get(*key) != Some(*value))
        .map(|(key, _)| key.clone())
        .collect();
    keys.sort();
    keys
}

/// The shape of a `unit` image (`data-model.md` section 3.2).
pub fn unit_shape(has_parent: bool, archived: bool) -> Vec<(&'static str, Kind)> {
    vec![
        ("id", Kind::String),
        ("machine_id", Kind::String),
        ("path", Kind::String),
        (
            "parent_path",
            if has_parent { Kind::String } else { Kind::Null },
        ),
        ("depth", Kind::Number),
        ("name", Kind::String),
        (
            "archived_at",
            if archived { Kind::String } else { Kind::Null },
        ),
        ("created_at", Kind::String),
        ("updated_at", Kind::String),
    ]
}
