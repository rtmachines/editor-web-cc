//! Tests of the limit rows of a sent form and of the switch of a limit mode (ADR-0039 (k)).
//!
//! A forged form can send any leaf path. The set keeps only the rows that the type tree holds, and
//! a switch copies at most one row per leaf of the tree.

use recipe_domain::plant::{LeafPath, Primitive, TypeTree};

use super::limits::{LimitSet, LimitTexts};
use super::{GridRow, GridSource, GridView, ValueTexts};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::parameters::tree_kit::{Kit, array_tree, zones_tree};

/// Build one limit row of texts.
fn limit(path: &str, min: &str, max: &str) -> LimitTexts {
    LimitTexts {
        path: LeafPath::parse(path).unwrap(),
        min: min.to_owned(),
        max: max.to_owned(),
        max_length: String::new(),
    }
}

/// Build the English grid of `tree` with the sent limit rows and the pressed level `toggle`.
fn grid(tree: &TypeTree, limits: Vec<LimitTexts>, toggle: Option<&str>) -> GridView {
    let bundles = Bundles::load().unwrap();
    let level = toggle.map(|text| LeafPath::parse(text).unwrap());
    let source = GridSource {
        tree,
        values: ValueTexts::default(),
        limits,
        toggle: level.as_ref(),
        chunk: 100,
        root_label: "SPEED".to_owned(),
    };
    GridView::build(source, Texts::new(&bundles, Lang::En))
}

/// Return the current mode sentence of each level of the grid.
fn currents(view: &GridView) -> Vec<String> {
    view.bodies
        .iter()
        .flat_map(|body| &body.rows)
        .filter_map(GridRow::mode)
        .map(|mode| mode.current.clone())
        .collect()
}

/// Return true when the grid shows limit inputs for the shown path `path`.
fn has_limit_inputs(view: &GridView, path: &str) -> bool {
    view.bodies
        .iter()
        .flat_map(|body| &body.rows)
        .filter_map(GridRow::leaf)
        .any(|row| row.path == path && row.min.is_some())
}

/// Resolve `ARRAY[1..1024] OF ST_S3`. `ST_S3.x` is `ARRAY[1..2] OF ST_S2`, `ST_S2.w` is
/// `ARRAY[1..2] OF ST_S1`, and `ST_S1.v` is `ARRAY[1..2] OF INT`. The tree has 8 192 leaves.
fn deep_tree() -> TypeTree {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let v = kit.array(int, &[(1, 2)]);
    let s1 = kit.structure("ST_S1", &[("v", v)]);
    let w = kit.array(s1, &[(1, 2)]);
    let s2 = kit.structure("ST_S2", &[("w", w)]);
    let x = kit.array(s2, &[(1, 2)]);
    let s3 = kit.structure("ST_S3", &[("x", x)]);
    let root = kit.array(s3, &[(1, 1024)]);
    kit.tree(root)
}

/// Return the 27 tails under one element of `deep_tree`: each level is `[*]`, `[1]`, or `[2]`.
fn deep_tails() -> Vec<String> {
    const CHOICES: [&str; 3] = ["[*]", "[1]", "[2]"];
    let mut tails = Vec::new();
    for x in CHOICES {
        for w in CHOICES {
            for v in CHOICES {
                tails.push(format!(".x{x}.w{w}.v{v}"));
            }
        }
    }
    tails
}

#[test]
fn grid_limits_a_sent_row_outside_the_tree_is_dropped() {
    let tree = array_tree(Primitive::Int, &[(1, 1000)]);
    let rows = vec![limit("[*]", "0", "10"), limit("[*].x", "1", "2")];
    let set = LimitSet::new(&tree, rows, None);
    assert!(set.get("[*]").is_some());
    assert!(set.get("[*].x").is_none());
}

#[test]
fn grid_limits_a_split_copies_only_the_rows_of_the_tree() {
    let tree = array_tree(Primitive::Int, &[(1, 1000)]);
    let rows = vec![
        limit("[*]", "0", "10"),
        limit("[*].x", "1", "2"),
        limit("[*][1]", "1", "2"),
    ];
    let level = LeafPath::parse("[*]").unwrap();
    let set = LimitSet::new(&tree, rows, Some(&level));
    assert_eq!(set.get("[5]").map(|row| row.max.as_str()), Some("10"));
    assert!(set.get("[5].x").is_none());
    assert!(set.get("[5][1]").is_none());
}

#[test]
fn grid_limits_a_forged_index_row_outside_the_tree_keeps_the_mode() {
    let rows = vec![
        limit("[*].setpoint", "0", "10"),
        limit("[3].nothing", "1", "2"),
    ];
    let view = grid(&zones_tree(), rows, None);
    assert_eq!(currents(&view), ["[*]: one limit for all elements"]);
}

#[test]
fn grid_limits_a_split_makes_at_most_one_row_per_leaf() {
    let tree = deep_tree();
    let rows = deep_tails()
        .iter()
        .map(|tail| limit(&format!("[*]{tail}"), "0", "1"))
        .collect();
    let level = LeafPath::parse("[*]").unwrap();
    let set = LimitSet::new(&tree, rows, Some(&level));
    let tails = deep_tails();
    let kept = (1..=1024)
        .flat_map(|index| tails.iter().map(move |tail| format!("[{index}]{tail}")))
        .filter(|path| set.get(path).is_some())
        .count();
    assert!(kept > 0);
    assert!(kept <= 8192, "{kept}");
}

#[test]
fn grid_limits_a_join_after_an_empty_split_gives_one_limit_for_all() {
    let rows = (1..=4)
        .flat_map(|index| {
            [
                limit(&format!("[{index}].setpoint"), "", ""),
                limit(&format!("[{index}].hold"), " ", ""),
            ]
        })
        .collect();
    let view = grid(&zones_tree(), rows, Some("[*]"));
    assert_eq!(currents(&view), ["[*]: one limit for all elements"]);
    assert!(has_limit_inputs(&view, "[*].setpoint"));
    assert!(!has_limit_inputs(&view, "[2].setpoint"));
}

#[test]
fn grid_limits_empty_element_rows_keep_a_limit_per_element() {
    let rows = (1..=4)
        .map(|index| limit(&format!("[{index}].setpoint"), "", ""))
        .collect();
    let view = grid(&zones_tree(), rows, None);
    assert_eq!(currents(&view), ["[*]: a limit per element"]);
    assert!(has_limit_inputs(&view, "[2].setpoint"));
}
