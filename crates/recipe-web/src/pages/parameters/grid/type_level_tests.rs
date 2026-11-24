//! Tests of the mode rows of the type levels in the leaf grid (ADR-0223, design D2).
//!
//! A type level is the path of an array level with `[*]` for each index. The grid shows one mode
//! row for each type level, at the place where the walk meets the type level for the first time.
use super::{GridRow, GridSource, GridView, LimitTexts, ValueTexts};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::parameters::tree_kit::Kit;
use recipe_domain::plant::{LeafPath, Primitive, TypeTree};
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
        root_label: "NEST".to_owned(),
    };
    GridView::build(source, Texts::new(&bundles, Lang::En))
}
/// Build the grid of `tree` in English with the limit rows `limits` and the pressed type level.
fn switched(tree: &TypeTree, limits: Vec<LimitTexts>, level: &str) -> GridView {
    let bundles = Bundles::load().unwrap();
    let level = LeafPath::parse(level).unwrap();
    let source = GridSource {
        tree,
        values: ValueTexts::default(),
        limits,
        toggle: Some(&level),
        chunk: CHUNK,
        root_label: "NEST".to_owned(),
    };
    GridView::build(source, Texts::new(&bundles, Lang::En))
}
/// Build one limit row of the bounds 0 to 100.
fn limit(path: &str) -> LimitTexts {
    bounded(path, "0", "100")
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
/// Return the shown path and the bounds of each row that has limit inputs, in row order.
fn input_rows(view: &GridView) -> Vec<(String, String, String)> {
    view.bodies
        .iter()
        .flat_map(|body| body.rows.iter())
        .filter_map(GridRow::leaf)
        .filter_map(|row| {
            let (min, max) = (row.min.as_ref()?, row.max.as_ref()?);
            Some((row.path.clone(), min.text.clone(), max.text.clone()))
        })
        .collect()
}
/// Build the expected entry of `input_rows`.
fn input(path: &str, min: &str, max: &str) -> (String, String, String) {
    (path.to_owned(), min.to_owned(), max.to_owned())
}
/// Return the notice of the mode row of the type level `level`.
fn notice_of(view: &GridView, level: &str) -> String {
    view.bodies
        .iter()
        .flat_map(|body| body.rows.iter())
        .filter_map(GridRow::mode)
        .find(|mode| mode.level == level)
        .map(|mode| mode.notice.clone())
        .unwrap()
}
/// Build the limit rows of the paths `paths`.
fn limits(paths: &[&str]) -> Vec<LimitTexts> {
    paths.iter().map(|path| limit(path)).collect()
}
/// Return the tree `ARRAY[1..2] OF ARRAY[1..3] OF INT`.
fn nest_tree() -> TypeTree {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let inner = kit.array(int, &[(1, 3)]);
    let root = kit.array(inner, &[(1, 2)]);
    kit.tree(root)
}
/// Return the level path of each mode row of the grid, in the order of the rows.
fn mode_levels(view: &GridView) -> Vec<String> {
    view.bodies
        .iter()
        .flat_map(|body| body.rows.iter())
        .filter_map(GridRow::mode)
        .map(|mode| mode.level.clone())
        .collect()
}
/// Return the sentence of each mode row that names the current mode.
fn mode_currents(view: &GridView) -> Vec<String> {
    view.bodies
        .iter()
        .flat_map(|body| body.rows.iter())
        .filter_map(GridRow::mode)
        .map(|mode| mode.current.clone())
        .collect()
}
/// Return the shown path of each leaf row, in the order of the rows.
fn leaf_paths(view: &GridView) -> Vec<String> {
    view.bodies
        .iter()
        .flat_map(|body| body.rows.iter())
        .filter_map(GridRow::leaf)
        .map(|row| row.path.clone())
        .collect()
}
/// Return the sorted level paths of the mode rows. The order of two mode rows does not matter.
fn sorted_levels(view: &GridView) -> Vec<String> {
    let mut levels = mode_levels(view);
    levels.sort();
    levels
}
#[test]
fn grid_a_nested_array_with_a_limit_per_element_has_one_mode_row_for_each_type_level() {
    let paths = ["[1][1]", "[1][2]", "[1][3]", "[2][1]", "[2][2]", "[2][3]"];
    let view = grid(&nest_tree(), limits(&paths));
    assert_eq!(mode_levels(&view), ["[*]", "[*][*]"]);
    assert_eq!(
        mode_currents(&view),
        ["[*]: a limit per element", "[*][*]: a limit per element"]
    );
    assert_eq!(leaf_paths(&view), paths);
}
#[test]
fn grid_a_nested_array_with_one_limit_for_all_has_one_mode_row_for_each_type_level() {
    let view = grid(&nest_tree(), Vec::new());
    assert_eq!(sorted_levels(&view), ["[*]", "[*][*]"]);
    let currents = mode_currents(&view);
    assert!(
        currents
            .iter()
            .all(|text| text.ends_with("one limit for all elements"))
    );
}
#[test]
fn grid_the_type_level_of_a_member_array_has_one_mode_row_for_all_the_elements_of_the_outer_array()
{
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let arr = kit.array(int, &[(1, 3)]);
    let cell = kit.structure("ST_Cell", &[("arr", arr)]);
    let root = kit.array(cell, &[(1, 3)]);
    let tree = kit.tree(root);
    let paths: Vec<String> = (1..=3)
        .flat_map(|outer| (1..=3).map(move |inner| format!("[{outer}].arr[{inner}]")))
        .collect();
    let rows: Vec<&str> = paths.iter().map(String::as_str).collect();
    let view = grid(&tree, limits(&rows));
    assert_eq!(mode_levels(&view), ["[*]", "[*].arr[*]"]);
    assert_eq!(leaf_paths(&view), paths);
}
#[test]
fn grid_a_two_dimensional_outer_array_gives_one_type_level_with_one_segment() {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let inner = kit.array(int, &[(1, 2)]);
    let root = kit.array(inner, &[(1, 2), (1, 2)]);
    let tree = kit.tree(root);
    let paths = [
        "[1,1][1]", "[1,1][2]", "[1,2][1]", "[1,2][2]", "[2,1][1]", "[2,1][2]", "[2,2][1]",
        "[2,2][2]",
    ];
    let view = grid(&tree, limits(&paths));
    assert_eq!(mode_levels(&view), ["[*]", "[*][*]"]);
    assert_eq!(leaf_paths(&view), paths);
}
#[test]
fn grid_a_type_level_with_mixed_modes_still_has_one_mode_row() {
    let paths = ["[1][*]", "[2][1]", "[2][2]", "[2][3]"];
    let view = grid(&nest_tree(), limits(&paths));
    assert_eq!(sorted_levels(&view), ["[*]", "[*][*]"]);
}

/// The notice of a type level that had mixed modes.
const MIXED_NOTICE: &str = "The elements of [*][*] had both modes. The form shows a limit per \
     element. Save the form to store the copied limits.";
/// Return the limit rows of a mixed type level: `[1][*]` of 0 to 100, and `[2][i]` of 0 to 5.
fn mixed_rows() -> Vec<LimitTexts> {
    let mut rows = vec![limit("[1][*]")];
    rows.extend(["[2][1]", "[2][2]", "[2][3]"].map(|path| bounded(path, "0", "5")));
    rows
}

#[test]
fn grid_mixed_modes_copy_the_all_limit_to_the_element_rows() {
    let view = grid(&nest_tree(), mixed_rows());
    assert_eq!(
        input_rows(&view),
        [
            input("[1][1]", "0", "100"),
            input("[1][2]", "0", "100"),
            input("[1][3]", "0", "100"),
            input("[2][1]", "0", "5"),
            input("[2][2]", "0", "5"),
            input("[2][3]", "0", "5"),
        ]
    );
    assert_eq!(
        mode_currents(&view),
        ["[*]: a limit per element", "[*][*]: a limit per element"]
    );
}

#[test]
fn grid_mixed_modes_do_not_expand_a_blank_all_row() {
    let rows = vec![
        bounded("[1][*]", "0", "10"),
        bounded("[2][*]", "", ""),
        bounded("[3][1]", "0", "20"),
    ];
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let inner = kit.array(int, &[(1, 2)]);
    let root = kit.array(inner, &[(1, 3)]);
    let view = grid(&kit.tree(root), rows);
    let inputs = input_rows(&view);
    assert!(inputs.contains(&input("[1][1]", "0", "10")));
    assert!(inputs.contains(&input("[1][2]", "0", "10")));
    assert!(inputs.contains(&input("[3][1]", "0", "20")));
    assert!(inputs.contains(&input("[2][*]", "", "")));
    assert!(
        inputs
            .iter()
            .all(|(path, _min, _max)| !path.starts_with("[2][1") && !path.starts_with("[2][2"))
    );
}

#[test]
fn grid_the_switch_to_a_limit_per_element_expands_a_blank_all_row() {
    let rows = vec![bounded("[1][*]", "0", "10"), bounded("[2][*]", "", "")];
    let view = switched(&nest_tree(), rows, "[*][*]");
    let inputs = input_rows(&view);
    assert!(inputs.contains(&input("[2][1]", "", "")));
    assert!(inputs.contains(&input("[2][3]", "", "")));
}

#[test]
fn grid_mixed_modes_show_one_notice_that_names_the_type_level() {
    let view = grid(&nest_tree(), mixed_rows());
    assert_eq!(notice_of(&view, "[*][*]"), MIXED_NOTICE);
    assert_eq!(notice_of(&view, "[*]"), "");
}

#[test]
fn grid_mixed_modes_of_a_member_type_level_copy_below_an_outer_all_level() {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let inner = kit.array(int, &[(1, 2)]);
    let outer = kit.array(inner, &[(1, 2)]);
    let cell = kit.structure("ST_Cell", &[("grid", outer)]);
    let root = kit.array(cell, &[(1, 2)]);
    let tree = kit.tree(root);
    let rows = vec![
        limit("[*].grid[1][*]"),
        bounded("[*].grid[2][1]", "0", "5"),
        bounded("[*].grid[2][2]", "0", "5"),
    ];
    let view = grid(&tree, rows);
    assert_eq!(
        input_rows(&view),
        [
            input("[*].grid[1][1]", "0", "100"),
            input("[*].grid[1][2]", "0", "100"),
            input("[*].grid[2][1]", "0", "5"),
            input("[*].grid[2][2]", "0", "5"),
        ]
    );
    assert!(notice_of(&view, "[*].grid[*][*]").contains("had both modes"));
}

#[test]
fn grid_without_mixed_modes_shows_no_mixed_notice() {
    let each = ["[1][1]", "[1][2]", "[1][3]", "[2][1]", "[2][2]", "[2][3]"];
    for rows in [limits(&each), limits(&["[1][*]", "[2][*]"]), Vec::new()] {
        let view = grid(&nest_tree(), rows);
        assert_eq!(notice_of(&view, "[*][*]"), "");
        assert_eq!(notice_of(&view, "[*]"), "");
    }
}

#[test]
fn grid_a_blank_all_row_next_to_index_limits_shows_no_mixed_notice() {
    let rows = vec![
        bounded("[1][1]", "0", "10"),
        bounded("[1][2]", "0", "10"),
        bounded("[1][3]", "0", "10"),
        bounded("[2][*]", "", ""),
    ];
    let view = grid(&nest_tree(), rows);
    assert_eq!(notice_of(&view, "[*][*]"), "");
    assert_eq!(
        input_rows(&view),
        [
            input("[1][1]", "0", "10"),
            input("[1][2]", "0", "10"),
            input("[1][3]", "0", "10"),
            input("[2][*]", "", ""),
        ]
    );
}

#[test]
fn grid_blank_index_rows_next_to_an_all_limit_show_no_mixed_notice() {
    let rows = vec![
        bounded("[1][1]", "", ""),
        bounded("[1][2]", "", ""),
        bounded("[1][3]", "", ""),
        bounded("[2][*]", "0", "10"),
    ];
    let view = grid(&nest_tree(), rows);
    assert_eq!(notice_of(&view, "[*][*]"), "");
    let rows = input_rows(&view);
    assert!(rows.contains(&input("[2][*]", "0", "10")));
    assert!(!rows.iter().any(|row| *row == input("[2][1]", "0", "10")));
}

#[test]
fn grid_mixed_modes_of_a_pressed_type_level_join_the_copied_limits_and_show_no_mixed_notice() {
    let view = switched(&nest_tree(), mixed_rows(), "[*][*]");
    assert_eq!(
        input_rows(&view),
        [input("[1][*]", "0", "100"), input("[2][*]", "0", "5")]
    );
    assert_eq!(notice_of(&view, "[*][*]"), "");
}

#[test]
fn grid_the_switch_of_an_inner_type_level_gives_one_row_all_for_each_array_level() {
    let paths = ["[1][1]", "[1][2]", "[1][3]", "[2][1]", "[2][2]", "[2][3]"];
    let rows = paths.iter().map(|path| bounded(path, "0", "10")).collect();
    let view = switched(&nest_tree(), rows, "[*][*]");
    assert_eq!(
        input_rows(&view),
        [input("[1][*]", "0", "10"), input("[2][*]", "0", "10")]
    );
    assert_eq!(mode_levels(&view), ["[*]", "[*][*]"]);
    assert_eq!(
        mode_currents(&view),
        [
            "[*]: a limit per element",
            "[*][*]: one limit for all elements"
        ]
    );
    assert_eq!(notice_of(&view, "[*][*]"), "");
}
#[test]
fn grid_the_switch_back_with_unequal_limits_in_one_array_level_counts_that_level() {
    let rows = vec![
        bounded("[1][1]", "0", "10"),
        bounded("[1][2]", "0", "10"),
        bounded("[1][3]", "0", "10"),
        bounded("[2][1]", "0", "5"),
        bounded("[2][2]", "0", "6"),
        bounded("[2][3]", "0", "7"),
    ];
    let view = switched(&nest_tree(), rows, "[*][*]");
    assert_eq!(
        input_rows(&view),
        [input("[1][*]", "0", "10"), input("[2][*]", "", "")]
    );
    assert_eq!(
        notice_of(&view, "[*][*]"),
        "The elements of [*][*] have different limits. Array levels with different limits: 1. \
         Enter one limit for all elements."
    );
    assert_eq!(notice_of(&view, "[*]"), "");
}
#[test]
fn grid_the_switch_back_counts_each_array_level_with_unequal_limits() {
    let rows = vec![
        bounded("[1][1]", "0", "1"),
        bounded("[1][2]", "0", "2"),
        bounded("[1][3]", "0", "3"),
        bounded("[2][1]", "0", "5"),
        bounded("[2][2]", "0", "6"),
        bounded("[2][3]", "0", "7"),
    ];
    let view = switched(&nest_tree(), rows, "[*][*]");
    assert_eq!(
        input_rows(&view),
        [input("[1][*]", "", ""), input("[2][*]", "", "")]
    );
    assert!(notice_of(&view, "[*][*]").contains("different limits: 2."));
    let currents = mode_currents(&view);
    assert!(
        currents.contains(&"[*]: a limit per element".to_owned()),
        "{currents:?}"
    );
}
#[test]
fn grid_the_switch_to_a_limit_per_element_moves_the_limit_of_each_array_level() {
    let rows = vec![bounded("[1][*]", "0", "10"), bounded("[2][*]", "0", "5")];
    let view = switched(&nest_tree(), rows, "[*][*]");
    assert_eq!(
        input_rows(&view),
        [
            input("[1][1]", "0", "10"),
            input("[1][2]", "0", "10"),
            input("[1][3]", "0", "10"),
            input("[2][1]", "0", "5"),
            input("[2][2]", "0", "5"),
            input("[2][3]", "0", "5"),
        ]
    );
    assert_eq!(
        mode_currents(&view),
        ["[*]: a limit per element", "[*][*]: a limit per element"]
    );
}
#[test]
fn grid_the_switch_of_a_member_type_level_moves_the_limits_of_each_array_level() {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let arr = kit.array(int, &[(1, 3)]);
    let cell = kit.structure("ST_Cell", &[("arr", arr)]);
    let root = kit.array(cell, &[(1, 2)]);
    let tree = kit.tree(root);
    let rows: Vec<LimitTexts> = (1..=2)
        .flat_map(|outer| (1..=3).map(move |inner| format!("[{outer}].arr[{inner}]")))
        .map(|path| bounded(&path, "0", "100"))
        .collect();
    let view = switched(&tree, rows, "[*].arr[*]");
    assert_eq!(
        input_rows(&view),
        [
            input("[1].arr[*]", "0", "100"),
            input("[2].arr[*]", "0", "100")
        ]
    );
    assert_eq!(notice_of(&view, "[*].arr[*]"), "");
}
#[test]
fn grid_the_switch_of_a_type_level_below_one_limit_for_all_copies_to_the_all_row() {
    let view = switched(&nest_tree(), vec![bounded("[*][*]", "1", "9")], "[*][*]");
    assert_eq!(
        input_rows(&view),
        [
            input("[*][1]", "1", "9"),
            input("[*][2]", "1", "9"),
            input("[*][3]", "1", "9"),
        ]
    );
}

/// Return the sentence of the mode row of the type level `level`.
fn current_of(view: &GridView, level: &str) -> String {
    view.bodies
        .iter()
        .flat_map(|body| body.rows.iter())
        .filter_map(GridRow::mode)
        .find(|mode| mode.level == level)
        .map(|mode| mode.current.clone())
        .unwrap()
}

#[test]
fn grid_an_index_limit_in_the_second_branch_gives_its_type_level_a_limit_per_element() {
    let rows = ["[2][1]", "[2][2]", "[2][3]"].map(limit).to_vec();
    let view = grid(&nest_tree(), rows);
    assert_eq!(current_of(&view, "[*]"), "[*]: a limit per element");
    assert_eq!(current_of(&view, "[*][*]"), "[*][*]: a limit per element");
}

#[test]
fn grid_the_switch_of_a_type_level_with_an_index_limit_in_the_second_branch_joins() {
    let rows = ["[2][1]", "[2][2]", "[2][3]"].map(limit).to_vec();
    let view = switched(&nest_tree(), rows, "[*][*]");
    assert_eq!(
        current_of(&view, "[*][*]"),
        "[*][*]: one limit for all elements"
    );
    let inputs = input_rows(&view);
    assert!(inputs.contains(&input("[2][*]", "0", "100")));
    assert!(
        inputs
            .iter()
            .all(|(path, _min, _max)| !path.starts_with("[2][1"))
    );
}

#[test]
fn grid_mixed_modes_keep_a_sent_index_limit_of_the_same_array_level() {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let root = kit.array(int, &[(1, 2)]);
    let tree = kit.tree(root);
    let rows = vec![bounded("[*]", "0", "10"), bounded("[1]", "0", "5")];
    let view = grid(&tree, rows);
    assert_eq!(
        input_rows(&view),
        [input("[1]", "0", "5"), input("[2]", "0", "10")]
    );
}
