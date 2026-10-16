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
    LimitTexts {
        path: LeafPath::parse(path).unwrap(),
        min: "0".to_owned(),
        max: "1".to_owned(),
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

#[test]
fn grid_index_levels_nested_all_rows_and_index_rows_give_the_mode_of_each_level() {
    let rows = vec![limit("[*].x[2].w[*].v[1]"), limit("[3].x[*].w[1].v[*]")];
    let set = LimitSet::new(&nested_tree(), rows, None);
    let cases = [
        ("[*]", true),
        ("[*].x[*]", true),
        ("[3].x[*]", false),
        ("[3].x[*].w[*]", true),
        ("[*].x[2].w[*]", false),
        ("[*].x[2].w[*].v[*]", true),
        ("[3].x[*].w[1].v[*]", false),
        ("[1].x[*]", false),
    ];
    for (level, each) in cases {
        assert_eq!(set.is_each(level), each, "level: {level}");
    }
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
