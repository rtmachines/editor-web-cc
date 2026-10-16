//! Tests of the groups of a nested array in the leaf grid (task 17.1, ADR-0045).
//!
//! Only an array that does not stand inside a group splits into groups. An array inside a group
//! shows all its rows inside that group and has no toggle of its own.

use recipe_domain::plant::{LeafPath, Primitive, TypeTree};

use super::grid::{GridBody, GridRow, GridSource, GridView, LimitTexts, ValueTexts};
use super::tree_kit::Kit;
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;

/// The size of a group in the tests: `form.array_chunk`.
const CHUNK: usize = 100;

/// Build the grid of `tree` in English with the limit rows `limits`.
fn grid(tree: &TypeTree, limits: Vec<LimitTexts>) -> GridView {
    let bundles = Bundles::load().unwrap();
    let source = GridSource {
        tree,
        values: ValueTexts::default(),
        limits,
        toggle: None,
        chunk: CHUNK,
        root_label: "ZONES".to_owned(),
    };
    GridView::build(source, Texts::new(&bundles, Lang::En))
}

/// Return the tree `ARRAY[1..outer] OF ST_Cell`, where `ST_Cell` holds `arr` of type
/// `ARRAY[1..inner] OF INT`.
fn cells_tree(outer: i32, inner: i32) -> TypeTree {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let arr = kit.array(int, &[(1, inner)]);
    let cell = kit.structure("ST_Cell", &[("arr", arr)]);
    let root = kit.array(cell, &[(1, outer)]);
    kit.tree(root)
}

/// Return the bodies of the groups of the grid.
fn groups(view: &GridView) -> Vec<&GridBody> {
    view.bodies
        .iter()
        .filter(|body| !body.id.is_empty())
        .collect()
}

/// Return the shown path of each leaf row and row `[*]` of one body.
fn body_paths(body: &GridBody) -> Vec<String> {
    body.rows
        .iter()
        .filter_map(GridRow::leaf)
        .map(|row| row.path.clone())
        .collect()
}

/// Return the texts of every toggle row of the grid.
fn toggle_texts(view: &GridView) -> Vec<String> {
    view.bodies
        .iter()
        .flat_map(|body| &body.rows)
        .filter_map(GridRow::toggle)
        .map(|toggle| toggle.text.clone())
        .collect()
}

#[test]
fn grid_an_array_inside_a_group_shows_all_its_rows_in_that_group() {
    let view = grid(&cells_tree(101, 101), Vec::new());
    let groups = groups(&view);
    let ids: Vec<&str> = groups.iter().map(|body| body.id.as_str()).collect();
    assert_eq!(ids, ["grid-group-0", "grid-group-1"]);
    let first = body_paths(groups.first().unwrap());
    let mut expected = Vec::new();
    for outer in 1..=100 {
        for inner in 1..=101 {
            expected.push(format!("[{outer}].arr[{inner}]"));
        }
    }
    assert_eq!(first, expected);
    let last = body_paths(groups.last().unwrap());
    let expected: Vec<String> = (1..=101)
        .map(|inner| format!("[101].arr[{inner}]"))
        .collect();
    assert_eq!(last, expected);
    let inside_toggles = groups
        .iter()
        .flat_map(|body| &body.rows)
        .filter_map(GridRow::toggle)
        .count();
    assert_eq!(inside_toggles, 0);
    assert_eq!(
        toggle_texts(&view),
        [
            "Elements [1]..[100] of [*] (elements: 100)",
            "Elements [101]..[101] of [*] (elements: 1)"
        ]
    );
}

#[test]
fn grid_an_array_member_of_a_struct_outside_a_group_splits_into_groups() {
    let view = grid(&cells_tree(2, 101), Vec::new());
    let shape: Vec<(&str, bool, usize, Option<String>)> = groups(&view)
        .into_iter()
        .map(|body| {
            let paths = body_paths(body);
            (
                body.id.as_str(),
                body.hidden,
                paths.len(),
                paths.first().cloned(),
            )
        })
        .collect();
    let first = |path: &str| Some(path.to_owned());
    assert_eq!(
        shape,
        [
            ("grid-group-0", false, 100, first("[1].arr[1]")),
            ("grid-group-1", true, 1, first("[1].arr[101]")),
            ("grid-group-2", false, 100, first("[2].arr[1]")),
            ("grid-group-3", true, 1, first("[2].arr[101]")),
        ]
    );
    assert_eq!(
        toggle_texts(&view).first().unwrap(),
        "Elements [1]..[100] of [*].arr[*] (elements: 100)"
    );
}

#[test]
fn grid_the_row_all_and_the_mode_row_of_a_nested_level_stand_inside_the_group() {
    let limits = vec![LimitTexts {
        path: LeafPath::parse("[1].arr[*]").unwrap(),
        min: "1".to_owned(),
        max: "5".to_owned(),
        max_length: String::new(),
    }];
    let view = grid(&cells_tree(101, 101), limits);
    let groups = groups(&view);
    let first = groups.first().unwrap();
    let all = first
        .rows
        .iter()
        .filter_map(GridRow::leaf)
        .find(|row| row.path == "[1].arr[*]")
        .unwrap();
    let bounds = all.min.as_ref().zip(all.max.as_ref());
    let bounds = bounds.map(|(min, max)| (min.text.as_str(), max.text.as_str()));
    assert_eq!(bounds, Some(("1", "5")));
    let modes: Vec<&str> = first
        .rows
        .iter()
        .filter_map(GridRow::mode)
        .map(|mode| mode.level.as_str())
        .take(2)
        .collect();
    assert_eq!(modes, ["[1].arr[*]", "[2].arr[*]"]);
    assert!(first.rows.iter().all(|row| row.toggle().is_none()));
    assert_eq!(groups.len(), 2);
}
