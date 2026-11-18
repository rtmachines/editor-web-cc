//! The tree of a parameter after a rebind to a newer struct revision (design D13, ADR-0018).
//!
//! The chain of nested arrays is a loop and never a recursion (`reliability.md` section 2.2).

use crate::plant::limits::{MAX_DEPTH, MAX_LEAVES};
use crate::plant::{ArrayDimensions, PlantError, PlcTypeId, TypeLimit};

use super::{S7_ARRAY_OF_STRUCT_EXTRA, TypeNode, TypeTree};

impl TypeTree {
    /// Build the tree of this type after a rebind from the struct `from` to the tree `to`.
    ///
    /// The type uses `from` when its root is the struct `from`. It also uses `from` when its
    /// root is an array whose element chain ends in the struct `from`. A struct member of type
    /// `from` is no use. The new tree replaces the struct by the root of `to`. Every array node
    /// keeps its dimensions and its old type identifier as a placeholder. The caller creates the
    /// new array rows later. The platform stays.
    ///
    /// Returns `None` when the type does not use `from`, or when the root of `to` is not a
    /// struct.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-018` with the struct name of `to` when the new tree has more leaves than
    /// `MAX_LEAVES` or more levels than `MAX_DEPTH`.
    pub(crate) fn rebind(&self, from: PlcTypeId, to: &Self) -> Result<Option<Self>, PlantError> {
        let TypeNode::Struct { name, .. } = to.root() else {
            return Ok(None);
        };
        let Some(chain) = array_chain(self.root(), from) else {
            return Ok(None);
        };
        let too_large = |limit| PlantError::TypeTooLarge {
            type_name: name.to_string(),
            limit,
        };
        let leaf_count = chain_leaves(&chain, to.leaf_count())
            .filter(|leaves| *leaves <= MAX_LEAVES)
            .ok_or_else(|| too_large(TypeLimit::Leaves))?;
        let depth = chain_depth(&chain, to.depth())
            .filter(|depth| *depth <= MAX_DEPTH)
            .ok_or_else(|| too_large(TypeLimit::Depth))?;
        let s7_nesting = if chain.is_empty() {
            Some(to.s7_nesting)
        } else {
            to.s7_nesting.checked_add(S7_ARRAY_OF_STRUCT_EXTRA)
        }
        .ok_or_else(|| too_large(TypeLimit::Depth))?;
        let root =
            chain
                .into_iter()
                .rev()
                .fold(to.root().clone(), |element, (type_id, dimensions)| {
                    TypeNode::Array {
                        type_id,
                        dimensions: dimensions.clone(),
                        element: Box::new(element),
                    }
                });
        Ok(Some(Self {
            platform: self.platform,
            root,
            leaf_count,
            depth,
            s7_nesting,
        }))
    }
}

/// Multiply the leaves of the new struct by the element count of every array of the chain.
///
/// Returns `None` on an overflow.
fn chain_leaves(chain: &[(PlcTypeId, &ArrayDimensions)], struct_leaves: u32) -> Option<u32> {
    chain
        .iter()
        .try_fold(u64::from(struct_leaves), |leaves, (_, dimensions)| {
            dimensions
                .element_count()
                .and_then(|count| leaves.checked_mul(count))
        })
        .and_then(|leaves| u32::try_from(leaves).ok())
}

/// Add the levels of every array of the chain to the depth of the new struct.
///
/// Returns `None` on an overflow.
fn chain_depth(chain: &[(PlcTypeId, &ArrayDimensions)], struct_depth: u32) -> Option<u32> {
    chain
        .iter()
        .try_fold(struct_depth, |depth, (_, dimensions)| {
            u32::try_from(dimensions.len())
                .ok()
                .and_then(|levels| depth.checked_add(levels))
        })
}

/// Collect the array nodes from the root down to the struct `from`, outermost first.
///
/// Returns `None` when the chain of array elements does not end in the struct `from`.
fn array_chain(root: &TypeNode, from: PlcTypeId) -> Option<Vec<(PlcTypeId, &ArrayDimensions)>> {
    let mut chain = Vec::new();
    let mut node = root;
    loop {
        match node {
            TypeNode::Array {
                type_id,
                dimensions,
                element,
            } => {
                chain.push((*type_id, dimensions));
                node = element;
            }
            TypeNode::Struct { type_id, .. } if *type_id == from => return Some(chain),
            TypeNode::Struct { .. } | TypeNode::Leaf { .. } => return None,
        }
    }
}

#[cfg(test)]
mod tests;
