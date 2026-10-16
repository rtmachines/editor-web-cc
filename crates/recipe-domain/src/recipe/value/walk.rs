//! The walk of a JSON value and a type tree together (design D8).
//!
//! The walk uses an explicit stack and never recurses (`reliability.md` section 2.2). It
//! replaces each leaf in place by its canonical stored form.

use std::collections::BTreeMap;

use serde_json::Value;

use super::ParameterValue;
use super::expected::Expected;
use super::fault::ValueFault;
use super::leaf::{LeafType, Mode, convert};
use crate::plant::limits::MAX_DEPTH;
use crate::plant::{
    ArrayDimension, ArrayDimensions, LeafSegment, MemberName, TypeNode, TypeTree, extended,
};

/// One part of the value that the walk has not checked yet.
struct Task<'a> {
    slot: &'a mut Value,
    node: &'a TypeNode,
    /// Segments of the path to the node.
    path: Vec<LeafSegment>,
    /// PLC indexes of the dimensions that the walk has passed in the current array.
    indexes: Vec<i32>,
    /// Number of dimensions of the current array that the walk has passed.
    level: usize,
}

/// Check `json` against `tree` and return the canonical value.
pub(super) fn canonicalize(
    mut json: Value,
    tree: &TypeTree,
    mode: Mode,
) -> Result<ParameterValue, ValueFault> {
    if exceeds_depth(&json) {
        return Err(ValueFault::mismatch(
            Vec::new(),
            Expected::Depth { levels: MAX_DEPTH },
        ));
    }
    let mut stack = vec![Task {
        slot: &mut json,
        node: tree.root(),
        path: Vec::new(),
        indexes: Vec::new(),
        level: 0,
    }];
    while let Some(task) = stack.pop() {
        match task.node {
            TypeNode::Leaf {
                primitive,
                string_length,
                ..
            } => {
                let leaf = LeafType {
                    primitive: *primitive,
                    string_length: *string_length,
                    platform: tree.platform(),
                    mode,
                };
                let stored = convert(task.slot, leaf)
                    .map_err(|expected| ValueFault::mismatch(task.path, expected))?;
                *task.slot = stored;
            }
            TypeNode::Struct { members, .. } => {
                stack.extend(struct_tasks(task.slot, members, &task.path)?);
            }
            TypeNode::Array { .. } => stack.extend(array_tasks(task)?),
        }
    }
    Ok(ParameterValue {
        root: tree.root_type(),
        platform: tree.platform(),
        json,
    })
}

/// Return `true` when the value has more than `json.max_depth` levels of arrays and objects.
///
/// The count uses an explicit stack and stops at the first value that is too deep.
fn exceeds_depth(json: &Value) -> bool {
    let mut stack = vec![(json, 1_u32)];
    while let Some((value, depth)) = stack.pop() {
        let children: Vec<&Value> = match value {
            Value::Array(items) => items.iter().collect(),
            Value::Object(members) => members.values().collect(),
            _ => continue,
        };
        if depth > MAX_DEPTH {
            return true;
        }
        let Some(next) = depth.checked_add(1) else {
            return true;
        };
        stack.extend(children.into_iter().map(|child| (child, next)));
    }
    false
}

/// Match the keys of a JSON object with the members of a struct.
///
/// The tasks come out in reverse member order, so the first member pops first.
fn struct_tasks<'a>(
    slot: &'a mut Value,
    members: &'a [(MemberName, TypeNode)],
    path: &[LeafSegment],
) -> Result<Vec<Task<'a>>, ValueFault> {
    let expected = || Expected::Object {
        members: members.iter().map(|(name, _)| name.clone()).collect(),
    };
    let Value::Object(map) = slot else {
        return Err(ValueFault::mismatch(path.to_vec(), expected()));
    };
    let mut slots: BTreeMap<&str, &mut Value> = map
        .iter_mut()
        .map(|(key, value)| (key.as_str(), value))
        .collect();
    let mut tasks = Vec::with_capacity(members.len());
    for (name, node) in members {
        let Some(child) = slots.remove(name.as_str()) else {
            return Err(ValueFault::mismatch(path.to_vec(), expected()));
        };
        tasks.push(Task {
            slot: child,
            node,
            path: extended(path, LeafSegment::Member(name.clone())),
            indexes: Vec::new(),
            level: 0,
        });
    }
    if !slots.is_empty() {
        return Err(ValueFault::mismatch(path.to_vec(), expected()));
    }
    tasks.reverse();
    Ok(tasks)
}

/// Return the dimension at `level` and the items of the JSON array that must match it.
///
/// `here` is the path of the array in a fault.
fn dimension_items<'a>(
    slot: &'a mut Value,
    dimensions: &ArrayDimensions,
    level: usize,
    here: &[LeafSegment],
) -> Result<(ArrayDimension, &'a mut Vec<Value>), ValueFault> {
    let Some(dimension) = dimensions.iter().nth(level) else {
        return Err(ValueFault::mismatch(here.to_vec(), Expected::Array));
    };
    let count = dimension.count();
    let expected = || Expected::ArrayCount { count };
    let Value::Array(items) = slot else {
        return Err(ValueFault::mismatch(here.to_vec(), expected()));
    };
    if u32::try_from(items.len()) != Ok(count) {
        return Err(ValueFault::mismatch(here.to_vec(), expected()));
    }
    Ok((*dimension, items))
}

/// Match the JSON array of one dimension with the bounds of that dimension.
///
/// The tasks come out in reverse element order. A task for an element of the last dimension
/// holds the element node. A task for any other dimension holds the array node again.
fn array_tasks(task: Task<'_>) -> Result<Vec<Task<'_>>, ValueFault> {
    let TypeNode::Array {
        dimensions,
        element,
        ..
    } = task.node
    else {
        return Err(ValueFault::mismatch(task.path, Expected::Array));
    };
    let here = located(&task.path, &task.indexes);
    let (dimension, items) = dimension_items(task.slot, dimensions, task.level, &here)?;
    let Some(next_level) = task.level.checked_add(1) else {
        return Err(ValueFault::mismatch(here, Expected::Array));
    };
    let is_last = next_level == dimensions.len();
    let mut tasks = Vec::with_capacity(items.len());
    for (position, item) in items.iter_mut().enumerate() {
        let Some(index) = plc_index(dimension.lower(), position) else {
            return Err(ValueFault::mismatch(here, Expected::Index));
        };
        let mut reached = task.indexes.clone();
        reached.push(index);
        tasks.push(if is_last {
            Task {
                slot: item,
                node: element,
                path: extended(&task.path, LeafSegment::Index(reached)),
                indexes: Vec::new(),
                level: 0,
            }
        } else {
            Task {
                slot: item,
                node: task.node,
                path: task.path.clone(),
                indexes: reached,
                level: next_level,
            }
        });
    }
    tasks.reverse();
    Ok(tasks)
}

/// Return the PLC index of the element at a JSON position.
pub(in crate::recipe) fn plc_index(lower: i32, position: usize) -> Option<i32> {
    let sum = i64::from(lower).checked_add(i64::try_from(position).ok()?)?;
    i32::try_from(sum).ok()
}

/// Return the path of the part that the walk reached, with the indexes passed so far.
fn located(path: &[LeafSegment], indexes: &[i32]) -> Vec<LeafSegment> {
    if indexes.is_empty() {
        path.to_vec()
    } else {
        extended(path, LeafSegment::Index(indexes.to_vec()))
    }
}
