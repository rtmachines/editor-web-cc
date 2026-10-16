//! Check of a value against the process limits of its leaves (invariant I7, design D8, D9).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use rust_decimal::Decimal;
use serde_json::Value;

use super::ParameterValue;
use super::fault::{ValueFault, ValueFaults};
use super::leaves::{ValueLeaf, value_leaves};
use super::numeric::numeric_range;
use super::{canonical, parse_decimal};
use crate::plant::{LeafPath, PlcPlatform, Primitive, TimeType, TypeTree};
use crate::recipe::limit::{ProcessLimit, bound_text};

impl ParameterValue {
    /// Check the value against `limits` and return the first fault (invariant I7).
    ///
    /// [`ParameterValue::check_all`] gives the order of the faults.
    ///
    /// # Errors
    ///
    /// Returns the first fault of [`ParameterValue::check_all`].
    pub fn check(&self, tree: &TypeTree, limits: &[ProcessLimit]) -> Result<(), ValueFault> {
        self.check_all(tree, limits)
            .map_err(|faults| faults.first().clone())
    }

    /// Check the value against `limits` and return every fault (invariant I7, ADR-0041).
    ///
    /// The cost is the number of leaves plus the number of leaves that the limits cover. A tree
    /// or a limit of another root type gives one fault. Otherwise the list first holds each leaf
    /// that two limits cover, in the order of the limits. Then it holds each leaf outside its
    /// limits, in tree order.
    ///
    /// # Errors
    ///
    /// Returns [`ValueFault::WrongTree`] when `tree` or a limit has another root type or platform.
    /// Returns [`ValueFault::LimitConflict`] for each leaf that two limits cover.
    /// Returns [`ValueFault::OutOfLimit`] for each leaf outside its limits.
    pub fn check_all(&self, tree: &TypeTree, limits: &[ProcessLimit]) -> Result<(), ValueFaults> {
        if tree.root_type() != self.root || tree.platform() != self.platform {
            return Err(ValueFault::WrongTree.into());
        }
        let faults = ordered_faults(&self.json, tree, limits, BTreeMap::new())?;
        ValueFaults::from_vec(faults).map_or(Ok(()), Err)
    }
}

/// Check the leaves of `json` against `limits` and merge the faults of the leaves in `shape`.
///
/// `shape` holds the fault of each leaf that did not convert (ADR-0041 (b)). Such a leaf gets no
/// check against its limit. The list first holds each leaf that two limits cover, in the order
/// of the limits. Then it holds the fault of each leaf in `shape` and of each leaf outside its
/// limits, in tree order.
///
/// # Errors
///
/// Returns [`ValueFault::WrongTree`] when a limit has another root type or platform than `tree`,
/// or when `json` has not the shape of `tree`.
pub(super) fn ordered_faults(
    json: &Value,
    tree: &TypeTree,
    limits: &[ProcessLimit],
    mut shape: BTreeMap<LeafPath, ValueFault>,
) -> Result<Vec<ValueFault>, ValueFault> {
    let is_other_tree = |limit: &ProcessLimit| {
        limit.root_type() != tree.root_type() || limit.platform() != tree.platform()
    };
    if limits.iter().any(is_other_tree) {
        return Err(ValueFault::WrongTree);
    }
    let (cover, mut faults) = covered_leaves(limits, tree)?;
    let leaves = value_leaves(json, tree).ok_or(ValueFault::WrongTree)?;
    for leaf in &leaves {
        if let Some(fault) = shape.remove(&leaf.path) {
            faults.push(fault);
        } else if let Some(limit) = cover.get(&leaf.path) {
            faults.extend(check_leaf(leaf, limit, tree.platform()).err());
        }
    }
    faults.extend(shape.into_values());
    Ok(faults)
}

/// Map each concrete leaf path to the limit that covers it, and list each leaf that two limits
/// cover.
fn covered_leaves<'a>(
    limits: &'a [ProcessLimit],
    tree: &TypeTree,
) -> Result<(BTreeMap<LeafPath, &'a ProcessLimit>, Vec<ValueFault>), ValueFault> {
    let mut cover = BTreeMap::new();
    let mut conflicts = Vec::new();
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
                Entry::Occupied(taken) => conflicts.push(ValueFault::LimitConflict {
                    leaf: taken.key().clone(),
                }),
            }
        }
    }
    Ok((cover, conflicts))
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
