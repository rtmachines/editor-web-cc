//! Tests of the macros of the leaf grid and of the read-only grid fragments (task 10.3).
//!
//! The rules come from design D8, ADR-0039 (i) to (n), and `design-system.md` 10.4, 10.5, 10.8.

use askama::Template;

use super::icon;
use crate::error::{FieldError, FieldReason, FormErrors};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::parameters::grid::{
    GridLimitsFragment, GridSource, GridValuesFragment, GridView, LimitTexts, ValueTexts,
};
use crate::pages::parameters::tree_kit::{
    array_tree, line_tree, primitive_tree, stored_texts, string_tree, zones_tree,
};
use crate::pages::test_helpers::assert_clean_html;
use recipe_domain::plant::{LeafPath, Primitive, TypeTree};

#[derive(Template)]
#[template(path = "test/grid.html")]
struct GridPage<'a> {
    t: Texts<'a>,
    view: GridView,
    errors: FormErrors,
}

#[derive(Template)]
#[template(path = "test/grid-form.html")]
struct GridFormPage<'a> {
    t: Texts<'a>,
    action: String,
    view: GridView,
    errors: FormErrors,
}

/// Build the grid of `tree` in English with the root label `SPEED`.
fn view(
    tree: &TypeTree,
    values: ValueTexts,
    limits: Vec<LimitTexts>,
    bundles: &Bundles,
) -> GridView {
    let source = GridSource {
        tree,
        values,
        limits,
        toggle: None,
        chunk: 100,
        root_label: "SPEED".to_owned(),
    };
    GridView::build(source, Texts::new(bundles, Lang::En))
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

/// Render the grid of a stored value with its limits.
fn render(tree: &TypeTree, json: &str, limits: Vec<LimitTexts>) -> String {
    let bundles = Bundles::load().unwrap();
    let values = ValueTexts::from_stored(&stored_texts(tree, json));
    GridPage {
        t: Texts::new(&bundles, Lang::En),
        view: view(tree, values, limits, &bundles),
        errors: FormErrors::default(),
    }
    .render()
    .unwrap()
}

/// The attributes of every data input of the grid before its value.
const DATA: &str = "type=\"text\" class=\"data\" autocapitalize=\"none\" spellcheck=\"false\" autocomplete=\"off\"";

#[test]
fn grid_component_the_grid_is_a_table_in_a_tree_with_six_column_headers() {
    let html = render(&primitive_tree(Primitive::Int), "1200", Vec::new());
    assert!(html.contains("<div class=\"tree\">"), "{html}");
    assert!(
        html.contains("<div class=\"table-wrap\" role=\"region\" tabindex=\"0\" aria-labelledby=\"grid-caption\"><table class=\"leaf-grid\"><caption id=\"grid-caption\">Values and process limits</caption>"),
        "{html}"
    );
    let head = "<thead><tr><th scope=\"col\" id=\"grid-col-path\">Path</th><th scope=\"col\" id=\"grid-col-value\">Value</th><th scope=\"col\" id=\"grid-col-min\">Minimum</th><th scope=\"col\" id=\"grid-col-max\">Maximum</th><th scope=\"col\" id=\"grid-col-max-length\">Maximum length</th><th scope=\"col\" id=\"grid-col-type\">Type</th></tr></thead>";
    assert!(html.contains(head), "{html}");
    let hint = html.find(
        "<p class=\"field-hint\" id=\"grid-hint-integer\">A whole number, for example 1200.</p>",
    );
    let table = html.find("<table");
    assert!(hint.is_some() && hint < table, "{html}");
    assert_clean_html(&html);
}

#[test]
fn grid_component_a_numeric_row_has_the_value_the_bounds_and_the_type() {
    let html = render(
        &primitive_tree(Primitive::Int),
        "1200",
        vec![limit("", "0", "3000")],
    );
    let expected = format!(
        "<tr><th scope=\"row\" id=\"grid-row-0\">SPEED</th><td><input id=\"value-0\" name=\"value\" {DATA} value=\"1200\" aria-labelledby=\"grid-row-0 grid-col-value\" aria-describedby=\"grid-hint-integer\"><input type=\"hidden\" name=\"limits[0].leaf_path\" value=\"\"></td><td><input id=\"limits-0-min\" name=\"limits[0].min\" {DATA} value=\"0\" aria-labelledby=\"grid-row-0 grid-col-min\" aria-describedby=\"grid-hint-integer\"></td><td><input id=\"limits-0-max\" name=\"limits[0].max\" {DATA} value=\"3000\" aria-labelledby=\"grid-row-0 grid-col-max\" aria-describedby=\"grid-hint-integer\"></td><td class=\"grid-note\">not applicable</td><td class=\"data\">INT</td></tr>"
    );
    assert!(html.contains(&expected), "{html}");
    assert!(!html.contains("inputmode"), "{html}");
}

#[test]
fn grid_component_a_string_row_has_maxlength_and_a_length_input() {
    let html = render(
        &string_tree(Primitive::String, 20),
        "\"LINE A\"",
        Vec::new(),
    );
    assert!(
        html.contains(&format!("<input id=\"value-0\" name=\"value\" {DATA} maxlength=\"20\" value=\"LINE A\" aria-labelledby=\"grid-row-0 grid-col-value\" aria-describedby=\"grid-hint-string-20\">")),
        "{html}"
    );
    assert!(
        html.contains("<td class=\"grid-note\">not applicable</td><td class=\"grid-note\">not applicable</td><td><input id=\"limits-0-max_length\" name=\"limits[0].max_length\""),
        "{html}"
    );
}

#[test]
fn grid_component_a_bool_row_is_a_select_with_true_and_false_and_no_limit() {
    let tree = line_tree();
    let json = r#"{"name":"A","speed":5,"zone":{"setpoint":1.5,"hold":"T#90s","enabled":false}}"#;
    let html = render(&tree, json, Vec::new());
    assert!(
        html.contains("<select id=\"value-4\" name=\"value.zone.enabled\" aria-labelledby=\"grid-row-4 grid-col-value\"><option value=\"true\">TRUE</option><option value=\"false\" selected>FALSE</option></select></td><td class=\"grid-note\">not applicable</td><td class=\"grid-note\">not applicable</td><td class=\"grid-note\">not applicable</td><td class=\"data\">BOOL</td>"),
        "{html}"
    );
    assert!(html.contains("value=\"T#1m30s\""), "{html}");
}

#[test]
fn grid_component_a_bool_leaf_without_a_text_has_an_empty_first_option_and_is_required() {
    let bundles = Bundles::load().unwrap();
    let tree = primitive_tree(Primitive::Bool);
    let html = GridPage {
        t: Texts::new(&bundles, Lang::En),
        view: view(&tree, ValueTexts::default(), Vec::new(), &bundles),
        errors: FormErrors::default(),
    }
    .render()
    .unwrap();
    assert!(
        html.contains("<select id=\"value-0\" name=\"value\" aria-labelledby=\"grid-row-0 grid-col-value\" aria-required=\"true\"><option value=\"\" selected></option><option value=\"true\">TRUE</option><option value=\"false\">FALSE</option></select>"),
        "{html}"
    );
}

#[test]
fn grid_component_the_limits_fragment_without_a_limit_shows_the_empty_state() {
    let bundles = Bundles::load().unwrap();
    let html = GridLimitsFragment {
        t: Texts::new(&bundles, Lang::En),
        rows: &[],
        root_label: "SPEED".to_owned(),
    }
    .render()
    .unwrap();
    assert!(
        html.contains("<tbody><tr><td colspan=\"4\"><p class=\"empty\">No process limits.</p></td></tr></tbody>"),
        "{html}"
    );
}

#[test]
fn grid_component_a_row_all_names_the_cell_every_element_in_its_labels() {
    let tree = array_tree(Primitive::Int, &[(1, 8)]);
    let html = render(&tree, "[1,2,3,4,5,6,7,8]", vec![limit("[*]", "0", "3000")]);
    assert!(
        html.contains(&format!("<tr class=\"row-all\"><th scope=\"row\" id=\"grid-row-0\">[*]</th><td><span id=\"grid-row-0-all\">every element [1..8]</span><input type=\"hidden\" name=\"limits[0].leaf_path\" value=\"[*]\"></td><td><input id=\"limits-0-min\" name=\"limits[0].min\" {DATA} value=\"0\" aria-labelledby=\"grid-row-0 grid-row-0-all grid-col-min\" aria-describedby=\"grid-hint-integer\"></td>")),
        "{html}"
    );
    assert!(
        html.contains("<td colspan=\"3\" class=\"grid-note\">0 to 3000 (from [*])</td><td class=\"data\">INT</td>"),
        "{html}"
    );
    assert_eq!(html.matches("<tr class=\"row-all\">").count(), 1);
}

#[test]
fn grid_component_the_mode_row_has_the_current_mode_and_one_submit_button() {
    let html = render(&zones_tree(), ZONES, Vec::new());
    assert!(
        html.contains("<tr><td colspan=\"6\"><span class=\"grid-note\">[*]: one limit for all elements</span> <button type=\"submit\" id=\"grid-mode-0\" name=\"limit_mode\" value=\"[*]\" class=\"btn btn-secondary\">Set a limit per element of [*]</button></td></tr>"),
        "{html}"
    );
}

/// The value of `ZONES` with four zones.
const ZONES: &str = r#"[{"setpoint":1,"hold":"T#1s","enabled":true},{"setpoint":2,"hold":"T#1s","enabled":true},{"setpoint":3,"hold":"T#1s","enabled":true},{"setpoint":4,"hold":"T#1s","enabled":true}]"#;

#[test]
fn grid_component_the_join_notice_is_a_warning() {
    let bundles = Bundles::load().unwrap();
    let tree = zones_tree();
    let level = LeafPath::parse("[*]").unwrap();
    let source = GridSource {
        tree: &tree,
        values: ValueTexts::default(),
        limits: vec![
            limit("[1].setpoint", "1", "2"),
            limit("[2].setpoint", "3", "4"),
        ],
        toggle: Some(&level),
        chunk: 100,
        root_label: String::new(),
    };
    let html = GridPage {
        t: Texts::new(&bundles, Lang::En),
        view: GridView::build(source, Texts::new(&bundles, Lang::En)),
        errors: FormErrors::default(),
    }
    .render()
    .unwrap();
    let notice = format!(
        "<div class=\"alert alert-warning\">{}<span>The elements of [*] have different limits. Enter one limit for all elements.</span></div>",
        icon("alert-warning")
    );
    assert!(html.contains(&notice), "{html}");
}

#[test]
fn grid_component_a_group_has_a_toggle_body_and_a_closed_body() {
    let tree = array_tree(Primitive::Int, &[(1, 1000)]);
    let bundles = Bundles::load().unwrap();
    let html = GridPage {
        t: Texts::new(&bundles, Lang::En),
        view: view(&tree, ValueTexts::default(), Vec::new(), &bundles),
        errors: FormErrors::default(),
    }
    .render()
    .unwrap();
    let toggle = |group: u32, expanded: &str, range: &str| {
        format!(
            "<tbody><tr><td colspan=\"6\" class=\"grid-toggle\"><button type=\"button\" class=\"tree-node\" aria-expanded=\"{expanded}\" aria-controls=\"grid-group-{group}\">{}{}<span>Elements {range} of [*] (elements: 100)</span></button></td></tr></tbody>",
            icon("tree-collapsed"),
            icon("tree-expanded")
        )
    };
    assert!(html.contains(&toggle(0, "true", "[1]..[100]")), "{html}");
    assert!(
        html.contains(&toggle(9, "false", "[901]..[1000]")),
        "{html}"
    );
    assert!(html.contains("<tbody id=\"grid-group-0\"><tr>"), "{html}");
    assert!(
        html.contains("<tbody id=\"grid-group-9\" hidden><tr>"),
        "{html}"
    );
    assert_eq!(html.matches("aria-controls=\"grid-group-").count(), 10);
    assert_eq!(html.matches(" hidden>").count(), 9);
}

/// The toggle of a group names the index range and the count of its elements (design-system.md
/// 10.8). The last group of 250 elements holds 50 elements.
#[test]
fn grid_component_a_toggle_names_the_count_of_its_group_in_polish() {
    let tree = array_tree(Primitive::Int, &[(1, 250)]);
    let bundles = Bundles::load().unwrap();
    let source = GridSource {
        tree: &tree,
        values: ValueTexts::default(),
        limits: Vec::new(),
        toggle: None,
        chunk: 100,
        root_label: "SPEED".to_owned(),
    };
    let t = Texts::new(&bundles, Lang::Pl);
    let html = GridPage {
        t,
        view: GridView::build(source, t),
        errors: FormErrors::default(),
    }
    .render()
    .unwrap();
    assert!(
        html.contains("<span>Elementy [1]..[100] z [*] (elementy: 100)</span>"),
        "{html}"
    );
    assert!(
        html.contains("<span>Elementy [201]..[250] z [*] (elementy: 50)</span>"),
        "{html}"
    );
}

#[test]
fn grid_component_an_input_with_a_line_is_invalid_and_shows_the_line_in_its_cell() {
    let bundles = Bundles::load().unwrap();
    let tree = primitive_tree(Primitive::Int);
    let line = FieldError::bad_input(
        "field-value-leaf",
        vec![("leaf", String::new())],
        FieldReason::ValueKey,
        "value-0".to_owned(),
    );
    let errors = FormErrors::new([&line], &bundles, Lang::En);
    let grid = view(&tree, ValueTexts::default(), Vec::new(), &bundles).with_errors(&errors);
    let html = GridPage {
        t: Texts::new(&bundles, Lang::En),
        view: grid,
        errors,
    }
    .render()
    .unwrap();
    assert!(
        html.contains(" aria-labelledby=\"grid-row-0 grid-col-value\" aria-invalid=\"true\" aria-describedby=\"grid-hint-integer value-0-error\"><div class=\"field-error\" id=\"value-0-error\"><p>"),
        "{html}"
    );
}

#[test]
fn grid_component_the_bound_inputs_name_the_hint_of_their_class() {
    let tree = primitive_tree(Primitive::Time);
    let html = render(&tree, "\"T#1s\"", vec![limit("", "T#0ms", "T#1m")]);
    assert!(
        html.contains("<input id=\"limits-0-min\" name=\"limits[0].min\" type=\"text\" class=\"data\" autocapitalize=\"none\" spellcheck=\"false\" autocomplete=\"off\" value=\"T#0ms\" aria-labelledby=\"grid-row-0 grid-col-min\" aria-describedby=\"grid-hint-duration-t\">"),
        "{html}"
    );
    assert!(
        html.contains("value=\"T#1m\" aria-labelledby=\"grid-row-0 grid-col-max\" aria-describedby=\"grid-hint-duration-t\">"),
        "{html}"
    );
    let text = render(&string_tree(Primitive::String, 8), "\"A\"", Vec::new());
    assert!(
        text.contains("aria-labelledby=\"grid-row-0 grid-col-max-length\">"),
        "{text}"
    );
}

#[test]
fn grid_component_a_limit_input_with_a_line_is_invalid_and_names_its_line() {
    let bundles = Bundles::load().unwrap();
    let tree = primitive_tree(Primitive::Int);
    let line = FieldError::bad_input(
        "field-limit-min",
        vec![("leaf", String::new())],
        FieldReason::LimitRow,
        "limits-0-min".to_owned(),
    );
    let errors = FormErrors::new([&line], &bundles, Lang::En);
    let grid = view(&tree, ValueTexts::default(), Vec::new(), &bundles).with_errors(&errors);
    let html = GridPage {
        t: Texts::new(&bundles, Lang::En),
        view: grid,
        errors,
    }
    .render()
    .unwrap();
    assert!(
        html.contains(" aria-labelledby=\"grid-row-0 grid-col-min\" aria-invalid=\"true\" aria-describedby=\"grid-hint-integer limits-0-min-error\"><div class=\"field-error\" id=\"limits-0-min-error\"><p>"),
        "{html}"
    );
}

#[test]
fn grid_component_the_save_button_is_the_first_submit_button_of_the_form() {
    let bundles = Bundles::load().unwrap();
    let tree = zones_tree();
    let html = GridFormPage {
        t: Texts::new(&bundles, Lang::En),
        action: "/recipes/R-100/versions/3/parameters".to_owned(),
        view: view(&tree, ValueTexts::default(), Vec::new(), &bundles),
        errors: FormErrors::default(),
    }
    .render()
    .unwrap();
    let submits: Vec<&str> = html
        .match_indices("type=\"submit\" id=\"")
        .filter_map(|(at, marker)| {
            let rest = html.get(at.checked_add(marker.len()).unwrap()..)?;
            rest.split('"').next()
        })
        .collect();
    assert_eq!(
        submits,
        ["parameter-save", "parameter-show-fields", "grid-mode-0"]
    );
    assert!(
        html.contains("<div class=\"actions\"><button type=\"submit\" id=\"parameter-save\" class=\"btn btn-primary\">Save parameter</button> <button type=\"submit\" id=\"parameter-show-fields\" name=\"fields\" value=\"1\" class=\"btn btn-secondary\">Show value fields</button></div>"),
        "{html}"
    );
}

#[test]
fn grid_component_every_sent_text_is_escaped() {
    let bundles = Bundles::load().unwrap();
    let tree = array_tree(Primitive::Int, &[(1, 2)]);
    let sent = vec![(
        LeafPath::parse("[1]").unwrap(),
        "\"><script>alert(1)</script>".to_owned(),
    )];
    let html = GridPage {
        t: Texts::new(&bundles, Lang::En),
        view: view(
            &tree,
            ValueTexts::from_sent(&sent),
            vec![limit("[*]", "<b>", "'x'")],
            &bundles,
        ),
        errors: FormErrors::default(),
    }
    .render()
    .unwrap();
    assert!(!html.contains("<script"), "{html}");
    assert!(!html.contains("<b>"), "{html}");
    assert!(
        html.contains("value=\"&#34;&#62;&#60;script&#62;"),
        "{html}"
    );
    assert_clean_html(&html);
    assert!(!html.contains("hx-"), "{html}");
}

#[test]
fn grid_component_the_values_fragment_shows_texts_and_no_input() {
    let bundles = Bundles::load().unwrap();
    let tree = line_tree();
    let json =
        r#"{"name":"A <b>","speed":5,"zone":{"setpoint":1.5,"hold":"T#90s","enabled":true}}"#;
    let values = ValueTexts::from_stored(&stored_texts(&tree, json));
    let grid = view(&tree, values, vec![limit(".speed", "0", "10")], &bundles);
    let html = GridValuesFragment {
        t: Texts::new(&bundles, Lang::En),
        view: &grid,
    }
    .render()
    .unwrap();
    for tag in ["<input", "<select", "<button", "<form"] {
        assert!(!html.contains(tag), "{tag}: {html}");
    }
    assert!(
        html.contains("<tr><th scope=\"row\">.speed</th><td class=\"data\">5</td><td class=\"data\">0</td><td class=\"data\">10</td><td class=\"grid-note\">not applicable</td><td class=\"data\">INT</td></tr>"),
        "{html}"
    );
    assert!(
        html.contains("<td class=\"data\">A &#60;b&#62;</td>"),
        "{html}"
    );
    assert!(html.contains("<td class=\"data\">TRUE</td>"), "{html}");
    assert!(
        html.contains("<caption id=\"grid-values-caption\">"),
        "{html}"
    );
    assert_clean_html(&html);
}

#[test]
fn grid_component_the_limits_fragment_shows_one_row_per_limit() {
    let bundles = Bundles::load().unwrap();
    let mut length = limit(".name", "", "");
    length.max_length = "8".to_owned();
    let rows = vec![limit("", "0", "3000"), length];
    let html = GridLimitsFragment {
        t: Texts::new(&bundles, Lang::En),
        rows: &rows,
        root_label: "SPEED".to_owned(),
    }
    .render()
    .unwrap();
    assert!(
        html.contains("<thead><tr><th scope=\"col\">Path</th><th scope=\"col\">Minimum</th><th scope=\"col\">Maximum</th><th scope=\"col\">Maximum length</th></tr></thead>"),
        "{html}"
    );
    assert!(
        html.contains("<tr><th scope=\"row\">SPEED</th><td class=\"data\">0</td><td class=\"data\">3000</td><td class=\"data\"></td></tr>"),
        "{html}"
    );
    assert!(
        html.contains("<tr><th scope=\"row\">.name</th><td class=\"data\"></td><td class=\"data\"></td><td class=\"data\">8</td></tr>"),
        "{html}"
    );
    assert_clean_html(&html);
}
