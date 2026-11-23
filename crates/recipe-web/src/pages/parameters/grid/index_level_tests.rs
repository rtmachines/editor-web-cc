//! Tests of the levels that hold an index row, and of the work of the mode check (ADR-0039 (k)).
//!
//! A forged form can send more limit rows than the tree has leaves, because the rows of `[*]` and
//! of each index resolve. The mode check of a level must not scan every row for each array node.

use recipe_domain::plant::{LeafPath, Primitive, TypeTree};

use super::limits::{LimitSet, LimitTexts};
use crate::pages::parameters::tree_kit::Kit;

/// The segments of one level of `nested_tree`: every element, or one index.
const CHOICES: [&str; 5] = ["[*]", "[1]", "[2]", "[3]", "[4]"];

/// Build one limit row of texts.
fn limit(path: &str) -> LimitTexts {
    bounded(path, "0", "1")
}

/// Build one limit row of the bounds `min` to `max`.
fn bounded(path: &str, min: &str, max: &str) -> LimitTexts {
    LimitTexts {
        path: LeafPath::parse(path).unwrap(),
        min: min.to_owned(),
        max: max.to_owned(),
        max_length: String::new(),
    }
}

/// Resolve `ARRAY[1..4] OF ST_S3`. `ST_S3.x` is `ARRAY[1..4] OF ST_S2`, `ST_S2.w` is
/// `ARRAY[1..4] OF ST_S1`, and `ST_S1.v` is `ARRAY[1..4] OF INT`. The tree has 256 leaves.
fn nested_tree() -> TypeTree {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let v = kit.array(int, &[(1, 4)]);
    let s1 = kit.structure("ST_S1", &[("v", v)]);
    let w = kit.array(s1, &[(1, 4)]);
    let s2 = kit.structure("ST_S2", &[("w", w)]);
    let x = kit.array(s2, &[(1, 4)]);
    let s3 = kit.structure("ST_S3", &[("x", x)]);
    let root = kit.array(s3, &[(1, 4)]);
    kit.tree(root)
}

/// Return the 625 leaf paths of `nested_tree` with `[*]` or an index at each level.
fn every_row() -> Vec<LimitTexts> {
    let mut rows = Vec::new();
    for r in CHOICES {
        for x in CHOICES {
            for w in CHOICES {
                for v in CHOICES {
                    rows.push(limit(&format!("{r}.x{x}.w{w}.v{v}")));
                }
            }
        }
    }
    rows
}

/// Return the 85 levels that a value grid of `nested_tree` checks when each level has a limit
/// per element: one level for each array node.
fn walk_levels() -> Vec<String> {
    let indexes = ["[1]", "[2]", "[3]", "[4]"];
    let mut levels = vec!["[*]".to_owned()];
    for r in indexes {
        levels.push(format!("{r}.x[*]"));
        for x in indexes {
            levels.push(format!("{r}.x{x}.w[*]"));
            for w in indexes {
                levels.push(format!("{r}.x{x}.w{w}.v[*]"));
            }
        }
    }
    levels
}

#[test]
fn grid_index_levels_the_mode_check_does_not_scan_every_row_for_each_level() {
    let tree = nested_tree();
    let set = LimitSet::new(&tree, every_row(), None);
    let levels = walk_levels();
    assert_eq!(levels.len(), 85);
    assert!(levels.iter().all(|level| set.is_each(level)));
    let leaves = usize::try_from(tree.leaf_count()).unwrap();
    assert!(set.checks() <= leaves, "{} checks", set.checks());
}

/// Assert the mode of each level of `cases` for the limit rows `rows` of the nested tree.
fn assert_modes(rows: Vec<LimitTexts>, cases: &[(&str, bool)]) {
    let set = LimitSet::new(&nested_tree(), rows, None);
    for (level, each) in cases {
        assert_eq!(set.is_each(level), *each, "level: {level}");
    }
}

#[test]
fn grid_index_levels_nested_all_rows_and_index_rows_give_the_mode_of_each_level() {
    let rows = vec![limit("[*].x[2].w[*].v[1]")];
    let cases = [
        ("[*]", false),
        ("[*].x[*]", true),
        ("[*].x[2].w[*]", false),
        ("[*].x[2].w[*].v[*]", true),
        ("[1].x[*]", false),
    ];
    assert_modes(rows, &cases);
}

#[test]
fn grid_index_levels_nested_index_rows_below_an_outer_index_give_the_mode_of_each_level() {
    let rows = vec![limit("[3].x[*].w[1].v[*]")];
    let cases = [
        ("[*]", true),
        ("[3].x[*]", false),
        ("[3].x[*].w[*]", true),
        ("[3].x[*].w[1].v[*]", false),
        ("[1].x[*]", false),
    ];
    assert_modes(rows, &cases);
}

#[test]
fn grid_index_levels_a_split_gives_the_mode_of_the_copied_rows() {
    let rows = vec![limit("[*].x[2].w[*].v[*]")];
    let toggle = LeafPath::parse("[*]").unwrap();
    let set = LimitSet::new(&nested_tree(), rows, Some(&toggle));
    let cases = [
        ("[*]", true),
        ("[1].x[*]", true),
        ("[4].x[*]", true),
        ("[*].x[*]", false),
        ("[1].x[2].w[*]", false),
    ];
    for (level, each) in cases {
        assert_eq!(set.is_each(level), each, "level: {level}");
    }
}

#[test]
fn grid_index_levels_a_join_removes_the_mode_of_the_joined_rows() {
    let rows = (1..=4)
        .map(|index| limit(&format!("[{index}].x[2].w[*].v[*]")))
        .collect();
    let toggle = LeafPath::parse("[*]").unwrap();
    let set = LimitSet::new(&nested_tree(), rows, Some(&toggle));
    let cases = [("[*]", false), ("[*].x[*]", true), ("[1].x[*]", false)];
    for (level, each) in cases {
        assert_eq!(set.is_each(level), each, "level: {level}");
    }
}

#[test]
fn grid_index_levels_a_switch_of_an_inner_type_level_gives_the_mode_of_each_array_level() {
    let mut rows = vec![limit("[1].x[1].w[*].v[*]")];
    rows.extend((1..=4).map(|index| limit(&format!("[2].x[{index}].w[*].v[*]"))));
    let toggle = LeafPath::parse("[*].x[*]").unwrap();
    let set = LimitSet::new(&nested_tree(), rows, Some(&toggle));
    let cases = [
        ("[*]", true),
        ("[1].x[*]", false),
        ("[2].x[*]", false),
        ("[2].x[*].w[*]", false),
        ("[2].x[1].w[*]", false),
    ];
    for (level, each) in cases {
        assert_eq!(set.is_each(level), each, "level: {level}");
    }
    assert!(set.get("[2].x[*].w[*].v[*]").is_some());
    assert!(set.get("[1].x[*].w[*].v[*]").is_none());
    assert_eq!(set.unequal_count("[*].x[*]"), 1);
    assert_eq!(set.unequal_count("[*]"), 0);
}

/// The bound of the row visits of a switch: two passes over the 2^14 limit rows.
const VISIT_BOUND: usize = 2 * (1 << 14);

/// Resolve `ARRAY[1..1024] OF ARRAY[1..16] OF INT`. The tree has 2^14 leaves and 2^10 array
/// levels at the type level `[*][*]`.
fn wide_tree() -> TypeTree {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let inner = kit.array(int, &[(1, 16)]);
    let root = kit.array(inner, &[(1, 1024)]);
    kit.tree(root)
}

#[test]
fn grid_index_levels_a_switch_back_of_a_type_level_makes_at_most_two_visits_for_each_row() {
    let rows = (1..=1024)
        .flat_map(|outer| (1..=16).map(move |inner| (outer, inner)))
        .map(|(outer, inner)| {
            let max = if (outer, inner) == (5, 3) { 99 } else { outer };
            bounded(&format!("[{outer}][{inner}]"), "0", &max.to_string())
        })
        .collect();
    let toggle = LeafPath::parse("[*][*]").unwrap();
    let set = LimitSet::new(&wide_tree(), rows, Some(&toggle));
    assert_eq!(set.get("[7][*]").map(|row| row.max.as_str()), Some("7"));
    assert!(set.get("[7][3]").is_none());
    assert!(set.get("[5][*]").is_none());
    assert_eq!(set.unequal_count("[*][*]"), 1);
    assert!(!set.is_each("[7][*]"));
    assert!(set.visits() <= VISIT_BOUND, "{} visits", set.visits());
}

#[test]
fn grid_index_levels_a_switch_to_a_limit_per_element_of_a_type_level_makes_at_most_two_visits() {
    let rows = (1..=1024)
        .map(|outer| bounded(&format!("[{outer}][*]"), "0", &outer.to_string()))
        .collect();
    let toggle = LeafPath::parse("[*][*]").unwrap();
    let set = LimitSet::new(&wide_tree(), rows, Some(&toggle));
    assert_eq!(set.get("[7][16]").map(|row| row.max.as_str()), Some("7"));
    assert!(set.get("[7][*]").is_none());
    assert!(set.is_each("[7][*]"));
    assert!(set.visits() <= VISIT_BOUND, "{} visits", set.visits());
}
