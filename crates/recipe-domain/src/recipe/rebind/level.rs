//! The struct level of a parameter tree and the kept members of a rebind (design D13, step 4).

use std::collections::BTreeSet;

use crate::plant::{ArrayDimensions, MemberName, TypeNode, TypeTree};

/// The array chain of a tree and the members of the struct at its end.
///
/// The chain is empty when the root is the struct. An array root gives one entry for each
/// array node, outermost first.
pub(super) struct StructLevel<'t> {
    pub(super) chain: Vec<&'t ArrayDimensions>,
    pub(super) members: &'t [(MemberName, TypeNode)],
}

/// Return the struct level of `tree`, or `None` when the element chain ends in no struct.
pub(super) fn struct_level(tree: &TypeTree) -> Option<StructLevel<'_>> {
    let mut chain = Vec::new();
    let mut node = tree.root();
    loop {
        match node {
            TypeNode::Array {
                dimensions,
                element,
                ..
            } => {
                chain.push(dimensions);
                node = element;
            }
            TypeNode::Struct { members, .. } => return Some(StructLevel { chain, members }),
            TypeNode::Leaf { .. } => return None,
        }
    }
}

/// Return the names of the target members that the source also has with the same type shape.
///
/// Every other target member is new. A source member that is not in the result is dropped
/// (ADR-0018: a member whose type changed is new). A member of another type shape is treated
/// as new (ADR-0146). Two different rows can have one shape.
pub(super) fn kept_members<'t>(
    source: &StructLevel<'_>,
    target: &StructLevel<'t>,
) -> BTreeSet<&'t MemberName> {
    target
        .members
        .iter()
        .filter(|(name, node)| {
            source
                .members
                .iter()
                .any(|(old, old_node)| old == name && same_type_shape(old_node, node))
        })
        .map(|(name, _)| name)
        .collect()
}

/// Return `true` when two member types have one type shape (ADR-0146 (a) to (f)).
///
/// The function solves one problem: a seed row `STRING` and a machine row `STRING(80)` are two
/// rows with one shape, so a rebind must keep the member. Two leaves have one shape when every
/// field except the row is equal. Two arrays have one shape when the dimensions are equal and
/// the elements have one shape. Two structs have one shape only when they are one row, because
/// a newer struct revision is a new member. The walk follows array elements only, so it needs
/// no recursion.
fn same_type_shape(old: &TypeNode, new: &TypeNode) -> bool {
    let (mut old, mut new) = (old, new);
    loop {
        match (old, new) {
            (
                TypeNode::Leaf {
                    type_id: _,
                    primitive: old_primitive,
                    leaf_class: old_class,
                    size_bytes: old_size,
                    string_length: old_length,
                },
                TypeNode::Leaf {
                    type_id: _,
                    primitive,
                    leaf_class,
                    size_bytes,
                    string_length,
                },
            ) => {
                return old_primitive == primitive
                    && old_class == leaf_class
                    && old_size == size_bytes
                    && old_length == string_length;
            }
            (
                TypeNode::Array {
                    dimensions: old_dimensions,
                    element: old_element,
                    ..
                },
                TypeNode::Array {
                    dimensions,
                    element,
                    ..
                },
            ) => {
                if old_dimensions != dimensions {
                    return false;
                }
                old = old_element;
                new = element;
            }
            (
                TypeNode::Struct {
                    type_id: old_id, ..
                },
                TypeNode::Struct { type_id: id, .. },
            ) => {
                return old_id == id;
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests;
