//! Expansion of `[*]` into the concrete leaf paths of a type tree.

use super::{LeafPath, LeafSegment, extended, step};
use crate::plant::type_tree::index_tuples;
use crate::plant::{TypeNode, TypeTree};

impl LeafPath {
    /// List the concrete paths that this path names inside `tree`.
    ///
    /// A path without `[*]` gives itself. Each `[*]` runs over every index tuple of its array
    /// in row-major order, the order of [`TypeTree::leaves`]. The function gives `None` when the
    /// tree has no scalar leaf at the path. The result has [`LeafTarget::count`] paths.
    ///
    /// [`LeafTarget::count`]: crate::plant::LeafTarget::count
    pub(crate) fn concrete_paths(&self, tree: &TypeTree) -> Option<Vec<LeafPath>> {
        let mut node = tree.root();
        let mut paths: Vec<Vec<LeafSegment>> = vec![Vec::new()];
        for segment in self.segments() {
            let (next, _) = step(node, segment)?;
            paths = match (segment, node) {
                (LeafSegment::All, TypeNode::Array { dimensions, .. }) => {
                    let tuples = index_tuples(dimensions);
                    paths
                        .iter()
                        .flat_map(|prefix| {
                            tuples
                                .iter()
                                .map(|tuple| extended(prefix, LeafSegment::Index(tuple.clone())))
                        })
                        .collect()
                }
                (other, _) => paths
                    .iter()
                    .map(|prefix| extended(prefix, other.clone()))
                    .collect(),
            };
            node = next;
        }
        matches!(node, TypeNode::Leaf { .. })
            .then(|| paths.into_iter().map(LeafPath::from_segments).collect())
    }
}

#[cfg(test)]
mod tests;
