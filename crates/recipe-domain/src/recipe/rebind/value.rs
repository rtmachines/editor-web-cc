//! The value of a parameter after a rebind (design D13, steps 4 and 5, ADR-0018).

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::build::{build_value, visit_elements};
use super::level::{StructLevel, kept_members, struct_level};
use super::map::MapFault;
use super::{MemberValue, RebindRequest};
use crate::plant::{LeafPath, LeafSegment, MemberName, TypeNode, TypeTree, extended};
use crate::recipe::draft::{leaf_error, stored_value};
use crate::recipe::value::{request_leaf, request_text};
use crate::recipe::{
    Expected, ParameterKey, ParameterTag, ParameterValue, RecipeError, StoredValue,
};

/// The inputs that map the value of one parameter.
pub(super) struct ValueMapping<'a> {
    pub(super) key: ParameterKey,
    pub(super) tag: &'a ParameterTag,
    pub(super) request: &'a RebindRequest,
    pub(super) old_tree: &'a TypeTree,
    pub(super) new_tree: &'a TypeTree,
    pub(super) old_value: &'a ParameterValue,
}

/// Map the stored value of a parameter to the planned tree.
///
/// A kept member copies its subtree. A new member is a skeleton of its type. Each new scalar leaf
/// takes its value from the request and passes the current rule set. A removed member disappears.
/// The new value must pass the check against the planned tree. That check reads the kept members
/// under the earlier rule set, because the rebind does not write them (ADR-0120 (d)).
///
/// Only a bug reaches the defect of the tree shape and the defect of the final check. A kept
/// member keeps its shape, and a new leaf passes the current rule set. That set lies inside the
/// earlier one.
///
/// # Errors
///
/// Returns the defect of the stored value when it has not the shape of the tree or fails under
/// the planned tree. The caller parsed the old value under the old tree already. Otherwise it
/// returns every entry of the parameter in leaf order, then one entry per sent path that is no
/// new leaf. A missing value is `E-RCP-028`. A bad value and a value of a path that is no new
/// leaf are `E-RCP-005`. A new number with too many significant digits is `E-RCP-055`.
pub(super) fn map_value(input: &ValueMapping<'_>) -> Result<StoredValue, MapFault> {
    let tag = input.tag;
    let (Some(source), Some(target)) = (struct_level(input.old_tree), struct_level(input.new_tree))
    else {
        return Err(MapFault::Defect(shape_error(tag)));
    };
    let kept = kept_members(&source, &target);
    let mut json = input.old_value.as_json().clone();
    let mut found = Found::default();
    let is_shaped = visit_elements(&mut json, &source.chain, &mut |element, prefix| {
        let mapped = map_element(input, (&target, &kept), element, prefix, &mut found);
        *element = mapped;
    });
    if is_shaped.is_none() {
        return Err(MapFault::Defect(shape_error(tag)));
    }
    let mut entries = found.entries;
    entries.extend(unknown_paths(input, &found.used));
    if !entries.is_empty() {
        return Err(MapFault::Entries(entries));
    }
    stored_value(tag, &StoredValue::from(json), input.new_tree)
        .map(|value| value.to_stored())
        .map_err(MapFault::Defect)
}

/// The result of the walk over the new leaves: the entries and the leaf paths of the new leaves.
#[derive(Default)]
struct Found {
    entries: Vec<RecipeError>,
    used: BTreeSet<LeafPath>,
}

/// Give one `E-RCP-005` entry per sent path of the parameter that is no new leaf.
///
/// The request holds such a value only by a fault, so the plan refuses it and drops nothing.
fn unknown_paths<'r>(
    input: &'r ValueMapping<'_>,
    used: &'r BTreeSet<LeafPath>,
) -> impl Iterator<Item = RecipeError> + 'r {
    input
        .request
        .new_member_values
        .keys()
        .filter(move |(key, leaf)| *key == input.key && !used.contains(leaf))
        .map(|(_, leaf)| RecipeError::ValueShapeMismatch {
            tag: input.tag.clone(),
            leaf: leaf.clone(),
            expected: Expected::KnownLeaf,
        })
}

/// Build the whole-value error for a stored value that has not the shape of the tree.
fn shape_error(tag: &ParameterTag) -> RecipeError {
    RecipeError::ValueShapeMismatch {
        tag: tag.clone(),
        leaf: LeafPath::whole(),
        expected: Expected::ParameterType,
    }
}

/// Map one struct value of the source level to the members of the target.
fn map_element(
    input: &ValueMapping<'_>,
    (target, kept): (&StructLevel<'_>, &BTreeSet<&MemberName>),
    element: &mut Value,
    prefix: &[LeafSegment],
    found: &mut Found,
) -> Value {
    let Value::Object(mut old_members) = std::mem::take(element) else {
        return Value::Null;
    };
    let mut members = Map::new();
    for (name, node) in target.members {
        let value = if kept.contains(name) {
            old_members.remove(name.as_str())
        } else {
            let path = extended(prefix, LeafSegment::Member(name.clone()));
            Some(build_value(node, path, &mut |leaf_path, leaf| {
                new_leaf(input, leaf_path, leaf, found)
            }))
        };
        if let Some(value) = value {
            members.insert(name.as_str().to_owned(), value);
        }
    }
    Value::Object(members)
}

/// Give the value of one new scalar leaf. A missing or bad value adds an entry.
///
/// The class of the new leaf converts a JSON value and a form text with one set of rules. The
/// leaf path goes into `found.used`.
fn new_leaf(
    input: &ValueMapping<'_>,
    path: &[LeafSegment],
    node: &TypeNode,
    found: &mut Found,
) -> Value {
    let leaf = LeafPath::from_segments(path.to_vec());
    found.used.insert(leaf.clone());
    let entries = &mut found.entries;
    let tag = input.tag.clone();
    let Some(given) = input
        .request
        .new_member_values
        .get(&(input.key, leaf.clone()))
    else {
        entries.push(RecipeError::RebindNeedsValue { tag, leaf });
        return Value::Null;
    };
    let machine = (input.new_tree.platform(), input.new_tree.string_encoding());
    let converted = match given {
        MemberValue::Json(value) => request_leaf(value, node, machine),
        MemberValue::Text(text) => request_text(text, node, machine),
    };
    match converted {
        Ok(value) => value,
        Err(fault) => {
            entries.push(leaf_error(&tag, leaf, fault));
            Value::Null
        }
    }
}

#[cfg(test)]
mod tests;
