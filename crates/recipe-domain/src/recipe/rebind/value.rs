//! The value of a parameter after a rebind (design D13, steps 4 and 5, ADR-0018).

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::build::{build_value, visit_elements};
use super::level::{StructLevel, kept_members, struct_level};
use super::{MemberValue, RebindRequest};
use crate::plant::{LeafPath, LeafSegment, MemberName, TypeNode, TypeTree, extended};
use crate::recipe::draft::stored_value;
use crate::recipe::value::{request_leaf, request_text};
use crate::recipe::{Expected, ParameterKey, ParameterTag, RecipeError, StoredValue};

/// The inputs that map the value of one parameter.
pub(super) struct ValueMapping<'a> {
    pub(super) key: ParameterKey,
    pub(super) tag: &'a ParameterTag,
    pub(super) request: &'a RebindRequest,
    pub(super) old_tree: &'a TypeTree,
    pub(super) new_tree: &'a TypeTree,
    pub(super) old_value: &'a StoredValue,
}

/// Map the stored value of a parameter to the planned tree.
///
/// A kept member copies its subtree. A new member is a skeleton of its type. Each new scalar leaf
/// takes its value from the request. A removed member disappears. The new value must pass the
/// check against the planned tree.
///
/// # Errors
///
/// Returns every entry of the parameter in leaf order, then one entry per sent path that is no
/// new leaf. A missing value is `E-RCP-028`. A bad value, a value of a path that is no new leaf,
/// and a stored value of the wrong shape are `E-RCP-005`.
pub(super) fn map_value(input: &ValueMapping<'_>) -> Result<StoredValue, Vec<RecipeError>> {
    let tag = input.tag;
    let old = stored_value(tag, input.old_value, input.old_tree).map_err(|error| vec![error])?;
    let (Some(source), Some(target)) = (struct_level(input.old_tree), struct_level(input.new_tree))
    else {
        return Err(vec![shape_error(tag)]);
    };
    let kept = kept_members(&source, &target);
    let mut json = old.as_json().clone();
    let mut found = Found::default();
    let is_shaped = visit_elements(&mut json, &source.chain, &mut |element, prefix| {
        let mapped = map_element(input, (&target, &kept), element, prefix, &mut found);
        *element = mapped;
    });
    if is_shaped.is_none() {
        return Err(vec![shape_error(tag)]);
    }
    let mut entries = found.entries;
    entries.extend(unknown_paths(input, &found.used));
    if !entries.is_empty() {
        return Err(entries);
    }
    stored_value(tag, &StoredValue::from(json), input.new_tree)
        .map(|value| value.to_stored())
        .map_err(|error| vec![error])
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
    let platform = input.new_tree.platform();
    let converted = match given {
        MemberValue::Json(value) => request_leaf(value, node, platform),
        MemberValue::Text(text) => request_text(text, node, platform),
    };
    match converted {
        Ok(value) => value,
        Err(expected) => {
            entries.push(RecipeError::ValueShapeMismatch {
                tag,
                leaf,
                expected,
            });
            Value::Null
        }
    }
}

#[cfg(test)]
mod tests;
