//! The scalar leaves of a value that matches its type tree (design D8).
//!
//! The walk uses an explicit stack and never recurses (`reliability.md` section 2.2).

use serde_json::Value;

use super::plc_index;
use crate::plant::{LeafPath, LeafSegment, Primitive, TypeNode, TypeTree, extended};

/// One scalar leaf of a value with its concrete path.
pub(super) struct ValueLeaf<'a> {
    /// The concrete path. It has one index segment for each array and never `[*]`.
    pub(super) path: LeafPath,
    /// The primitive of the leaf type.
    pub(super) primitive: Primitive,
    /// The canonical JSON of the leaf.
    pub(super) value: &'a Value,
}

/// One part of the value that the walk has not listed yet.
struct Task<'a> {
    value: &'a Value,
    node: &'a TypeNode,
    path: Vec<LeafSegment>,
    /// PLC indexes of the dimensions that the walk has passed in the current array.
    indexes: Vec<i32>,
    /// Number of dimensions of the current array that the walk has passed.
    level: usize,
}

/// List the scalar leaves of `json` in the order of `TypeTree::leaves`.
///
/// The function gives `None` when the shape of the value differs from the tree.
pub(super) fn value_leaves<'a>(json: &'a Value, tree: &'a TypeTree) -> Option<Vec<ValueLeaf<'a>>> {
    let mut found = Vec::new();
    let mut stack = vec![Task {
        value: json,
        node: tree.root(),
        path: Vec::new(),
        indexes: Vec::new(),
        level: 0,
    }];
    while let Some(task) = stack.pop() {
        match task.node {
            TypeNode::Leaf { primitive, .. } => found.push(ValueLeaf {
                path: LeafPath::from_segments(task.path),
                primitive: *primitive,
                value: task.value,
            }),
            TypeNode::Struct { members, .. } => {
                let Value::Object(map) = task.value else {
                    return None;
                };
                for (name, node) in members.iter().rev() {
                    stack.push(Task {
                        value: map.get(name.as_str())?,
                        node,
                        path: extended(&task.path, LeafSegment::Member(name.clone())),
                        indexes: Vec::new(),
                        level: 0,
                    });
                }
            }
            TypeNode::Array { .. } => push_elements(task, &mut stack)?,
        }
    }
    Some(found)
}

/// Push the tasks of the elements of one array dimension, last element first.
fn push_elements<'a>(task: Task<'a>, stack: &mut Vec<Task<'a>>) -> Option<()> {
    let TypeNode::Array {
        dimensions,
        element,
        ..
    } = task.node
    else {
        return None;
    };
    let dimension = dimensions.iter().nth(task.level)?;
    let Value::Array(items) = task.value else {
        return None;
    };
    if u32::try_from(items.len()).ok()? != dimension.count() {
        return None;
    }
    let next_level = task.level.checked_add(1)?;
    let is_last = next_level == dimensions.len();
    for (position, item) in items.iter().enumerate().rev() {
        let index = plc_index(dimension.lower(), position)?;
        let mut reached = task.indexes.clone();
        reached.push(index);
        stack.push(if is_last {
            Task {
                value: item,
                node: element,
                path: extended(&task.path, LeafSegment::Index(reached)),
                indexes: Vec::new(),
                level: 0,
            }
        } else {
            Task {
                value: item,
                node: task.node,
                path: task.path.clone(),
                indexes: reached,
                level: next_level,
            }
        });
    }
    Some(())
}
