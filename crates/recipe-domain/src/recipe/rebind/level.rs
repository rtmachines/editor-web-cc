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

/// Return the names of the target members that the source also has with the same type.
///
/// Every other target member is new. A source member that is not in the result is dropped
/// (ADR-0018: a member whose type changed is treated as new).
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
                .any(|(old, old_node)| old == name && old_node.type_id() == node.type_id())
        })
        .map(|(name, _)| name)
        .collect()
}
