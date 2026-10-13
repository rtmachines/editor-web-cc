//! Property tests of `resolve` on random type trees (`reliability.md` section 6).

use proptest::prelude::*;

use super::strategy::type_trees;
use super::*;

/// Deepest tree that the strategy builds.
const GENERATED_DEPTH: u32 = 6;

fn resolved(generated: &super::strategy::GeneratedTypes) -> Result<TypeTree, TestCaseError> {
    TypeTree::resolve(generated.root, &generated.types)
        .map_err(|error| TestCaseError::fail(error.to_string()))
}

proptest! {
    #[test]
    fn generated_trees_resolve_to_a_tree(generated in type_trees()) {
        prop_assert!(TypeTree::resolve(generated.root, &generated.types).is_ok());
    }

    #[test]
    fn leaf_count_equals_the_count_of_the_generator(generated in type_trees()) {
        let tree = resolved(&generated)?;
        prop_assert_eq!(u64::from(tree.leaf_count()), generated.expected_leaves);
    }

    #[test]
    fn depth_equals_the_depth_of_the_generator_and_stays_within_six(generated in type_trees()) {
        let tree = resolved(&generated)?;
        prop_assert_eq!(tree.depth(), generated.expected_depth);
        prop_assert!(tree.depth() <= GENERATED_DEPTH);
    }

    #[test]
    fn resolving_twice_gives_equal_trees(generated in type_trees()) {
        let first = resolved(&generated)?;
        let second = resolved(&generated)?;
        prop_assert_eq!(first, second);
    }
}

#[test]
fn strategy_reaches_deep_and_wide_trees() {
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::deterministic();
    let strategy = type_trees();
    let samples: Vec<_> = (0..300)
        .map(|_| strategy.new_tree(&mut runner).unwrap().current())
        .collect();
    let deepest = samples.iter().map(|s| s.expected_depth).max().unwrap();
    let widest = samples.iter().map(|s| s.expected_leaves).max().unwrap();
    assert!(deepest >= 4, "deepest sample has depth {deepest}");
    assert!(widest >= 20, "widest sample has {widest} leaves");
}
