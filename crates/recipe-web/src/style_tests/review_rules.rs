//! Rule tests of `app.css` for the code review fixes R10 to R13 and R16 (task 8.5, task 8.7).
//!
//! Each test reads one declaration that `design-system.md` sections 5, 6, 10.10, and 10.11 name.

use super::sheet;

/// Return the value of `property` in the last top-level rule whose selector list holds `selector`.
fn value_of(selector: &str, property: &str) -> Option<String> {
    sheet()
        .iter()
        .filter(|rule| rule.media.is_none())
        .filter(|rule| rule.selectors.iter().any(|s| s == selector))
        .rev()
        .find_map(|rule| rule.value(property).map(str::to_owned))
}

/// Assert that `selector` sets `property` to `expected`.
fn assert_rule(selector: &str, property: &str, expected: &str) {
    assert_eq!(
        value_of(selector, property).as_deref(),
        Some(expected),
        "{selector} {property}"
    );
}

#[test]
fn review_rules_table_data_breaks_only_between_words() {
    assert_rule(".table-wrap td", "overflow-wrap", "break-word");
    assert_rule(".table-wrap .data", "overflow-wrap", "break-word");
}

#[test]
fn review_rules_empty_state_uses_the_full_width_of_main() {
    assert_rule(".empty", "max-width", "none");
}

#[test]
fn review_rules_tree_node_aligns_its_content_to_the_start() {
    assert_rule(".tree-node", "justify-content", "flex-start");
}

#[test]
fn review_rules_a_form_in_main_has_the_look_of_a_panel() {
    assert_rule("main form", "padding", "var(--space-4)");
    assert_rule("main form", "background", "var(--color-surface)");
    assert_rule(
        "main form",
        "border",
        "var(--edge) solid var(--color-border)",
    );
    assert_rule("main form", "border-radius", "var(--radius)");
}

#[test]
fn review_rules_no_bare_form_rule_gives_the_panel_look_to_the_top_bar() {
    let sheet = sheet();
    let bare = sheet
        .iter()
        .filter(|rule| rule.selectors.iter().any(|s| s == "form"))
        .any(|rule| rule.value("padding").is_some() || rule.value("border").is_some());
    assert!(!bare, "the panel look belongs to `main form`");
}

#[test]
fn review_rules_actions_align_a_button_with_the_input_above_it() {
    assert_rule(".actions", "align-items", "flex-end");
    assert_rule(".actions .field", "margin-bottom", "0");
}

#[test]
fn review_rules_description_list_has_bold_muted_terms_and_no_indent() {
    assert_rule("dt", "font-weight", "700");
    assert_rule("dt", "color", "var(--color-text-muted)");
    assert_rule("dt", "font-size", "var(--text-sm)");
    assert_rule("dd", "margin", "0");
}

#[test]
fn review_rules_top_bar_and_table_header_keep_their_shape_in_forced_colors() {
    for selector in [".topbar", "th"] {
        assert_rule(selector, "border", "var(--edge) solid transparent");
        let sheet = sheet();
        let forced = sheet
            .iter()
            .filter(|rule| {
                rule.media
                    .as_deref()
                    .is_some_and(|m| m.contains("forced-colors"))
            })
            .filter(|rule| rule.selectors.iter().any(|s| s == selector))
            .any(|rule| rule.value("border-color") == Some("ButtonBorder"));
        assert!(
            forced,
            "{selector} needs a border color in forced-colors mode"
        );
    }
}

#[test]
fn review_rules_a_form_in_actions_has_no_box_of_its_own() {
    assert_rule(".actions form", "border", "0");
    assert_rule(".actions form", "padding", "0");
    assert_rule(".actions form", "background", "transparent");
    assert_rule(".actions form", "margin-bottom", "0");
}

#[test]
fn review_rules_tree_leaf_shows_path_and_name_with_a_gap() {
    assert_rule(".tree-leaf", "display", "flex");
    assert_rule(".tree-leaf", "gap", "var(--space-2)");
}
