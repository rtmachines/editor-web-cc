//! Tests of `GridView`, the leaf grid of a parameter (task 10.2, ADR-0039 (j) to (n), design D8
//! and D16).

use recipe_domain::plant::{LeafPath, Primitive, TypeTree};
use recipe_domain::recipe::StoredLimit;

use super::grid::{
    GridRow, GridSource, GridView, LeafInput, LeafRow, LimitTexts, ModeRow, ValueTexts,
};
use super::tree_kit::{
    array_tree, line_tree, primitive_tree, stored_texts, string_tree, zones_tree,
};
use crate::error::{FieldError, FieldReason, FormErrors};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;

/// The size of a group in the tests: `form.array_chunk`.
const CHUNK: usize = 100;

/// Build the grid of `tree` in English.
fn grid(
    tree: &TypeTree,
    values: ValueTexts,
    limits: Vec<LimitTexts>,
    toggle: Option<&LeafPath>,
) -> GridView {
    let bundles = Bundles::load().unwrap();
    let source = GridSource {
        tree,
        values,
        limits,
        toggle,
        chunk: CHUNK,
        root_label: "SPEED".to_owned(),
    };
    GridView::build(source, Texts::new(&bundles, Lang::En))
}

/// Build the grid of a stored value without limits.
fn stored_grid(tree: &TypeTree, json: &str) -> GridView {
    grid(
        tree,
        ValueTexts::from_stored(&stored_texts(tree, json)),
        Vec::new(),
        None,
    )
}

/// Build one limit row of texts.
fn limit(path: &str, min: &str, max: &str) -> LimitTexts {
    LimitTexts {
        path: LeafPath::parse(path).unwrap(),
        min: min.to_owned(),
        max: max.to_owned(),
        max_length: String::new(),
    }
}

/// Return every row of the grid in the order of the table.
fn rows(view: &GridView) -> Vec<&GridRow> {
    view.bodies.iter().flat_map(|body| &body.rows).collect()
}

/// Return the leaf rows and the rows `[*]` of the grid.
fn leaf_rows(view: &GridView) -> Vec<&LeafRow> {
    rows(view).into_iter().filter_map(GridRow::leaf).collect()
}

/// Return the mode rows of the grid.
fn mode_rows(view: &GridView) -> Vec<&ModeRow> {
    rows(view).into_iter().filter_map(GridRow::mode).collect()
}

/// Return the shown path of each leaf row and row `[*]`.
fn paths(view: &GridView) -> Vec<String> {
    leaf_rows(view).iter().map(|row| row.path.clone()).collect()
}

/// Return the row of the shown path `path`.
fn row<'v>(view: &'v GridView, path: &str) -> &'v LeafRow {
    leaf_rows(view)
        .into_iter()
        .find(|row| row.path == path)
        .unwrap()
}

/// Return the texts of the minimum and the maximum input of a row, or `None` without inputs.
fn bounds(row: &LeafRow) -> Option<(String, String)> {
    Some((
        row.min.as_ref()?.text.clone(),
        row.max.as_ref()?.text.clone(),
    ))
}

/// Assert the value input and the hint of the grid of a primitive with the stored value `json`.
///
/// `input` holds the flag `choice` and `maxlength`. `hint` holds the `id` and the sentence, or two
/// empty texts for a class without a hint.
fn assert_class(tree: &TypeTree, json: &str, text: &str, input: (bool, u16), hint: (&str, &str)) {
    let view = stored_grid(tree, json);
    let only = leaf_rows(&view);
    assert_eq!(only.len(), 1, "{json}");
    let value = only.first().unwrap().value.as_ref().unwrap();
    assert_eq!(
        (value.id.as_str(), value.name.as_str(), value.text.as_str()),
        ("value-0", "value", text),
        "{json}"
    );
    assert_eq!((value.choice, value.maxlength), input, "{json}");
    assert_eq!(value.hint, hint.0, "{json}");
    let hints: Vec<(&str, &str)> = view
        .hints
        .iter()
        .map(|one| (one.id.as_str(), one.text.as_str()))
        .collect();
    let expected = if hint.0.is_empty() {
        Vec::new()
    } else {
        vec![hint]
    };
    assert_eq!(hints, expected, "{json}");
}

#[test]
fn grid_numeric_classes_have_a_text_input_and_the_hint_of_their_kind() {
    let int = ("grid-hint-integer", "A whole number, for example 1200.");
    assert_class(
        &primitive_tree(Primitive::Int),
        "1200",
        "1200",
        (false, 0),
        int,
    );
    let real = (
        "grid-hint-number",
        "A decimal number with a dot, for example 1.5.",
    );
    assert_class(
        &primitive_tree(Primitive::Real),
        "1.50",
        "1.5",
        (false, 0),
        real,
    );
}

#[test]
fn grid_bool_char_and_string_classes_have_their_input() {
    let none = ("", "");
    assert_class(
        &primitive_tree(Primitive::Bool),
        "true",
        "true",
        (true, 0),
        none,
    );
    assert_class(
        &primitive_tree(Primitive::Char),
        "\"A\"",
        "A",
        (false, 0),
        none,
    );
    let string = ("grid-hint-string-20", "Text of at most 20 characters.");
    let tree = string_tree(Primitive::String, 20);
    assert_class(&tree, "\"LINE A\"", "LINE A", (false, 20), string);
    let wide = ("grid-hint-string-10", "Text of at most 10 characters.");
    let tree = string_tree(Primitive::Wstring, 10);
    assert_class(&tree, "\"Ża\"", "Ża", (false, 10), wide);
}

#[test]
fn grid_time_classes_show_the_canonical_literal_and_an_example_of_their_prefix() {
    let time = (
        "grid-hint-duration-t",
        "A duration as an IEC literal, for example T#1m30s.",
    );
    let tree = primitive_tree(Primitive::Time);
    assert_class(&tree, "\"T#90s\"", "T#1m30s", (false, 0), time);
    let date = (
        "grid-hint-date-d",
        "A date as an IEC literal, for example D#2026-10-12.",
    );
    let tree = primitive_tree(Primitive::Date);
    assert_class(&tree, "\"D#2026-10-12\"", "D#2026-10-12", (false, 0), date);
    let date_time = (
        "grid-hint-date-time-dt",
        "A date and a time as an IEC literal, for example DT#2026-10-12-12:30:00.",
    );
    let tree = primitive_tree(Primitive::DateAndTime);
    let literal = "DT#2026-10-12-12:30:00";
    assert_class(
        &tree,
        &format!("\"{literal}\""),
        literal,
        (false, 0),
        date_time,
    );
}

#[test]
fn grid_a_row_shows_only_the_limit_inputs_of_its_class() {
    let int = stored_grid(&primitive_tree(Primitive::Int), "1200");
    let row = row(&int, "SPEED");
    assert_eq!(
        row.min
            .as_ref()
            .map(|input| (input.id.as_str(), input.name.as_str())),
        Some(("limits-0-min", "limits[0].min"))
    );
    assert_eq!(
        row.max.as_ref().map(|input| input.id.as_str()),
        Some("limits-0-max")
    );
    assert!(row.max_length.is_none());
    assert_eq!(
        row.limit_path
            .as_ref()
            .map(|hidden| (hidden.name.as_str(), hidden.value.as_str())),
        Some(("limits[0].leaf_path", ""))
    );
    let text = stored_grid(&string_tree(Primitive::String, 20), "\"A\"");
    let text_row = leaf_rows(&text).into_iter().next().unwrap();
    assert!(text_row.min.is_none() && text_row.max.is_none());
    assert_eq!(
        text_row
            .max_length
            .as_ref()
            .map(|input| (input.id.as_str(), input.name.as_str())),
        Some(("limits-0-max_length", "limits[0].max_length"))
    );
    for primitive in [Primitive::Bool, Primitive::Char] {
        let json = if primitive == Primitive::Bool {
            "false"
        } else {
            "\"B\""
        };
        let view = stored_grid(&primitive_tree(primitive), json);
        let none = leaf_rows(&view).into_iter().next().unwrap();
        assert!(none.min.is_none() && none.max.is_none() && none.max_length.is_none());
        assert!(none.limit_path.is_none() && none.from_all.is_empty());
    }
}

#[test]
fn grid_an_array_with_one_limit_for_all_has_a_row_all_before_its_eight_rows() {
    let tree = array_tree(Primitive::Int, &[(1, 8)]);
    let json = "[100,200,300,400,500,600,700,800]";
    let view = grid(
        &tree,
        ValueTexts::from_stored(&stored_texts(&tree, json)),
        vec![limit("[*]", "0", "3000")],
        None,
    );
    let expected: Vec<String> = std::iter::once("[*]".to_owned())
        .chain((1..=8).map(|index| format!("[{index}]")))
        .collect();
    assert_eq!(paths(&view), expected);
    let all = row(&view, "[*]");
    assert!(all.value.is_none());
    assert_eq!(
        all.all.as_ref().map(|cell| cell.text.as_str()),
        Some("every element [1..8]")
    );
    assert_eq!(bounds(all), Some(("0".to_owned(), "3000".to_owned())));
    let third = row(&view, "[3]");
    assert_eq!(
        third.value.as_ref().map(|value| value.id.as_str()),
        Some("value-2")
    );
    assert_eq!(
        third.value.as_ref().map(|value| value.name.as_str()),
        Some("value[3]")
    );
    assert_eq!(bounds(third), None);
    assert_eq!(third.from_all, "0 to 3000 (from [*])");
}

#[test]
fn grid_the_mode_row_follows_the_row_all_and_names_the_level() {
    let tree = array_tree(Primitive::Int, &[(1, 8)]);
    let view = grid(
        &tree,
        ValueTexts::default(),
        vec![limit("[*]", "0", "3000")],
        None,
    );
    let kinds: Vec<&str> = rows(&view)
        .iter()
        .take(3)
        .map(|row| match (row.leaf(), row.mode()) {
            (Some(_), _) => "leaf",
            (None, Some(_)) => "mode",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["leaf", "mode", "leaf"]);
    let modes = mode_rows(&view);
    let mode = modes.first().unwrap();
    assert_eq!(
        (mode.id.as_str(), mode.level.as_str()),
        ("grid-mode-0", "[*]")
    );
    assert_eq!(mode.label, "Set a limit per element of [*]");
    assert_eq!(mode.current, "[*]: one limit for all elements");
    assert!(mode.notice.is_empty());
}

#[test]
fn grid_a_row_of_an_element_without_a_limit_for_all_says_so() {
    let tree = array_tree(Primitive::Int, &[(1, 2)]);
    let view = grid(&tree, ValueTexts::default(), Vec::new(), None);
    assert_eq!(row(&view, "[1]").from_all, "no limit yet (from [*])");
    let strings = {
        let mut kit = super::tree_kit::Kit::new();
        let text = kit.string(Primitive::String, 8);
        let root = kit.array(text, &[(1, 2)]);
        kit.tree(root)
    };
    let mut length = limit("[*]", "", "");
    length.max_length = "6".to_owned();
    let view = grid(&strings, ValueTexts::default(), vec![length], None);
    assert_eq!(
        row(&view, "[2]").from_all,
        "at most 6 characters (from [*])"
    );
    assert_eq!(
        row(&view, "[*]")
            .max_length
            .as_ref()
            .map(|input| input.text.as_str()),
        Some("6")
    );
}

#[test]
fn grid_a_two_dimensional_array_runs_in_row_major_order() {
    let tree = array_tree(Primitive::Real, &[(0, 1), (1, 2)]);
    let view = stored_grid(&tree, "[[0.5,1.5],[2.5,3.5]]");
    assert_eq!(paths(&view), ["[*]", "[0,1]", "[0,2]", "[1,1]", "[1,2]"]);
    assert_eq!(
        row(&view, "[*]")
            .all
            .as_ref()
            .map(|cell| cell.text.as_str()),
        Some("every element [0..1,1..2]")
    );
    let texts: Vec<String> = leaf_rows(&view)
        .iter()
        .filter_map(|row| row.value.as_ref().map(|value| value.text.clone()))
        .collect();
    assert_eq!(texts, ["0.5", "1.5", "2.5", "3.5"]);
}

#[test]
fn grid_a_struct_in_a_struct_joins_the_member_names() {
    let tree = line_tree();
    let json = r#"{"name":"A","speed":5,"zone":{"setpoint":1.5,"hold":"T#90s","enabled":true}}"#;
    let view = stored_grid(&tree, json);
    assert_eq!(
        paths(&view),
        [
            ".name",
            ".speed",
            ".zone.setpoint",
            ".zone.hold",
            ".zone.enabled"
        ]
    );
    let enabled = row(&view, ".zone.enabled");
    assert!(enabled.value.as_ref().unwrap().choice);
    assert!(enabled.min.is_none() && enabled.max_length.is_none());
    assert_eq!(
        row(&view, ".zone.hold").value.as_ref().unwrap().text,
        "T#1m30s"
    );
    assert_eq!(row(&view, ".zone.hold").type_text, "TIME");
    assert_eq!(row(&view, ".name").type_text, "STRING(20)");
    let ids: Vec<&str> = view.hints.iter().map(|hint| hint.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "grid-hint-string-20",
            "grid-hint-integer",
            "grid-hint-number",
            "grid-hint-duration-t"
        ]
    );
    assert_eq!(
        row(&view, ".speed").value.as_ref().unwrap().hint,
        "grid-hint-integer"
    );
}

#[test]
fn grid_an_array_of_structs_has_rows_all_then_the_element_rows() {
    let view = grid(&zones_tree(), ValueTexts::default(), Vec::new(), None);
    let mut expected = vec![
        "[*].setpoint".to_owned(),
        "[*].hold".to_owned(),
        "[*].enabled".to_owned(),
    ];
    for index in 1..=4 {
        for member in ["setpoint", "hold", "enabled"] {
            expected.push(format!("[{index}].{member}"));
        }
    }
    assert_eq!(paths(&view), expected);
    assert_eq!(
        row(&view, "[4].enabled")
            .value
            .as_ref()
            .map(|value| value.id.as_str()),
        Some("value-11")
    );
    assert_eq!(mode_rows(&view).len(), 1);
}

#[test]
fn grid_limits_with_an_index_show_a_limit_per_element() {
    let limits = vec![
        limit("[1].setpoint", "20", "250"),
        limit("[2].setpoint", "30", "260"),
    ];
    let view = grid(&zones_tree(), ValueTexts::default(), limits, None);
    assert!(!paths(&view).iter().any(|path| path.starts_with("[*]")));
    assert_eq!(
        bounds(row(&view, "[2].setpoint")),
        Some(("30".to_owned(), "260".to_owned()))
    );
    assert_eq!(
        bounds(row(&view, "[3].setpoint")),
        Some((String::new(), String::new()))
    );
    let modes = mode_rows(&view);
    let mode = modes.first().unwrap();
    assert_eq!(mode.label, "Set one limit for all elements of [*]");
    assert_eq!(mode.current, "[*]: a limit per element");
}

#[test]
fn grid_the_switch_to_a_limit_per_element_copies_the_limit_of_all() {
    let level = LeafPath::parse("[*]").unwrap();
    let view = grid(
        &zones_tree(),
        ValueTexts::default(),
        vec![limit("[*].setpoint", "20", "250")],
        Some(&level),
    );
    for index in 1..=4 {
        let setpoint = row(&view, &format!("[{index}].setpoint"));
        assert_eq!(
            bounds(setpoint),
            Some(("20".to_owned(), "250".to_owned())),
            "{index}"
        );
        assert_eq!(
            setpoint
                .limit_path
                .as_ref()
                .map(|hidden| hidden.value.clone()),
            Some(format!("[{index}].setpoint"))
        );
    }
    assert!(!paths(&view).iter().any(|path| path.starts_with("[*]")));
    assert_eq!(
        mode_rows(&view).first().unwrap().current,
        "[*]: a limit per element"
    );
}

#[test]
fn grid_the_switch_to_a_limit_per_element_without_a_limit_keeps_the_mode() {
    let level = LeafPath::parse("[*]").unwrap();
    let view = grid(
        &zones_tree(),
        ValueTexts::default(),
        Vec::new(),
        Some(&level),
    );
    assert_eq!(
        bounds(row(&view, "[1].setpoint")),
        Some((String::new(), String::new()))
    );
    assert!(!paths(&view).iter().any(|path| path.starts_with("[*]")));
}

#[test]
fn grid_the_switch_back_with_equal_limits_joins_them() {
    let level = LeafPath::parse("[*]").unwrap();
    let limits = (1..=4)
        .map(|index| limit(&format!("[{index}].setpoint"), "20", "250"))
        .collect();
    let view = grid(&zones_tree(), ValueTexts::default(), limits, Some(&level));
    assert_eq!(
        bounds(row(&view, "[*].setpoint")),
        Some(("20".to_owned(), "250".to_owned()))
    );
    assert_eq!(
        row(&view, "[2].setpoint").from_all,
        "20 to 250 (from [*].setpoint)"
    );
    assert!(mode_rows(&view).first().unwrap().notice.is_empty());
}

#[test]
fn grid_the_switch_back_with_unequal_limits_leaves_all_empty_with_a_notice() {
    let level = LeafPath::parse("[*]").unwrap();
    let limits = vec![
        limit("[1].setpoint", "20", "250"),
        limit("[2].setpoint", "30", "260"),
    ];
    let view = grid(&zones_tree(), ValueTexts::default(), limits, Some(&level));
    assert_eq!(
        bounds(row(&view, "[*].setpoint")),
        Some((String::new(), String::new()))
    );
    let modes = mode_rows(&view);
    let mode = modes.first().unwrap();
    assert_eq!(
        mode.notice,
        "The elements of [*] have different limits. Enter one limit for all elements."
    );
    assert_eq!(mode.current, "[*]: one limit for all elements");
}

#[test]
fn grid_an_array_of_1000_elements_has_ten_groups_and_only_the_first_is_open() {
    let tree = array_tree(Primitive::Int, &[(1, 1000)]);
    let view = grid(&tree, ValueTexts::default(), Vec::new(), None);
    let groups: Vec<(&str, bool, usize)> = view
        .bodies
        .iter()
        .filter(|body| !body.id.is_empty())
        .map(|body| (body.id.as_str(), body.hidden, body.rows.len()))
        .collect();
    let expected: Vec<(String, bool, usize)> = (0..10)
        .map(|group| (format!("grid-group-{group}"), group > 0, 100))
        .collect();
    let expected: Vec<(&str, bool, usize)> = expected
        .iter()
        .map(|(id, hidden, count)| (id.as_str(), *hidden, *count))
        .collect();
    assert_eq!(groups, expected);
    let toggles: Vec<(String, bool, String)> = rows(&view)
        .into_iter()
        .filter_map(GridRow::toggle)
        .map(|toggle| (toggle.group.clone(), toggle.expanded, toggle.text.clone()))
        .collect();
    assert_eq!(toggles.len(), 10);
    assert_eq!(
        toggles.first().unwrap(),
        &(
            "grid-group-0".to_owned(),
            true,
            "Elements [1]..[100] of [*] (elements: 100)".to_owned()
        )
    );
    assert_eq!(
        toggles.last().unwrap(),
        &(
            "grid-group-9".to_owned(),
            false,
            "Elements [901]..[1000] of [*] (elements: 100)".to_owned()
        )
    );
}

#[test]
fn grid_each_toggle_stands_in_its_own_body_before_its_group() {
    let tree = array_tree(Primitive::Int, &[(1, 250)]);
    let view = grid(&tree, ValueTexts::default(), Vec::new(), None);
    let shape: Vec<(bool, usize, bool)> = view
        .bodies
        .iter()
        .map(|body| {
            let first_is_toggle = body.rows.first().and_then(GridRow::toggle).is_some();
            (body.id.is_empty(), body.rows.len(), first_is_toggle)
        })
        .collect();
    assert_eq!(
        shape,
        [
            (true, 2, false),
            (true, 1, true),
            (false, 100, false),
            (true, 1, true),
            (false, 100, false),
            (true, 1, true),
            (false, 50, false)
        ]
    );
}

#[test]
fn grid_a_group_with_an_error_line_is_open() {
    let tree = array_tree(Primitive::Int, &[(1, 1000)]);
    let view = grid(&tree, ValueTexts::default(), Vec::new(), None);
    let id = view
        .input_id(&LeafPath::parse("[550]").unwrap(), LeafInput::Value)
        .unwrap();
    assert_eq!(id, "value-549");
    let line = FieldError::bad_input(
        "field-value-leaf",
        vec![("leaf", "[550]".to_owned())],
        FieldReason::ValueKey,
        id,
    );
    let bundles = Bundles::load().unwrap();
    let errors = FormErrors::new([&line], &bundles, Lang::En);
    let view = view.with_errors(&errors);
    let open: Vec<&str> = view
        .bodies
        .iter()
        .filter(|body| !body.id.is_empty() && !body.hidden)
        .map(|body| body.id.as_str())
        .collect();
    assert_eq!(open, ["grid-group-0", "grid-group-5"]);
    let expanded = rows(&view)
        .into_iter()
        .filter_map(GridRow::toggle)
        .filter(|toggle| toggle.expanded)
        .count();
    assert_eq!(expanded, 2);
}

#[test]
fn grid_an_array_inside_a_group_forms_no_group_of_its_own() {
    let mut kit = super::tree_kit::Kit::new();
    let int = kit.primitive(Primitive::Int);
    let inner = kit.array(int, &[(1, 101)]);
    let cell = kit.structure("ST_Cell", &[("arr", inner)]);
    let root = kit.array(cell, &[(1, 101)]);
    let tree = kit.tree(root);
    let view = grid(&tree, ValueTexts::default(), Vec::new(), None);
    let groups: Vec<&str> = view
        .bodies
        .iter()
        .filter(|body| !body.id.is_empty())
        .map(|body| body.id.as_str())
        .collect();
    assert_eq!(groups, ["grid-group-0", "grid-group-1"]);
    let toggles = rows(&view).into_iter().filter_map(GridRow::toggle).count();
    assert_eq!(toggles, 2);
}

#[test]
fn grid_a_limit_mode_that_names_no_array_level_is_ignored() {
    let plain = grid(&zones_tree(), ValueTexts::default(), Vec::new(), None);
    let level = LeafPath::parse("[1].hold[*]").unwrap();
    let pressed = grid(
        &zones_tree(),
        ValueTexts::default(),
        Vec::new(),
        Some(&level),
    );
    assert_eq!(paths(&pressed), paths(&plain));
    let currents = |view: &GridView| -> Vec<String> {
        mode_rows(view)
            .iter()
            .map(|mode| mode.current.clone())
            .collect()
    };
    assert_eq!(currents(&pressed), currents(&plain));
}

#[test]
fn grid_without_a_chunk_has_no_group() {
    let tree = array_tree(Primitive::Int, &[(1, 300)]);
    let bundles = Bundles::load().unwrap();
    let source = GridSource {
        tree: &tree,
        values: ValueTexts::default(),
        limits: Vec::new(),
        toggle: None,
        chunk: 0,
        root_label: String::new(),
    };
    let view = GridView::build(source, Texts::new(&bundles, Lang::En));
    assert_eq!(view.bodies.len(), 1);
}

#[test]
fn grid_the_input_of_a_leaf_path_is_its_value_input_or_its_limit_input() {
    let tree = array_tree(Primitive::Int, &[(1, 8)]);
    let view = grid(
        &tree,
        ValueTexts::default(),
        vec![limit("[*]", "0", "1")],
        None,
    );
    let id = |text: &str, input| view.input_id(&LeafPath::parse(text).unwrap(), input);
    assert_eq!(id("[3]", LeafInput::Value), Some("value-2".to_owned()));
    assert_eq!(id("[3]", LeafInput::Limit), Some("limits-0-min".to_owned()));
    assert_eq!(id("[*]", LeafInput::Limit), Some("limits-0-min".to_owned()));
    assert_eq!(id("[*]", LeafInput::Value), None);
    assert_eq!(id("[9]", LeafInput::Value), None);
    let strings = grid(
        &string_tree(Primitive::String, 4),
        ValueTexts::default(),
        Vec::new(),
        None,
    );
    assert_eq!(
        strings.input_id(&LeafPath::parse("").unwrap(), LeafInput::Value),
        Some("value-0".to_owned())
    );
}

#[test]
fn grid_sent_texts_show_as_sent() {
    let tree = array_tree(Primitive::Int, &[(1, 2)]);
    let sent = vec![
        (LeafPath::parse("[1]").unwrap(), " 12,5 ".to_owned()),
        (LeafPath::parse("[2]").unwrap(), "<b>".to_owned()),
    ];
    let view = grid(&tree, ValueTexts::from_sent(&sent), Vec::new(), None);
    assert_eq!(row(&view, "[1]").value.as_ref().unwrap().text, " 12,5 ");
    assert_eq!(row(&view, "[2]").value.as_ref().unwrap().text, "<b>");
}

#[test]
fn grid_the_sent_limit_rows_of_a_form_show_as_sent() {
    let pairs: Vec<(String, String)> = [
        ("limits[0].leaf_path", "[*]"),
        ("limits[0].min", " 1,5 "),
        ("limits[0].max", ""),
    ]
    .iter()
    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
    .collect();
    let sent = super::fields::read(&pairs, super::fields::FieldSet::Parameter).unwrap();
    let tree = array_tree(Primitive::Int, &[(1, 2)]);
    let view = grid(
        &tree,
        ValueTexts::from_sent(sent.values()),
        LimitTexts::from_sent(sent.limits()),
        None,
    );
    assert_eq!(
        bounds(row(&view, "[*]")),
        Some((" 1,5 ".to_owned(), String::new()))
    );
}

#[test]
fn grid_stored_limits_of_a_time_leaf_show_as_iec_literals() {
    let tree = array_tree(Primitive::Time, &[(1, 2)]);
    let stored = [StoredLimit {
        leaf_path: LeafPath::parse("[*]").unwrap(),
        min: Some("1000".parse().unwrap()),
        max: Some("90000".parse().unwrap()),
        max_length: None,
    }];
    let limits = LimitTexts::from_stored(&tree, &stored);
    let texts: Vec<(String, &str, &str)> = limits
        .iter()
        .map(|row| (row.path.to_string(), row.min.as_str(), row.max.as_str()))
        .collect();
    assert_eq!(texts, [("[*]".to_owned(), "T#1s", "T#1m30s")]);
    let length = StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: None,
        max_length: Some(12),
    };
    let strings = string_tree(Primitive::String, 20);
    let rows = LimitTexts::from_stored(&strings, &[length]);
    assert_eq!(rows.first().map(|row| row.max_length.as_str()), Some("12"));
}

#[test]
fn grid_the_row_of_a_primitive_shows_the_root_label() {
    let view = stored_grid(&primitive_tree(Primitive::Int), "7");
    assert_eq!(paths(&view), ["SPEED"]);
    assert_eq!(leaf_rows(&view).first().unwrap().id, "grid-row-0");
}
