//! Rule tests of the style sheets for the code review fixes R10 to R13 and R16 (task 8.5, task 8.7).
//!
//! Each test reads one declaration that `design-system.md` sections 5, 6, 10.10, and 10.11 name.
//! A test reads the style sheet that holds the rule: `app.css` for a base rule and `components.css`
//! for a component (ADR-0037 r).

use super::css::Rule;
use super::{app_sheet, components_sheet, sheet};

/// Return the value of `property` in the last top-level rule of `rules` whose selector list holds
/// `selector`.
fn value_of(rules: &[Rule], selector: &str, property: &str) -> Option<String> {
    rules
        .iter()
        .filter(|rule| rule.media.is_none())
        .filter(|rule| rule.selectors.iter().any(|s| s == selector))
        .rev()
        .find_map(|rule| rule.value(property).map(str::to_owned))
}

/// Assert that `selector` sets `property` to `expected` in `rules`.
fn assert_rule(rules: &[Rule], selector: &str, property: &str, expected: &str) {
    assert_eq!(
        value_of(rules, selector, property).as_deref(),
        Some(expected),
        "{selector} {property}"
    );
}

/// Return true when `rules` hold a forced-colors rule for `selector` with a `ButtonBorder` border.
fn keeps_border_in_forced_colors(rules: &[Rule], selector: &str) -> bool {
    rules
        .iter()
        .filter(|rule| {
            rule.media
                .as_deref()
                .is_some_and(|m| m.contains("forced-colors"))
        })
        .filter(|rule| rule.selectors.iter().any(|s| s == selector))
        .any(|rule| rule.value("border-color") == Some("ButtonBorder"))
}

#[test]
fn review_rules_table_data_breaks_only_between_words() {
    let rules = components_sheet();
    assert_rule(&rules, ".table-wrap td", "overflow-wrap", "break-word");
    assert_rule(&rules, ".table-wrap .data", "overflow-wrap", "break-word");
}

#[test]
fn review_rules_empty_state_uses_the_full_width_of_main() {
    assert_rule(&components_sheet(), ".empty", "max-width", "none");
}

#[test]
fn review_rules_tree_node_aligns_its_content_to_the_start() {
    assert_rule(
        &components_sheet(),
        ".tree-node",
        "justify-content",
        "flex-start",
    );
}

#[test]
fn review_rules_a_form_in_main_has_the_look_of_a_panel() {
    let rules = app_sheet();
    assert_rule(&rules, "main form", "padding", "var(--space-4)");
    assert_rule(&rules, "main form", "background", "var(--color-surface)");
    assert_rule(
        &rules,
        "main form",
        "border",
        "var(--edge) solid var(--color-border)",
    );
    assert_rule(&rules, "main form", "border-radius", "var(--radius)");
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
    let rules = app_sheet();
    assert_rule(&rules, ".actions", "align-items", "flex-end");
    assert_rule(&rules, ".actions .field", "margin-bottom", "0");
}

#[test]
fn review_rules_description_list_has_bold_muted_terms_and_no_indent() {
    let rules = app_sheet();
    assert_rule(&rules, "dt", "font-weight", "700");
    assert_rule(&rules, "dt", "color", "var(--color-text-muted)");
    assert_rule(&rules, "dt", "font-size", "var(--text-sm)");
    assert_rule(&rules, "dd", "margin", "0");
}

#[test]
fn review_rules_top_bar_and_table_header_keep_their_shape_in_forced_colors() {
    for (selector, rules) in [(".topbar", app_sheet()), ("th", components_sheet())] {
        assert_rule(&rules, selector, "border", "var(--edge) solid transparent");
        assert!(
            keeps_border_in_forced_colors(&rules, selector),
            "{selector} needs a border color in forced-colors mode"
        );
    }
}

#[test]
fn review_rules_a_form_in_actions_has_no_box_of_its_own() {
    let rules = app_sheet();
    assert_rule(&rules, ".actions form", "border", "0");
    assert_rule(&rules, ".actions form", "padding", "0");
    assert_rule(&rules, ".actions form", "background", "transparent");
    assert_rule(&rules, ".actions form", "margin-bottom", "0");
}

#[test]
fn review_rules_tree_leaf_shows_path_and_name_with_a_gap() {
    let rules = components_sheet();
    assert_rule(&rules, ".tree-leaf", "display", "flex");
    assert_rule(&rules, ".tree-leaf", "gap", "var(--space-2)");
}
