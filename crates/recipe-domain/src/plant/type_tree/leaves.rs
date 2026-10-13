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
mod tests {
    use super::super::fixtures::{
        PLATFORM, array_row, id, primitive_row, string_row, struct_row, types,
    };
    use crate::plant::{LeafPath, Primitive, TypeTree};

    const INT: u128 = 1;
    const REAL: u128 = 2;

    fn tree_of(mut rows: Vec<crate::plant::PlcType>, root: u128) -> TypeTree {
        let mut all = vec![
            primitive_row(INT, Primitive::Int),
            primitive_row(REAL, Primitive::Real),
        ];
        all.append(&mut rows);
        TypeTree::resolve(id(root), &types(all)).unwrap()
    }

    fn paths(tree: &TypeTree) -> Vec<String> {
        tree.leaves()
            .iter()
            .map(|leaf| leaf.path().to_string())
            .collect()
    }

    #[test]
    fn tree_keeps_the_platform_of_the_platform_types() {
        let tree = tree_of(Vec::new(), INT);
        assert_eq!(tree.platform(), PLATFORM);
    }

    #[test]
    fn primitive_gives_one_leaf_with_the_empty_path() {
        let tree = tree_of(Vec::new(), INT);
        let leaves = tree.leaves();
        assert_eq!(leaves.len(), 1);
        let leaf = leaves.first().unwrap();
        assert!(leaf.path().is_empty());
        assert_eq!(leaf.target().primitive(), Primitive::Int);
        assert_eq!(leaf.target().count(), 1);
    }

    #[test]
    fn member_array_gives_one_path_per_element() {
        let rows = vec![
            array_row(10, INT, &[(1, 3)]),
            struct_row(20, "ST_Zone", &[("temp_sp", REAL), ("ramps", 10)]),
        ];
        let tree = tree_of(rows, 20);
        assert_eq!(
            paths(&tree),
            [".temp_sp", ".ramps[1]", ".ramps[2]", ".ramps[3]"]
        );
    }

    #[test]
    fn two_dimensional_array_runs_in_row_major_order() {
        let tree = tree_of(vec![array_row(10, INT, &[(0, 1), (1, 2)])], 10);
        assert_eq!(paths(&tree), ["[0,1]", "[0,2]", "[1,1]", "[1,2]"]);
    }

    #[test]
    fn struct_of_a_struct_joins_the_member_names() {
        let rows = vec![
            struct_row(10, "ST_B", &[("b", INT)]),
            struct_row(11, "ST_A", &[("a", 10), ("z", REAL)]),
        ];
        let tree = tree_of(rows, 11);
        assert_eq!(paths(&tree), [".a.b", ".z"]);
    }

    #[test]
    fn array_of_array_gives_one_segment_per_array() {
        let rows = vec![array_row(10, INT, &[(1, 2)]), array_row(11, 10, &[(5, 6)])];
        let tree = tree_of(rows, 11);
        assert_eq!(paths(&tree), ["[5][1]", "[5][2]", "[6][1]", "[6][2]"]);
    }

    #[test]
    fn leaf_describes_its_type() {
        let rows = vec![string_row(10, 16), struct_row(20, "ST_A", &[("label", 10)])];
        let tree = tree_of(rows, 20);
        let leaf = tree.leaves().into_iter().next().unwrap();
        assert_eq!(leaf.target().primitive(), Primitive::String);
        assert_eq!(leaf.target().string_length(), Some(16));
        assert_eq!(leaf.target().type_id(), id(10));
    }

    #[test]
    fn tree_with_16384_leaves_gives_16384_distinct_leaves() {
        let tree = tree_of(vec![array_row(10, INT, &[(1, 128), (1, 128)])], 10);
        let leaves = tree.leaves();
        assert_eq!(leaves.len(), 16_384);
        assert_eq!(
            leaves.last().map(|leaf| leaf.path().to_string()),
            Some("[128,128]".to_owned())
        );
    }

    #[test]
    fn every_leaf_path_parses_back_and_resolves_to_one_leaf() {
        let rows = vec![
            array_row(10, INT, &[(1, 3)]),
            array_row(11, 10, &[(0, 1), (1, 2)]),
            struct_row(20, "ST_Zone", &[("temp_sp", REAL), ("grid", 11)]),
            struct_row(21, "ST_Line", &[("zone", 20), ("tail", INT)]),
        ];
        let tree = tree_of(rows, 21);
        assert_eq!(
            tree.leaves().len(),
            usize::try_from(tree.leaf_count()).unwrap()
        );
        for leaf in tree.leaves() {
            let parsed = LeafPath::parse(&leaf.path().to_string()).unwrap();
            assert_eq!(&parsed, leaf.path());
            let target = parsed.resolve(&tree).unwrap();
            assert_eq!(target.count(), 1);
            assert_eq!(target, leaf.target());
        }
    }
}
