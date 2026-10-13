//! The part `parameters[]` of the hashed body (`api.md` sections 6.1 and 6.3).

use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;

use super::types::type_ref;
use crate::plant::{TypeNode, UnitPath};
use crate::recipe::{Parameter, StoredLimit};

/// One limit of a parameter. A bound is canonical decimal text, or `None`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct LimitEntry {
    leaf_path: String,
    min: Option<String>,
    max: Option<String>,
    max_length: Option<String>,
}

/// One entry of `parameters[]`.
///
/// The type solves one problem: the hashed body holds the fields marked "In hash" and nothing
/// else. The entry has no name and no ordinal, so a rename cannot change the hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct ParameterEntry {
    unit_path: String,
    tag: String,
    key: String,
    type_id: String,
    s7_address: Option<String>,
    limits: Vec<LimitEntry>,
    value: Value,
}

impl ParameterEntry {
    /// Build the entry of a parameter.
    ///
    /// `unit` is the path of the unit of the parameter. `root` is the root node of its type.
    pub(super) fn of(parameter: &Parameter, unit: &UnitPath, root: &TypeNode) -> Self {
        Self {
            unit_path: unit.to_string(),
            tag: parameter.tag().as_str().to_owned(),
            key: parameter.key().to_string(),
            type_id: type_ref(root),
            s7_address: parameter.s7_address().map(|address| address.to_string()),
            limits: limit_entries(parameter.limits()),
            value: numbers_to_text(parameter.value().as_json()),
        }
    }
}

/// Write a bound as canonical decimal text.
fn decimal_text(bound: Decimal) -> String {
    bound.normalize().to_string()
}

/// Build the limit entries ordered by `leaf_path`, compared as text byte by byte.
fn limit_entries(limits: &[StoredLimit]) -> Vec<LimitEntry> {
    let mut entries: Vec<LimitEntry> = limits
        .iter()
        .map(|limit| LimitEntry {
            leaf_path: limit.leaf_path.to_string(),
            min: limit.min.map(decimal_text),
            max: limit.max.map(decimal_text),
            max_length: limit.max_length.map(|length| length.to_string()),
        })
        .collect();
    entries.sort_by(|a, b| a.leaf_path.cmp(&b.leaf_path));
    entries
}

/// Return the value with every numeric leaf as a JSON string.
///
/// The stored number keeps its canonical decimal text (`reliability.md` section 4.4). A boolean
/// and a string stay as they are. The walk uses an explicit stack.
fn numbers_to_text(value: &Value) -> Value {
    let mut copy = value.clone();
    let mut stack = vec![&mut copy];
    while let Some(slot) = stack.pop() {
        match slot {
            Value::Number(number) => {
                let text = number.to_string();
                *slot = Value::String(text);
            }
            Value::Array(items) => stack.extend(items.iter_mut()),
            Value::Object(members) => stack.extend(members.values_mut()),
            Value::Null | Value::Bool(_) | Value::String(_) => {}
        }
    }
    copy
}

#[cfg(test)]
mod tests;
