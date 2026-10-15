//! Rule tests of the fixes of the design review and the browser test (design D20: U5 to U8, U15,
//! U16, and U32; task 13.10 and 13.11).
//!
//! Each test reads one declaration. A rule of a base class lives in `app.css`. A new rule of a
//! component lives in `components.css` (design D9).

use super::rules::value_of;
use super::{app_sheet, components_sheet, root_tokens};

/// Assert that `selector` sets `property` to `expected` in `sheet`.
fn assert_rule(sheet: &[super::css::Rule], selector: &str, property: &str, expected: &str) {
    assert_eq!(
        value_of(sheet, selector, property).as_deref(),
        Some(expected),
        "{selector} {property}"
    );
}

/// Return the length in px of the token `name` of `:root`.
fn token_px(name: &str) -> u32 {
    let tokens = root_tokens(&app_sheet());
    let value = tokens
        .iter()
        .find(|(token, _value)| token == name)
        .map(|(_token, value)| value.clone())
        .unwrap();
    value.strip_suffix("px").unwrap().parse().unwrap()
}

/// The width of the unit path column of a tree row: the icon and the gap after it.
const PATH_COLUMN: &str = "calc(var(--icon-md) + var(--space-2))";

// --- U5: the path and the name of a link have a space ---------------------------------------------

#[test]
fn fix_rules_a_stand_alone_link_keeps_a_gap_between_its_flex_items() {
    assert_rule(&app_sheet(), ".target-link", "gap", "var(--space-2)");
}

// --- U6: the links of the unit page stand in a column with the gap of two targets ------------------

#[test]
fn fix_rules_a_list_of_links_is_a_flex_column_with_the_gap_of_two_targets() {
    let rules = components_sheet();
    assert_rule(&rules, ".link-list", "display", "flex");
    assert_rule(&rules, ".link-list", "flex-direction", "column");
    assert_rule(&rules, ".link-list", "gap", "var(--target-gap)");
    assert_rule(&rules, ".link-list", "list-style", "none");
}

// --- RQ-4: a link of a list keeps the width of its text -------------------------------------------

#[test]
fn fix_rules_a_list_of_links_keeps_each_link_as_wide_as_its_text() {
    assert_rule(
        &components_sheet(),
        ".link-list",
        "align-items",
        "flex-start",
    );
}

// --- RQ-1: a parent row and its first child row are 16 px apart ----------------------------------

#[test]
fn fix_rules_a_tree_group_starts_16_px_below_its_parent_row() {
    let rules = components_sheet();
    assert_rule(&rules, ".tree-group", "margin-top", "var(--space-4)");
    assert_eq!(token_px("--space-4"), 16);
}

// --- U8: a field in `.actions` shrinks and fills its row -----------------------------------------

#[test]
fn fix_rules_a_field_in_actions_has_a_flexible_width_and_may_shrink() {
    let rules = app_sheet();
    assert_rule(&rules, ".actions .field", "min-width", "0");
    assert_rule(
        &rules,
        ".actions .field",
        "flex",
        "1 1 calc(var(--width-form) / 3)",
    );
    assert_rule(&rules, ".actions .field", "margin-bottom", "0");
}

#[test]
fn fix_rules_an_input_in_actions_fills_its_field_and_may_shrink() {
    let rules = app_sheet();
    for selector in [".actions .field input", ".actions .field select"] {
        assert_rule(&rules, selector, "width", "100%");
        assert_rule(&rules, selector, "min-width", "0");
    }
}

// --- U16: the gaps and the empty state in a table -------------------------------------------------

#[test]
fn fix_rules_the_large_badge_and_the_table_keep_the_gap_of_a_form_field_below() {
    let rules = components_sheet();
    assert_rule(&rules, ".badge-large", "margin-bottom", "var(--space-4)");
    assert_rule(&rules, ".table-wrap", "margin-bottom", "var(--space-4)");
}

#[test]
fn fix_rules_the_empty_state_in_a_table_has_no_border_of_its_own() {
    let rules = components_sheet();
    assert_rule(&rules, ".table-wrap .empty", "border", "0");
    assert_rule(
        &rules,
        ".empty",
        "border",
        "var(--edge) solid var(--color-border)",
    );
}

// --- U7: the tree row ------------------------------------------------------------------------------

#[test]
fn fix_rules_a_wrapped_tree_row_starts_at_the_column_of_the_unit_path() {
    let rules = components_sheet();
    assert_rule(&rules, ".tree-row", "padding-left", PATH_COLUMN);
    assert_rule(
        &rules,
        ".tree-node",
        "margin-left",
        "calc((var(--icon-md) + var(--space-2)) * -1)",
    );
    // A leaf row already stands behind the padding of its list item. It takes no second indent.
    assert_rule(&rules, ".tree-leaf .tree-row", "padding-left", "0");
    assert_rule(&rules, ".tree-leaf", "padding-left", PATH_COLUMN);
}

#[test]
fn fix_rules_two_targets_of_a_row_keep_the_target_gap() {
    assert_rule(&components_sheet(), ".tree-row", "gap", "var(--target-gap)");
    assert_eq!(token_px("--target-gap"), 8);
}

#[test]
fn fix_rules_two_rows_have_a_larger_gap_than_two_targets_of_one_row() {
    let rules = components_sheet();
    for list in [".tree", ".tree-group"] {
        assert_rule(&rules, list, "gap", "var(--space-4)");
    }
    assert!(token_px("--space-4") > token_px("--target-gap"));
}

// --- U32: the member row ---------------------------------------------------------------------------

#[test]
fn fix_rules_a_member_row_leaves_room_for_the_type_input() {
    let rules = components_sheet();
    assert_rule(&rules, ".member-row", "padding", "var(--space-2)");
    assert_rule(&rules, ".member-fields .field", "min-width", "0");
    assert_rule(&rules, ".member-fields", "flex-wrap", "wrap");
}

// --- U15: the current revision and the member type -------------------------------------------------

#[test]
fn fix_rules_the_current_link_has_a_look_that_is_not_only_a_color() {
    let rules = components_sheet();
    let current = ".target-link[aria-current=\"page\"]";
    assert_rule(&rules, current, "font-weight", "700");
    assert_rule(
        &rules,
        current,
        "border",
        "var(--edge-control) solid var(--color-action)",
    );
    assert_rule(&rules, current, "border-radius", "var(--radius)");
    assert_rule(&rules, current, "padding", "0 var(--space-2)");
}

#[test]
fn fix_rules_the_member_type_in_a_tree_is_muted() {
    assert_rule(
        &components_sheet(),
        ".tree-type",
        "color",
        "var(--color-text-muted)",
    );
}
