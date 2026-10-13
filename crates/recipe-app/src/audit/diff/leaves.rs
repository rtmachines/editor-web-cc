//! Read the stored value of a parameter at the leaf paths of its type tree.
//!
//! `recipe-domain` walks a value and a tree together, but it does not export a function that
//! reads the value at one `LeafPath`. This module holds the small navigator that the diffs need.

use std::collections::BTreeMap;

use recipe_domain::plant::{LeafSegment, TypeNode, TypeTree};
use serde_json::Value;

/// Return the leaves of `value`, keyed by the text of their concrete leaf path.
///
/// A leaf that the value does not hold is absent from the result.
pub(super) fn stored_leaves(value: &Value, tree: &TypeTree) -> BTreeMap<String, Value> {
    tree.leaves()
        .iter()
        .filter_map(|leaf| {
            let found = value_at(value, tree.root(), leaf.path().segments())?;
            Some((leaf.path().to_string(), found.clone()))
        })
        .collect()
}

/// Follow `segments` through `root` and `value` together.
///
/// The function maps a PLC index of an array to the position in the JSON array with the lower
/// bound of the dimension. It gives `None` when the value has another shape than the tree.
fn value_at<'a>(value: &'a Value, root: &TypeNode, segments: &[LeafSegment]) -> Option<&'a Value> {
    let mut node = root;
    let mut current = value;
    for segment in segments {
        match (segment, node) {
            (LeafSegment::Member(name), TypeNode::Struct { members, .. }) => {
                node = members
                    .iter()
                    .find(|(member, _)| member == name)
                    .map(|(_, child)| child)?;
                current = current.get(name.as_str())?;
            }
            (
                LeafSegment::Index(indexes),
                TypeNode::Array {
                    dimensions,
                    element,
                    ..
                },
            ) => {
                if indexes.len() != dimensions.len() {
                    return None;
                }
                for (index, dimension) in indexes.iter().zip(dimensions.iter()) {
                    let position = usize::try_from(index.checked_sub(dimension.lower())?).ok()?;
                    current = current.get(position)?;
                }
                node = element;
            }
            _ => return None,
        }
    }
    Some(current)
}
