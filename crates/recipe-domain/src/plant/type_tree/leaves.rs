//! The concrete scalar leaves of a resolved type tree.
//!
//! The walk uses an explicit stack and never recurses (`reliability.md` section 2.2).

use crate::plant::{ArrayDimensions, LeafPath, LeafSegment, LeafTarget, extended};

use super::{TypeNode, TypeTree};

/// One scalar leaf of a tree with its concrete path.
///
/// The path has one index segment for each array and never `[*]`. The target describes the
/// leaf type and has the count 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaf {
    path: LeafPath,
    target: LeafTarget,
}

impl Leaf {
    /// Return the concrete path of the leaf.
    #[must_use]
    pub const fn path(&self) -> &LeafPath {
        &self.path
    }

    /// Return the description of the leaf type. Its count is 1.
    #[must_use]
    pub const fn target(&self) -> LeafTarget {
        self.target
    }
}

impl TypeTree {
    /// List every scalar leaf of the tree in declaration order.
    ///
    /// Struct members run in member order. Array elements run in row-major order from the lower
    /// to the upper bound of each dimension. The function uses an explicit stack.
    #[must_use]
    pub fn leaves(&self) -> Vec<Leaf> {
        let capacity = usize::try_from(self.leaf_count()).unwrap_or_default();
        let mut found = Vec::with_capacity(capacity);
        let mut stack = vec![(self.root(), Vec::new())];
        while let Some((node, path)) = stack.pop() {
            match node {
                TypeNode::Leaf {
                    type_id,
                    primitive,
                    leaf_class,
                    size_bytes,
                    string_length,
                } => found.push(Leaf {
                    path: LeafPath::from_segments(path),
                    target: LeafTarget::single(
                        *type_id,
                        *primitive,
                        *leaf_class,
                        *size_bytes,
                        *string_length,
                    ),
                }),
                TypeNode::Struct { members, .. } => {
                    for (name, child) in members.iter().rev() {
                        stack.push((child, extended(&path, LeafSegment::Member(name.clone()))));
                    }
                }
                TypeNode::Array {
                    dimensions,
                    element,
                    ..
                } => {
                    for indexes in index_tuples(dimensions).into_iter().rev() {
                        stack.push((
                            element.as_ref(),
                            extended(&path, LeafSegment::Index(indexes)),
                        ));
                    }
                }
            }
        }
        found
    }
}

/// List the index tuples of an array in row-major order. The first dimension changes slowest.
pub(crate) fn index_tuples(dimensions: &ArrayDimensions) -> Vec<Vec<i32>> {
    dimensions
        .iter()
        .fold(vec![Vec::new()], |tuples, dimension| {
            tuples
                .iter()
                .flat_map(|prefix| {
                    (dimension.lower()..=dimension.upper()).map(|index| {
                        let mut tuple = prefix.clone();
                        tuple.push(index);
                        tuple
                    })
                })
                .collect()
        })
}

#[cfg(test)]
mod tests;
