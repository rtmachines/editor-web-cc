//! Check of a value against the process limits of its leaves (invariant I7, design D8, D9).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use rust_decimal::Decimal;
use serde_json::Value;

use super::ParameterValue;
use super::fault::ValueFault;
use super::leaves::{ValueLeaf, value_leaves};
use super::numeric::numeric_range;
use super::{canonical, parse_decimal};
use crate::plant::{LeafPath, PlcPlatform, Primitive, TimeType, TypeTree};
use crate::recipe::limit::{ProcessLimit, bound_text};

impl ParameterValue {
    /// Check the value against `limits`, the process limits of its parameter (invariant I7).
    ///
    /// The cost is the number of leaves plus the number of leaves that the limits cover. The
    /// function reports in this order. First a tree or a limit of another root type. Then two
    /// limits for one leaf. Then the first leaf in tree order that is outside its limits.
    ///
    /// # Errors
    ///
    /// Returns [`ValueFault::WrongTree`] when `tree` or a limit has another root type or platform.
    /// Returns [`ValueFault::LimitConflict`] when two limits cover one leaf.
    /// Returns [`ValueFault::OutOfLimit`] when a leaf is outside its limits.
    pub fn check(&self, tree: &TypeTree, limits: &[ProcessLimit]) -> Result<(), ValueFault> {
        let is_same = |root, platform| root == self.root && platform == self.platform;
        let is_wrong_tree = !is_same(tree.root_type(), tree.platform())
            || limits.iter().any(|l| !is_same(l.root_type(), l.platform()));
        if is_wrong_tree {
            return Err(ValueFault::WrongTree);
        }
        let cover = covered_leaves(limits, tree)?;
        let leaves = value_leaves(&self.json, tree).ok_or(ValueFault::WrongTree)?;
        leaves
            .iter()
            .filter_map(|leaf| cover.get(&leaf.path).map(|limit| (leaf, *limit)))
            .try_for_each(|(leaf, limit)| check_leaf(leaf, limit, tree.platform()))
    }
}

/// Map each concrete leaf path to the limit that covers it. Two limits for one leaf conflict.
fn covered_leaves<'a>(
    limits: &'a [ProcessLimit],
    tree: &TypeTree,
) -> Result<BTreeMap<LeafPath, &'a ProcessLimit>, ValueFault> {
    let mut cover = BTreeMap::new();
    for limit in limits {
        let paths = limit
            .leaf_path()
            .concrete_paths(tree)
            .ok_or(ValueFault::WrongTree)?;
        for path in paths {
            match cover.entry(path) {
                Entry::Vacant(slot) => {
                    slot.insert(limit);
                }
                Entry::Occupied(taken) => {
                    return Err(ValueFault::LimitConflict {
                        leaf: taken.key().clone(),
                    });
                }
            }
        }
    }
    Ok(cover)
}

/// Check one leaf against the limit that covers it.
fn check_leaf(
    leaf: &ValueLeaf<'_>,
    limit: &ProcessLimit,
    platform: PlcPlatform,
) -> Result<(), ValueFault> {
    let fault = |value: String, min: String, max: String| ValueFault::OutOfLimit {
        leaf: leaf.path.clone(),
        value,
        min,
        max,
    };
    if let Some(max_length) = limit.max_length() {
        let Value::String(text) = leaf.value else {
            return Err(ValueFault::WrongTree);
        };
        let length = if leaf.primitive == Primitive::Wstring {
            text.encode_utf16().count()
        } else {
            text.chars().count()
        };
        return if length > usize::from(max_length) {
            Err(fault(
                length.to_string(),
                "0".into(),
                max_length.to_string(),
            ))
        } else {
            Ok(())
        };
    }
    let Value::Number(number) = leaf.value else {
        return Err(ValueFault::WrongTree);
    };
    let found = parse_decimal(&number.to_string())
        .and_then(canonical)
        .ok_or(ValueFault::WrongTree)?;
    let (low, high) = type_range(leaf.primitive, platform).ok_or(ValueFault::WrongTree)?;
    let min = limit.min().unwrap_or(low);
    let max = limit.max().unwrap_or(high);
    if found >= min && found <= max {
        return Ok(());
    }
    let text = |bound| bound_text(leaf.primitive, platform, bound);
    Err(fault(text(found), text(min), text(max)))
}

/// Return the lowest and the highest value of a numeric, duration, or instant type.
fn type_range(primitive: Primitive, platform: PlcPlatform) -> Option<(Decimal, Decimal)> {
    if let Some(range) = numeric_range(primitive) {
        return Some(range);
    }
    let (low, high) = TimeType::of(primitive, platform)?.range()?;
    Some((
        Decimal::try_from_i128_with_scale(low, 0).ok()?,
        Decimal::try_from_i128_with_scale(high, 0).ok()?,
    ))
}

#[cfg(test)]
mod tests;
