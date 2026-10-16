//! Rule tests of the recipe editor: the leaf grid, the error line, and the states "Deleted" and
//! "Outdated" (design D8, task 8.4, `design-system.md` sections 9, 10.4, 10.5, and 10.8).
//!
//! Every rule lives in `components.css`. A rule names a color, a space, and an icon size through
//! its token. The state rules equal the table of section 9 of the design document.

use super::css::parse_tables;
use super::rules::{assert_classes_listed_and_used, rules_of, value_of};
use super::{COMPONENTS_CSS, DESIGN, app_sheet, components_sheet, root_tokens};

/// The class names of the recipe editor. `components.css` lists them in its first comment and
/// holds at least one rule for each.
const RECIPE_CLASSES: [&str; 9] = [
    "alert-head",
    "leaf-grid",
    "row-all",
    "grid-note",
    "grid-toggle",
    "field-error",
    "field-error-code",
    "badge-deleted",
    "badge-outdated",
];

/// The state rows of section 9 that step P3.3 adds, and the class that shows each one.
const STATE_CLASSES: [(&str, &str); 2] = [
    ("Deleted recipe (ADR-0043)", ".badge-deleted"),
    ("Outdated parameter (ADR-0043)", ".badge-outdated"),
];

/// The expected declarations: selector, property, value.
const EXPECTED: &[(&str, &str, &str)] = &[
    // The grid is a table that scrolls sideways in its frame at a narrow width (section 10.4).
    (".leaf-grid tbody[hidden]", "display", "none"),
    (
        ".leaf-grid th[scope=\"row\"]",
        "background",
        "var(--color-surface)",
    ),
    // The row header holds the leaf path in the data font.
    (
        ".leaf-grid th[scope=\"row\"]",
        "font-family",
        "var(--font-mono)",
    ),
    (".leaf-grid th[scope=\"row\"]", "color", "var(--color-text)"),
    // The path breaks only between words, as every table cell does (section 5).
    (
        ".leaf-grid th[scope=\"row\"]",
        "overflow-wrap",
        "break-word",
    ),
    // A cell with an input has a padding of one `--space-1`, so two inputs keep the target gap.
    (".leaf-grid td", "padding", "var(--space-1)"),
    (".leaf-grid td input", "width", "100%"),
    (".leaf-grid td select", "width", "100%"),
    // An input keeps the width of three targets, so the frame scrolls at 320 px, not the input.
    (
        ".leaf-grid td input",
        "min-width",
        "calc(var(--target) * 3)",
    ),
    // The row `[*]` has the ground of the table header.
    (".row-all", "background", "var(--color-sunken)"),
    (
        ".leaf-grid .row-all th[scope=\"row\"]",
        "background",
        "var(--color-sunken)",
    ),
    // The word of a cell without a limit.
    (".grid-note", "color", "var(--color-text-muted)"),
    (".grid-note", "font-size", "var(--text-sm)"),
    // The cell of a group toggle.
    (".grid-toggle", "padding", "var(--space-1)"),
    (".grid-toggle .tree-node", "margin-left", "0"),
    // The error line under an input (section 10.5).
    // The markup is `div.field-error` with one `p` for each line (`field.html`, macro `error_line`).
    (".field-error", "color", "var(--color-error-fg)"),
    (".field-error", "font-size", "var(--text-sm)"),
    (".field-error p", "display", "flex"),
    (".field-error p", "align-items", "flex-start"),
    (".field-error p", "gap", "var(--space-1)"),
    (".field-error .icon", "width", "var(--icon-md)"),
    (".field-error .icon", "height", "var(--icon-md)"),
    (".field-error-code", "font-family", "var(--font-mono)"),
    (".field-error-code", "font-weight", "700"),
    // The head of the summary of field errors: icon, code, and count (section 10.6).
    (".alert-head", "display", "flex"),
    (".alert-head", "flex-wrap", "wrap"),
    (".alert-head", "align-items", "flex-start"),
    (".alert-head", "gap", "var(--space-2) var(--space-3)"),
    (".alert-head", "flex-basis", "100%"),
    // The state "Outdated" shows the tokens of the warning (section 9).
    (".badge-outdated", "color", "var(--color-warning-fg)"),
    (".badge-outdated", "background", "var(--color-warning-bg)"),
    (
        ".badge-outdated",
        "border-color",
        "var(--color-warning-edge)",
    ),
];

/// Return the length in px of the token `name` of `:root`.
fn token_px(name: &str) -> u32 {
    let tokens = root_tokens(&super::app_sheet());
    let value = tokens
        .iter()
        .find(|(token, _value)| token == name)
        .map(|(_token, value)| value.clone())
        .unwrap();
    value.strip_suffix("px").unwrap().parse().unwrap()
}

#[test]
fn recipe_rules_each_expected_declaration_has_the_value_of_the_design() {
    let sheet = components_sheet();
    for (selector, property, expected) in EXPECTED {
        let actual = value_of(&sheet, selector, property);
        assert_eq!(actual.as_deref(), Some(*expected), "{selector} {property}");
    }
}

#[test]
fn recipe_rules_the_grid_has_no_fixed_width_so_the_frame_scrolls_only_when_the_cells_need_it() {
    let sheet = components_sheet();
    for property in ["min-width", "width", "max-width"] {
        assert_eq!(value_of(&sheet, ".leaf-grid", property), None, "{property}");
        assert_eq!(
            value_of(&sheet, ".leaf-grid th[scope=\"row\"]", property),
            None,
            "{property}"
        );
    }
}

#[test]
fn recipe_rules_no_rule_of_components_css_is_sticky_so_no_cell_can_hide_a_focused_input() {
    // A sticky cell in a frame that scrolls sideways can cover a focused input (WCAG 2.4.11, D16-9).
    // The check covers the rules inside `@media` too.
    for rule in components_sheet() {
        assert_ne!(
            rule.value("position"),
            Some("sticky"),
            "{:?}",
            rule.selectors
        );
    }
}

#[test]
fn recipe_rules_the_comment_says_that_the_grid_stands_inside_a_table_wrap_frame() {
    let start = COMPONENTS_CSS.find(" * Leaf grid (").unwrap();
    let comment = COMPONENTS_CSS.get(start..).unwrap();
    let end = comment.find("*/").unwrap();
    assert!(comment.get(..end).unwrap().contains(".table-wrap"));
}

#[test]
fn recipe_rules_the_first_comment_lists_every_class_and_each_class_has_a_rule() {
    assert_classes_listed_and_used(COMPONENTS_CSS, &components_sheet(), &RECIPE_CLASSES);
}

#[test]
fn recipe_rules_no_recipe_class_lives_in_app_css() {
    let app = app_sheet();
    for class in RECIPE_CLASSES {
        let found = app
            .iter()
            .any(|rule| rule.selectors.iter().any(|s| s.contains(class)));
        assert!(!found, "app.css names .{class}");
    }
}

#[test]
fn recipe_rules_the_deleted_and_outdated_badges_use_the_tokens_of_the_state_table() {
    let rows = parse_tables(DESIGN, &["State", "Word key", "Tokens", "Icon"]).unwrap();
    let sheet = components_sheet();
    for (state, selector) in STATE_CLASSES {
        let row = rows.iter().find(|row| row[0] == state).unwrap();
        let tokens: Vec<String> = row[2]
            .split(", ")
            .map(|token| token.replace('`', ""))
            .collect();
        assert_eq!(tokens.len(), 3, "{state}");
        for (property, token) in ["color", "background", "border-color"].iter().zip(&tokens) {
            let expected = format!("var({token})");
            let actual = value_of(&sheet, selector, property);
            assert_eq!(
                actual.as_deref(),
                Some(expected.as_str()),
                "{state} {property}"
            );
        }
    }
}

#[test]
fn recipe_rules_the_deleted_badge_looks_like_the_retired_badge() {
    let sheet = components_sheet();
    for property in ["color", "background", "border-color"] {
        assert_eq!(
            value_of(&sheet, ".badge-deleted", property),
            value_of(&sheet, ".badge-retired", property),
            "{property}"
        );
    }
}

#[test]
fn recipe_rules_the_outdated_and_the_deleted_badge_differ_so_the_color_is_not_the_only_signal() {
    let sheet = components_sheet();
    assert_ne!(
        value_of(&sheet, ".badge-outdated", "color"),
        value_of(&sheet, ".badge-deleted", "color")
    );
    // The word and the icon come with the badge macros. The icon of Outdated is not the icon of
    // Deleted (`design-system.md` section 9).
    let rows = parse_tables(DESIGN, &["State", "Word key", "Tokens", "Icon"]).unwrap();
    let icon = |state: &str| {
        rows.iter()
            .find(|row| row[0] == state)
            .map(|row| row[3].clone())
    };
    assert_ne!(icon(STATE_CLASSES[0].0), icon(STATE_CLASSES[1].0));
}

#[test]
fn recipe_rules_two_inputs_of_adjacent_cells_keep_the_target_gap() {
    let padding = token_px("--space-1");
    let gap = token_px("--target-gap");
    assert_eq!(padding.checked_mul(2).unwrap(), gap);
    let sheet = components_sheet();
    assert_eq!(
        value_of(&sheet, ".leaf-grid td", "padding").as_deref(),
        Some("var(--space-1)")
    );
    assert_eq!(
        value_of(&sheet, ".grid-toggle", "padding").as_deref(),
        Some("var(--space-1)")
    );
}

#[test]
fn recipe_rules_an_input_with_an_error_has_the_error_edge_in_both_sheets_together() {
    let app = app_sheet();
    for selector in [
        "input[aria-invalid=\"true\"]",
        "select[aria-invalid=\"true\"]",
    ] {
        assert_eq!(
            value_of(&app, selector, "border-color").as_deref(),
            Some("var(--color-error-edge)"),
            "{selector}"
        );
    }
}

#[test]
fn recipe_rules_the_error_line_names_only_error_tokens_for_its_colors() {
    let sheet = components_sheet();
    for rule in rules_of(&sheet, ".field-error", None) {
        for (name, value) in &rule.decls {
            if name == "color" || name.starts_with("border") || name == "background" {
                assert!(value.contains("--color-error"), "{name}: {value}");
            }
        }
    }
}

#[test]
fn recipe_rules_the_error_line_shrinks_and_wraps_inside_a_narrow_cell() {
    let sheet = components_sheet();
    assert_eq!(
        value_of(&sheet, ".field-error p > *", "min-width").as_deref(),
        Some("0")
    );
    // The sentence breaks between words, never inside a word (design-system.md 5, RQ-4).
    assert_eq!(
        value_of(&sheet, ".field-error", "overflow-wrap").as_deref(),
        Some("break-word")
    );
}

#[test]
fn recipe_rules_the_error_line_wraps_below_the_code_when_the_cell_is_narrow() {
    let sheet = components_sheet();
    assert_eq!(
        value_of(&sheet, ".field-error p", "flex-wrap").as_deref(),
        Some("wrap")
    );
    // The sentence keeps a minimum width of three targets. A narrower room wraps it below the code.
    assert_eq!(
        value_of(&sheet, ".field-error p > :last-child", "flex").as_deref(),
        Some("1 1 calc(var(--target) * 3)")
    );
}

#[test]
fn recipe_rules_no_rule_of_the_error_line_breaks_a_word_into_pieces() {
    let sheet = components_sheet();
    for rule in sheet
        .iter()
        .filter(|rule| rule.selectors.iter().any(|s| s.starts_with(".field-error")))
    {
        assert_ne!(
            rule.value("overflow-wrap"),
            Some("anywhere"),
            "{:?}",
            rule.selectors
        );
        assert_ne!(
            rule.value("word-break"),
            Some("break-all"),
            "{:?}",
            rule.selectors
        );
    }
}

#[test]
fn recipe_rules_the_code_of_a_summary_line_never_breaks_and_the_sentence_still_wraps() {
    let sheet = components_sheet();
    // `.alert-code` stands inside the inline-flex `.target-link` of a summary line (RQ-3).
    assert_eq!(
        value_of(&sheet, ".alert-code", "white-space").as_deref(),
        Some("nowrap")
    );
    assert_eq!(
        value_of(&sheet, ".alert-code", "flex").as_deref(),
        Some("none")
    );
    // No rule makes the summary list or its sentence unbreakable.
    for selector in [".field-lines", ".field-lines li", ".target-link"] {
        assert_ne!(
            value_of(&app_sheet(), selector, "white-space").as_deref(),
            Some("nowrap"),
            "{selector}"
        );
        assert_ne!(
            value_of(&sheet, selector, "white-space").as_deref(),
            Some("nowrap"),
            "{selector}"
        );
    }
}

#[test]
fn recipe_rules_a_summary_link_wraps_its_sentence_below_the_code_inside_a_narrow_panel() {
    let sheet = components_sheet();
    // The fixed-width code and a long leaf path must not widen the link past the panel (RQ2-1).
    assert_eq!(
        value_of(&sheet, ".field-lines .target-link", "flex-wrap").as_deref(),
        Some("wrap")
    );
    let message = ".field-lines .target-link > :last-child";
    assert_eq!(value_of(&sheet, message, "min-width").as_deref(), Some("0"));
    assert_eq!(
        value_of(&sheet, message, "flex").as_deref(),
        Some("1 1 calc(var(--target) * 3)")
    );
    assert_eq!(
        value_of(&sheet, message, "overflow-wrap").as_deref(),
        Some("break-word")
    );
    // The code stays whole in the same link.
    assert_eq!(
        value_of(&sheet, ".alert-code", "white-space").as_deref(),
        Some("nowrap")
    );
}

#[test]
fn recipe_rules_a_summary_link_is_a_block_level_flex_box_so_a_long_word_cannot_widen_it() {
    // An inline-flex box is never narrower than its longest word. A flex box fits the panel (RQ2-2).
    assert_eq!(
        value_of(&components_sheet(), ".field-lines .target-link", "display").as_deref(),
        Some("flex")
    );
}

#[test]
fn recipe_rules_the_summary_link_rules_use_tokens_and_no_forced_word_break() {
    let sheet = components_sheet();
    let rules: Vec<_> = sheet
        .iter()
        .filter(|rule| {
            rule.selectors
                .iter()
                .any(|s| s.starts_with(".field-lines .target-link"))
        })
        .collect();
    assert!(!rules.is_empty());
    for rule in rules {
        assert_ne!(rule.value("overflow-wrap"), Some("anywhere"));
        assert_ne!(rule.value("word-break"), Some("break-all"));
        for (name, value) in &rule.decls {
            assert!(!value.contains("px"), "{name}: {value}");
        }
    }
}

#[test]
fn recipe_rules_a_form_with_a_leaf_grid_uses_the_full_width_of_main() {
    // The base rule keeps every other form at `--width-form` (section 6).
    assert_eq!(
        value_of(&app_sheet(), "form", "max-width").as_deref(),
        Some("var(--width-form)")
    );
    assert_eq!(
        value_of(&components_sheet(), "form:has(.leaf-grid)", "max-width").as_deref(),
        Some("none")
    );
}

#[test]
fn recipe_rules_every_other_child_of_a_form_with_a_leaf_grid_keeps_the_form_width() {
    // The fields, the notice, and the actions row stand beside the grid holder `.tree`.
    // Section 6: a form uses at most `--width-form`.
    assert_eq!(
        value_of(
            &components_sheet(),
            "form:has(.leaf-grid) > :not(.tree)",
            "max-width"
        )
        .as_deref(),
        Some("var(--width-form)")
    );
}

#[test]
fn recipe_rules_the_grid_frame_keeps_the_full_width_of_main() {
    // Section 6: a table uses the full width of `main`. No rule narrows the holder or the frame.
    let sheet = components_sheet();
    for selector in [".tree", ".table-wrap", ".leaf-grid"] {
        assert_eq!(value_of(&sheet, selector, "max-width"), None, "{selector}");
        assert_eq!(value_of(&sheet, selector, "width"), None, "{selector}");
    }
    let grid = include_str!("../../templates/components/grid.html");
    assert!(
        grid.contains("{%- macro grid(t, view, errors) -%}<div class=\"tree\">"),
        "the grid macro starts with the holder that the width rule excludes"
    );
}

#[test]
fn recipe_rules_the_form_width_comment_cites_the_rules_that_the_code_follows() {
    let raw = COMPONENTS_CSS
        .split("form:has(.leaf-grid) {")
        .next()
        .and_then(|head| head.rsplit("/*").next())
        .unwrap_or_default();
    // Join the comment lines into one text without the star margin.
    let comment = raw
        .replace('*', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(comment.contains("design-system.md 10.4"), "{comment}");
    assert!(
        comment.contains("A table uses the full width of `main`"),
        "{comment}"
    );
    assert!(
        comment.contains("a form uses at most `--width-form`"),
        "{comment}"
    );
    assert!(!comment.contains("exception"), "{comment}");
}

#[test]
fn recipe_rules_the_path_cell_of_the_grid_never_breaks_inside_a_word() {
    let sheet = components_sheet();
    for rule in rules_of(&sheet, ".leaf-grid th[scope=\"row\"]", None) {
        assert_ne!(rule.value("overflow-wrap"), Some("anywhere"));
        assert_ne!(rule.value("word-break"), Some("break-all"));
    }
}

#[test]
fn recipe_rules_the_error_code_never_breaks_and_the_sentence_still_wraps() {
    let sheet = components_sheet();
    assert_eq!(
        value_of(&sheet, ".field-error-code", "white-space").as_deref(),
        Some("nowrap")
    );
    assert_eq!(
        value_of(&sheet, ".field-error-code", "flex").as_deref(),
        Some("none")
    );
    // The sentence is the other flex item. It wraps between words (RQ-4).
    assert_eq!(
        value_of(&sheet, ".field-error", "overflow-wrap").as_deref(),
        Some("break-word")
    );
}

#[test]
fn recipe_rules_the_grid_adds_no_inline_size_and_no_transition() {
    let sheet = components_sheet();
    for rule in sheet.iter().filter(|rule| {
        rule.selectors
            .iter()
            .any(|s| RECIPE_CLASSES.iter().any(|class| s.contains(class)))
    }) {
        for (name, _value) in &rule.decls {
            assert!(!name.starts_with("transition"), "{:?}", rule.selectors);
            assert!(!name.starts_with("animation"), "{:?}", rule.selectors);
        }
    }
}
