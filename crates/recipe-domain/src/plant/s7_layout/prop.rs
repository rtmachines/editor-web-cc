//! Property tests of `S7Layout` on random type trees (`reliability.md` section 6).

use proptest::prelude::*;

use super::super::type_tree::strategy::type_trees;
use super::super::{Primitive, TypeNode, TypeTree};
use super::*;

/// Bits in one byte.
const BITS_PER_BYTE: u64 = 8;

fn resolved(
    generated: &super::super::type_tree::strategy::GeneratedTypes,
) -> Result<TypeTree, TestCaseError> {
    TypeTree::resolve(generated.root, &generated.types)
        .map_err(|error| TestCaseError::fail(error.to_string()))
}

fn layout(tree: &TypeTree) -> Result<S7Layout, TestCaseError> {
    S7Layout::of(tree).map_err(|error| TestCaseError::fail(error.to_string()))
}

/// Lower bound of the size: the non-`BOOL` leaves plus the `BOOL` leaves in whole bytes.
///
/// The walk counts each leaf once per element of every array above it.
fn storage_lower_bound(tree: &TypeTree) -> u64 {
    let mut stack = vec![(tree.root(), 1_u64)];
    let mut bytes = 0_u64;
    let mut bools = 0_u64;
    while let Some((node, copies)) = stack.pop() {
        match node {
            TypeNode::Leaf {
                primitive: Primitive::Bool,
                ..
            } => bools = bools.checked_add(copies).unwrap(),
            TypeNode::Leaf { size_bytes, .. } => {
                let leaf_bytes = copies.checked_mul(u64::from(*size_bytes)).unwrap();
                bytes = bytes.checked_add(leaf_bytes).unwrap();
            }
            TypeNode::Array {
                dimensions,
                element,
                ..
            } => {
                let count = dimensions.element_count().unwrap_or(0);
                stack.push((element, copies.checked_mul(count).unwrap()));
            }
            TypeNode::Struct { members, .. } => {
                stack.extend(members.iter().map(|(_, child)| (child, copies)));
            }
        }
    }
    bytes.checked_add(bools.div_ceil(BITS_PER_BYTE)).unwrap()
}

proptest! {
    #[test]
    fn layout_of_a_generated_tree_is_ok(generated in type_trees()) {
        let tree = resolved(&generated)?;
        prop_assert!(S7Layout::of(&tree).is_ok());
    }

    #[test]
    fn struct_and_array_sizes_are_even_and_start_even(generated in type_trees()) {
        let tree = resolved(&generated)?;
        let found = layout(&tree)?;
        if matches!(tree.root(), TypeNode::Leaf { .. }) {
            prop_assert_eq!(tree.leaf_count(), 1);
        } else {
            prop_assert_eq!(found.size_bytes() % 2, 0);
            prop_assert_eq!(found.start(), S7Start::Even);
        }
    }

    #[test]
    fn size_is_at_least_the_storage_of_the_leaves(generated in type_trees()) {
        let tree = resolved(&generated)?;
        let found = layout(&tree)?;
        prop_assert!(u64::from(found.size_bytes()) >= storage_lower_bound(&tree));
    }

    #[test]
    fn leaf_root_has_the_size_of_the_leaf(generated in type_trees()) {
        let tree = resolved(&generated)?;
        if let TypeNode::Leaf { size_bytes, .. } = tree.root() {
            prop_assert_eq!(layout(&tree)?.size_bytes(), *size_bytes);
        }
    }
}

#[test]
fn strategy_reaches_struct_array_and_leaf_roots() {
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::deterministic();
    let strategy = type_trees();
    let mut kinds = [0_u32; 3];
    for _ in 0..300 {
        let generated = strategy.new_tree(&mut runner).unwrap().current();
        let tree = TypeTree::resolve(generated.root, &generated.types).unwrap();
        let slot = match tree.root() {
            TypeNode::Leaf { .. } => 0,
            TypeNode::Array { .. } => 1,
            TypeNode::Struct { .. } => 2,
        };
        kinds[slot] += 1;
    }
    assert!(kinds.iter().all(|count| *count > 0), "{kinds:?}");
}
