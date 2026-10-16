//! The new member leaves of a parameter after a rebind (ADR-0018, ADR-0039 (f), design D4).
//!
//! The plan of the command and this function use one rule. `TypeTree::rebind` gives the planned
//! tree. `struct_level` and `kept_members` give the members that keep their value. Every other
//! member of the new revision is new, and each of its leaves needs a value.

use super::level::{kept_members, struct_level};
use crate::plant::{Leaf, LeafSegment, PlantError, PlcTypeId, TypeTree};

/// The tree of a parameter after a rebind and the new member leaves that need a value.
///
/// The type solves one problem: a page shows one input per new member leaf, and it converts the
/// text of each input by the class of its leaf in the planned tree. Both come from one rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebindLeaves {
    tree: TypeTree,
    leaves: Vec<Leaf>,
}

impl RebindLeaves {
    /// Return the planned tree of the parameter. A new leaf path resolves in it.
    #[must_use]
    pub const fn tree(&self) -> &TypeTree {
        &self.tree
    }

    /// Return the new member leaves, element by element, in member order inside one element.
    ///
    /// This is the order of the entries `E-RCP-028` of a rebind without values.
    #[must_use]
    pub fn leaves(&self) -> &[Leaf] {
        &self.leaves
    }
}

/// Return the new member leaves of a parameter of type `tree` after a rebind from `from` to `to`.
///
/// Returns `None` when the rebind does not move the parameter. The rebind moves a parameter whose
/// type is the struct `from`, or an array whose element chain ends in it (ADR-0018).
///
/// # Errors
///
/// Returns `E-PLT-018` when the planned tree is above a size limit, as the command does.
pub fn rebind_new_leaves(
    tree: &TypeTree,
    from: PlcTypeId,
    to: &TypeTree,
) -> Result<Option<RebindLeaves>, PlantError> {
    let Some(planned) = tree.rebind(from, to)? else {
        return Ok(None);
    };
    let leaves = {
        let (Some(source), Some(target)) = (struct_level(tree), struct_level(&planned)) else {
            return Ok(None);
        };
        let kept = kept_members(&source, &target);
        let depth = source.chain.len();
        planned
            .leaves()
            .into_iter()
            .filter(|leaf| {
                matches!(leaf.path().segments().get(depth),
                    Some(LeafSegment::Member(name)) if !kept.contains(name))
            })
            .collect()
    };
    Ok(Some(RebindLeaves {
        tree: planned,
        leaves,
    }))
}

#[cfg(test)]
mod tests;
