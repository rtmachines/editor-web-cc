//! Tests of `rebind_new_leaves`: each list equals the leaves that the plan asks values for.

use serde_json::{Value, json};

use super::rebind_new_leaves;
use crate::plant::fixtures::id;
use crate::plant::{Primitive, TypeTrees};
use crate::recipe::rebind::test_support::{
    INT, MODE_1, MODE_2, NAME_1, NAME_2, OLD_1, OLD_2, RAMP_1, ZONE_2, ZONE_3, ZONES_2, draft_with,
    param, request, trees, trees_with_zone_3,
};
use crate::recipe::{DraftError, RecipeError};

/// Return the paths of `rebind_new_leaves` for a parameter of type `type_n`.
fn new_paths(trees: &TypeTrees, type_n: u128, (from, to): (u128, u128)) -> Option<Vec<String>> {
    let tree = trees.tree(id(type_n)).unwrap();
    let target = trees.tree(id(to)).unwrap();
    rebind_new_leaves(tree, id(from), target)
        .unwrap()
        .map(|found| {
            found
                .leaves()
                .iter()
                .map(|leaf| leaf.path().to_string())
                .collect()
        })
}

/// Return the leaves of the `E-RCP-028` entries of a plan without values, in the order of the list.
fn needed_paths(
    trees: &TypeTrees,
    (type_n, value): (u128, Value),
    (from, to): (u128, u128),
) -> Vec<String> {
    let draft = draft_with(vec![param(1, "P", type_n, value, Vec::new())]);
    match draft.plan_rebind(&request(from, to), trees) {
        Ok(_plan) => Vec::new(),
        Err(DraftError::List(list)) => list
            .into_vec()
            .into_iter()
            .map(|entry| match entry {
                RecipeError::RebindNeedsValue { leaf, .. } => leaf.to_string(),
                other => format!("unexpected {other}"),
            })
            .collect(),
        Err(other) => vec![format!("unexpected {other:?}")],
    }
}

/// Assert that the new leaves are `expected` and equal the leaves that the plan needs.
fn assert_rule(
    trees: &TypeTrees,
    (type_n, value): (u128, Value),
    pair: (u128, u128),
    expected: &[&str],
) {
    let found = new_paths(trees, type_n, pair).unwrap();
    assert_eq!(found, expected);
    assert_eq!(needed_paths(trees, (type_n, value), pair), expected);
}

#[test]
fn rebind_new_leaves_of_a_flat_struct_are_its_new_member() {
    assert_rule(
        &trees(),
        (ZONE_2, json!({"setpoint": 5})),
        (ZONE_2, ZONE_3),
        &[".temp_max"],
    );
}

#[test]
fn rebind_new_leaves_of_an_array_of_structs_come_element_by_element() {
    let value = json!([{"setpoint": 1}, {"setpoint": 2}]);
    assert_rule(
        &trees(),
        (ZONES_2, value),
        (ZONE_2, ZONE_3),
        &["[1].temp_max", "[2].temp_max"],
    );
}

#[test]
fn rebind_new_leaves_hold_a_retyped_member() {
    let value = json!({"setpoint": 1, "mode": 1});
    assert_rule(&trees(), (MODE_1, value), (MODE_1, MODE_2), &[".mode"]);
}

#[test]
fn rebind_new_leaves_hold_a_renamed_member() {
    let trees = trees_with_zone_3(&[("set_point", INT)]);
    assert_rule(
        &trees,
        (ZONE_2, json!({"setpoint": 5})),
        (ZONE_2, ZONE_3),
        &[".set_point"],
    );
}

#[test]
fn rebind_new_leaves_hold_each_leaf_of_a_new_struct_member() {
    let trees = trees_with_zone_3(&[("setpoint", INT), ("ramp", RAMP_1)]);
    let expected = [
        ".ramp.setpoint",
        ".ramp.ramps[1]",
        ".ramp.ramps[2]",
        ".ramp.ramps[3]",
    ];
    assert_rule(
        &trees,
        (ZONE_2, json!({"setpoint": 5})),
        (ZONE_2, ZONE_3),
        &expected,
    );
}

#[test]
fn rebind_new_leaves_of_a_dropped_member_only_are_empty() {
    let value = json!({"setpoint": 1, "old_max": 1});
    assert_rule(&trees(), (OLD_1, value), (OLD_1, OLD_2), &[]);
}

#[test]
fn rebind_new_leaves_describe_a_string_member() {
    let trees = trees();
    let tree = trees.tree(id(NAME_1)).unwrap();
    let found = rebind_new_leaves(tree, id(NAME_1), trees.tree(id(NAME_2)).unwrap())
        .unwrap()
        .unwrap();
    let leaf = found.leaves().first().unwrap();
    assert_eq!(leaf.path().to_string(), ".label");
    assert_eq!(leaf.target().primitive(), Primitive::String);
    assert_eq!(leaf.target().string_length(), Some(10));
    assert_eq!(
        found.tree(),
        &tree
            .rebind(id(NAME_1), trees.tree(id(NAME_2)).unwrap())
            .unwrap()
            .unwrap()
    );
}

#[test]
fn rebind_new_leaves_of_a_type_that_the_rebind_does_not_move_are_none() {
    assert_eq!(new_paths(&trees(), INT, (ZONE_2, ZONE_3)), None);
}
