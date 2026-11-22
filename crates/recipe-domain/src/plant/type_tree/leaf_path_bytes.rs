//! The length of the longest leaf path of a resolved type tree (ADR-0211).
//!
//! The walk uses an explicit stack and never recurses (`reliability.md` section 2.2).
//! A sum that overflows stays at `usize::MAX`, which is above the limit.

use crate::plant::limits::MAX_LEAF_PATH_BYTES;
use crate::plant::{ArrayDimensions, PlantError, TypeLimit};

use super::{TypeNode, TypeTree, render_node};

impl TypeTree {
    /// Return the number of bytes of the longest canonical leaf path of the tree.
    ///
    /// The count equals the longest text that [`TypeTree::leaves`] gives. It adds `.` and the
    /// member name for each struct level and the index text for each array level. The index
    /// text of a dimension has the length of its longer bound, because the length of an integer
    /// grows with its size. The count needs no list of leaves.
    #[must_use]
    pub fn longest_leaf_path_bytes(&self) -> usize {
        let mut longest = 0;
        let mut stack = vec![(self.root(), 0_usize)];
        while let Some((node, prefix)) = stack.pop() {
            match node {
                TypeNode::Leaf { .. } => longest = longest.max(prefix),
                TypeNode::Struct { members, .. } => {
                    for (name, child) in members {
                        let segment = name.as_str().len().saturating_add(1);
                        stack.push((child, prefix.saturating_add(segment)));
                    }
                }
                TypeNode::Array {
                    dimensions,
                    element,
                    ..
                } => stack.push((
                    element.as_ref(),
                    prefix.saturating_add(index_text_bytes(dimensions)),
                )),
            }
        }
        longest
    }

    /// Check the length of the leaf paths of a type that a command creates (ADR-0211 (b), (c)).
    ///
    /// `resolve` does not run this check, so a stored type with a longer leaf path stays
    /// readable. The caller runs the check where a command creates a type row.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-018` with [`TypeLimit::LeafPathBytes`] and the text of the root type, when
    /// the longest leaf path has more than [`MAX_LEAF_PATH_BYTES`] bytes.
    pub fn check_leaf_path_bytes(&self) -> Result<(), PlantError> {
        if self.longest_leaf_path_bytes() > MAX_LEAF_PATH_BYTES {
            return Err(PlantError::TypeTooLarge {
                type_name: render_node(self.root()),
                limit: TypeLimit::LeafPathBytes,
            });
        }
        Ok(())
    }
}

/// Return the largest byte count of the index segment of an array, such as `[-12,3]`.
fn index_text_bytes(dimensions: &ArrayDimensions) -> usize {
    let numbers = dimensions.iter().fold(0_usize, |sum, dimension| {
        let wider = decimal_bytes(dimension.lower()).max(decimal_bytes(dimension.upper()));
        sum.saturating_add(wider)
    });
    let commas = dimensions.len().saturating_sub(1);
    numbers.saturating_add(commas).saturating_add(2)
}

/// Return the number of bytes of the decimal text of `value`, with its sign.
fn decimal_bytes(value: i32) -> usize {
    value.to_string().len()
}
