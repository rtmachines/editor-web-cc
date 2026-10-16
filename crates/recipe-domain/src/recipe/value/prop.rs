//! Property tests of `parse` and `check` on random type trees (`reliability.md` section 6).

mod generate;

use proptest::prelude::*;

use std::collections::BTreeSet;

use self::generate::{
    Counts, MixedSpec, Mutation, generate_limits, generate_value, mixed_specs,
    tighten_first_numeric,
};
use crate::plant::strategy::{GeneratedTypes, type_trees};
use crate::plant::{Primitive, TypeTree};
use crate::recipe::limit::ProcessLimit;
use crate::recipe::value::{LeafTexts, ParameterValue, ValueFault};

/// Number of cases of each property. The trees are large, so the count stays moderate.
const CASES: u32 = 64;
/// Number of cases of the round trip over every leaf class. These trees are small.
const MIXED_CASES: u32 = 256;
/// Size of the pool of random numbers that drives one generated value.
const POOL_SIZE: usize = 256;

fn resolved(generated: &GeneratedTypes) -> Result<TypeTree, TestCaseError> {
    TypeTree::resolve(generated.root, &generated.types)
        .map_err(|error| TestCaseError::fail(error.to_string()))
}

fn pool() -> impl Strategy<Value = Vec<i32>> {
    prop::collection::vec(any::<i32>(), POOL_SIZE..=POOL_SIZE)
}

/// Parse the text of a generated value.
fn parse(json: &serde_json::Value, tree: &TypeTree) -> Result<ParameterValue, ValueFault> {
    ParameterValue::parse(&json.to_string(), tree)
}

/// Return a position inside `count`, or `None` when the tree has no such part.
fn position(pick: usize, count: usize) -> Option<usize> {
    pick.checked_rem(count)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    #[test]
    fn generated_value_passes_parse_and_check(
        generated in type_trees(),
        seeds in pool(),
    ) {
        let tree = resolved(&generated)?;
        let (json, _) = generate_value(&tree, &seeds, Mutation::None);
        let parsed = parse(&json, &tree).map_err(|f| TestCaseError::fail(f.to_string()))?;
        let limits = generate_limits(&parsed, &tree, &seeds);
        prop_assert_eq!(parsed.check(&tree, &limits), Ok(()));
    }

    #[test]
    fn value_above_a_tightened_limit_fails_check_at_its_leaf(
        generated in type_trees(),
        seeds in pool(),
    ) {
        let tree = resolved(&generated)?;
        let (json, _) = generate_value(&tree, &seeds, Mutation::None);
        let parsed = parse(&json, &tree).map_err(|f| TestCaseError::fail(f.to_string()))?;
        let Some((limit, path)) = tighten_first_numeric(&parsed, &tree) else {
            return Ok(());
        };
        let fault = parsed.check(&tree, &[limit]).unwrap_err();
        let is_out = matches!(fault, ValueFault::OutOfLimit { .. });
        prop_assert!(is_out);
        prop_assert_eq!(fault.leaf(), &path);
    }

    #[test]
    fn doubled_limits_fail_check_with_a_conflict(
        generated in type_trees(),
        seeds in pool(),
    ) {
        let tree = resolved(&generated)?;
        let (json, _) = generate_value(&tree, &seeds, Mutation::None);
        let parsed = parse(&json, &tree).map_err(|f| TestCaseError::fail(f.to_string()))?;
        let limits = generate_limits(&parsed, &tree, &seeds);
        let doubled: Vec<ProcessLimit> = limits.iter().chain(limits.iter()).cloned().collect();
        let result = parsed.check(&tree, &doubled);
        if limits.is_empty() {
            prop_assert_eq!(result, Ok(()));
        } else {
            let is_conflict = matches!(result, Err(ValueFault::LimitConflict { .. }));
            prop_assert!(is_conflict);
        }
    }

    #[test]
    fn leaf_of_the_wrong_json_type_fails_parse_at_that_leaf(
        generated in type_trees(),
        seeds in pool(),
        pick in any::<usize>(),
    ) {
        let tree = resolved(&generated)?;
        let (_, counts) = generate_value(&tree, &seeds, Mutation::None);
        let Some(target) = position(pick, counts.leaves) else {
            return Err(TestCaseError::fail("a tree has at least one leaf"));
        };
        let (json, _) = generate_value(&tree, &seeds, Mutation::WrongLeaf(target));
        let fault = parse(&json, &tree).unwrap_err();
        let leaves = tree.leaves();
        let expected = leaves.get(target).map(|leaf| leaf.path().clone());
        prop_assert_eq!(Some(fault.leaf().clone()), expected);
    }

    #[test]
    fn struct_with_an_extra_key_fails_parse(
        generated in type_trees(),
        seeds in pool(),
        pick in any::<usize>(),
    ) {
        let tree = resolved(&generated)?;
        let (_, Counts { structs, .. }) = generate_value(&tree, &seeds, Mutation::None);
        let Some(target) = position(pick, structs) else {
            return Ok(());
        };
        let (json, _) = generate_value(&tree, &seeds, Mutation::ExtraKey(target));
        prop_assert!(parse(&json, &tree).is_err());
    }

    #[test]
    fn array_with_a_missing_element_fails_parse(
        generated in type_trees(),
        seeds in pool(),
        pick in any::<usize>(),
    ) {
        let tree = resolved(&generated)?;
        let (_, Counts { arrays, .. }) = generate_value(&tree, &seeds, Mutation::None);
        let Some(target) = position(pick, arrays) else {
            return Ok(());
        };
        let (json, _) = generate_value(&tree, &seeds, Mutation::ShortArray(target));
        prop_assert!(parse(&json, &tree).is_err());
    }

    #[test]
    fn stored_forms_of_value_and_limits_round_trip(
        generated in type_trees(),
        seeds in pool(),
    ) {
        let tree = resolved(&generated)?;
        let (json, _) = generate_value(&tree, &seeds, Mutation::None);
        let parsed = parse(&json, &tree).map_err(|f| TestCaseError::fail(f.to_string()))?;
        let read_back = ParameterValue::try_from((&parsed.to_stored(), &tree));
        prop_assert_eq!(read_back, Ok(parsed.clone()));
        for limit in generate_limits(&parsed, &tree, &seeds) {
            let back = ProcessLimit::try_from((&limit.to_stored(), &tree));
            prop_assert_eq!(back, Ok(limit));
        }
    }

    #[test]
    fn texts_of_a_value_build_the_same_value_again(
        generated in type_trees(),
        seeds in pool(),
    ) {
        let tree = resolved(&generated)?;
        let (json, _) = generate_value(&tree, &seeds, Mutation::None);
        let parsed = parse(&json, &tree).map_err(|f| TestCaseError::fail(f.to_string()))?;
        let texts = parsed
            .texts(&tree)
            .map_err(|f| TestCaseError::fail(f.to_string()))?;
        let pairs = texts.into_iter().map(|leaf| (leaf.path, leaf.text)).collect();
        let leaf_texts = LeafTexts::parse(pairs).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let rebuilt = ParameterValue::from_texts(&tree, &leaf_texts);
        prop_assert_eq!(rebuilt, Ok(parsed));
    }
}

/// Build the value of a mixed spec from its stored form, write its texts, and build it again.
fn mixed_round_trip(spec: &MixedSpec) -> Result<(), TestCaseError> {
    let (tree, stored) = spec.build().map_err(TestCaseError::fail)?;
    let value = ParameterValue::try_from((&stored, &tree))
        .map_err(|fault| TestCaseError::fail(fault.to_string()))?;
    let texts = value
        .texts(&tree)
        .map_err(|fault| TestCaseError::fail(fault.to_string()))?;
    let pairs = texts
        .into_iter()
        .map(|leaf| (leaf.path, leaf.text))
        .collect();
    let leaf_texts =
        LeafTexts::parse(pairs).map_err(|error| TestCaseError::fail(error.to_string()))?;
    prop_assert_eq!(ParameterValue::from_texts(&tree, &leaf_texts), Ok(value));
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(MIXED_CASES))]

    #[test]
    fn texts_of_every_leaf_class_build_the_same_value_again(spec in mixed_specs()) {
        mixed_round_trip(&spec)?;
    }
}

#[test]
fn mixed_specs_reach_every_primitive() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::deterministic();
    let strategy = mixed_specs();
    let mut reached = BTreeSet::new();
    for _ in 0..300 {
        let spec = strategy.new_tree(&mut runner).unwrap().current();
        reached.extend(spec.primitives());
    }
    let every: BTreeSet<Primitive> = Primitive::ALL.into_iter().collect();
    assert_eq!(reached, every);
}

#[test]
fn generators_reach_structs_arrays_and_limits() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::deterministic();
    let strategy = (type_trees(), pool());
    let (mut with_struct, mut with_array, mut with_limits) = (0_u32, 0_u32, 0_u32);
    for _ in 0..100 {
        let (generated, seeds) = strategy.new_tree(&mut runner).unwrap().current();
        let tree = TypeTree::resolve(generated.root, &generated.types).unwrap();
        let (json, counts) = generate_value(&tree, &seeds, Mutation::None);
        let parsed = parse(&json, &tree).unwrap();
        with_struct = with_struct
            .checked_add(u32::from(counts.structs > 0))
            .unwrap();
        with_array = with_array
            .checked_add(u32::from(counts.arrays > 0))
            .unwrap();
        let has_limits = !generate_limits(&parsed, &tree, &seeds).is_empty();
        with_limits = with_limits.checked_add(u32::from(has_limits)).unwrap();
    }
    assert!(with_struct >= 10, "{with_struct} trees with a struct");
    assert!(with_array >= 10, "{with_array} trees with an array");
    assert!(with_limits >= 10, "{with_limits} trees with a limit");
}
