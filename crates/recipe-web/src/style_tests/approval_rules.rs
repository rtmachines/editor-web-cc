//! Rule tests of the diff and the formatted JSON (design D7, task 9.2, `design-system.md` 9,
//! 10.12, 10.13, and 10.14, ADR-0050, ADR-0051).
//!
//! Every rule lives in `components.css`. A rule names a color, a space, a size, and a font through
//! its token, and the new rules add no token. The state rules equal the table of section 9. The
//! contrast test of `style_tests` covers each pair of tokens that these rules use.

use super::css::{Rule, parse_tables};
use super::rules::{assert_classes_listed_and_used, rules_of, value_of};
use super::{COMPONENTS_CSS, DESIGN, app_sheet, components_sheet, root_tokens};

/// The class names of the diff and the formatted JSON. `components.css` lists them in its first
/// comment and holds at least one rule for each.
const APPROVAL_CLASSES: [&str; 12] = [
    "badge-diff-limit",
    "badge-diff-changed",
    "badge-diff-added",
    "badge-diff-removed",
    "diff-summary",
    "diff-section",
    "diff-limits",
    "diff-value",
    "diff-none",
    "json",
    "pager-end",
    "table-wrap",
];

/// The icon of each diff state in section 9 and the class that shows it.
const DIFF_BADGES: [(&str, &str); 4] = [
    ("diff-limit", ".badge-diff-limit"),
    ("diff-changed", ".badge-diff-changed"),
    ("diff-added", ".badge-diff-added"),
    ("diff-removed", ".badge-diff-removed"),
];

/// The expected declarations: selector, property, value.
const EXPECTED: &[(&str, &str, &str)] = &[
    // The summary list: one item for each state, one below the other (section 10.13).
    (".diff-summary", "display", "flex"),
    (".diff-summary", "flex-direction", "column"),
    (".diff-summary", "gap", "var(--target-gap)"),
    (".diff-summary", "list-style", "none"),
    (".diff-summary li", "display", "flex"),
    (".diff-summary li", "flex-wrap", "wrap"),
    (".diff-summary li", "align-items", "center"),
    (".diff-summary li", "gap", "var(--space-2)"),
    // The heading of a section holds the badge and the name.
    (".diff-section h2", "display", "flex"),
    (".diff-section h2", "flex-wrap", "wrap"),
    (".diff-section h2", "align-items", "center"),
    // The table of the limits section has the border of an error (section 10.13).
    (
        ".table-wrap.diff-limits",
        "border-color",
        "var(--color-error-edge)",
    ),
    // The columns of the values and of the limits align to the right, header cells too.
    (".diff-value", "text-align", "right"),
    ("td.diff-value", "font-family", "var(--font-mono)"),
    // The word of an empty side.
    (".diff-none", "color", "var(--color-text-muted)"),
    // The formatted JSON (section 10.14).
    ("pre.json", "font-family", "var(--font-mono)"),
    ("pre.json", "font-size", "var(--text-sm)"),
    ("pre.json", "background", "var(--color-sunken)"),
    ("pre.json", "color", "var(--color-text)"),
    (
        "pre.json",
        "border",
        "var(--edge) solid var(--color-border)",
    ),
    ("pre.json", "white-space", "pre-wrap"),
    ("pre.json", "overflow-wrap", "anywhere"),
    // The detail column of a table keeps a minimum width of three targets (B15).
    (
        ".table-wrap pre.json",
        "min-width",
        "calc(var(--target) * 3)",
    ),
    // The text that replaces the link "Older entries" is as high as a link (section 10.12).
    (".pager-end", "min-height", "var(--target)"),
    (".pager-end", "display", "inline-flex"),
    (".pager-end", "align-items", "center"),
];

/// Return the three tokens of the state row whose icon is `icon`.
fn state_tokens(icon: &str) -> Vec<String> {
    let rows = parse_tables(DESIGN, &["State", "Word key", "Tokens", "Icon"]).unwrap();
    let row = rows
        .iter()
        .find(|row| row[3] == icon && row[1].starts_with("diff-state-"))
        .unwrap();
    row[2]
        .split(", ")
        .map(|token| token.replace('`', ""))
        .collect()
}

#[test]
fn approval_rules_each_expected_declaration_has_the_value_of_the_design() {
    let sheet = components_sheet();
    for (selector, property, expected) in EXPECTED {
        let actual = value_of(&sheet, selector, property);
        assert_eq!(actual.as_deref(), Some(*expected), "{selector} {property}");
    }
}

#[test]
fn approval_rules_each_diff_badge_uses_the_three_tokens_of_its_row_in_section_9() {
    let sheet = components_sheet();
    for (icon, selector) in DIFF_BADGES {
        let tokens = state_tokens(icon);
        assert_eq!(tokens.len(), 3, "{icon}");
        for (property, token) in ["color", "background", "border-color"].iter().zip(&tokens) {
            let expected = format!("var({token})");
            let actual = value_of(&sheet, selector, property);
            assert_eq!(
                actual.as_deref(),
                Some(expected.as_str()),
                "{selector} {property}"
            );
        }
    }
}

#[test]
fn approval_rules_the_three_value_states_share_one_class_and_the_four_looks_differ() {
    // The states "Value changed", "Definition changed", and "Translation changed" share the tokens
    // and the icon `diff-changed`. The word tells them apart (section 9).
    let rows = parse_tables(DESIGN, &["State", "Word key", "Tokens", "Icon"]).unwrap();
    let changed: Vec<&Vec<String>> = rows.iter().filter(|row| row[3] == "diff-changed").collect();
    assert_eq!(changed.len(), 3);
    assert!(changed.iter().all(|row| row[2] == changed[0][2]));
    let sheet = components_sheet();
    let colors: Vec<Option<String>> = DIFF_BADGES
        .iter()
        .map(|(_icon, selector)| value_of(&sheet, selector, "color"))
        .collect();
    for (index, color) in colors.iter().enumerate() {
        assert!(color.is_some());
        assert_eq!(
            colors.iter().filter(|other| *other == color).count(),
            1,
            "{index}"
        );
    }
}

/// Return the foreground and background token names of a rule, or `None` without both.
fn token_pair(rule: &Rule) -> Option<(String, String)> {
    let name = |value: &str| {
        value
            .strip_prefix("var(")
            .and_then(|rest| rest.strip_suffix(')'))
            .map(str::to_owned)
    };
    Some((
        name(rule.value("color")?)?,
        name(rule.value("background")?)?,
    ))
}

#[test]
fn approval_rules_each_pair_of_a_diff_rule_has_a_row_in_the_contrast_table() {
    let pairs = parse_tables(DESIGN, &["Foreground", "Background", "Minimum"]).unwrap();
    let sheet = components_sheet();
    let selectors = [
        "pre.json",
        ".badge-diff-limit",
        ".badge-diff-changed",
        ".badge-diff-added",
        ".badge-diff-removed",
    ];
    for selector in selectors {
        let pair = rules_of(&sheet, selector, None)
            .into_iter()
            .find_map(token_pair);
        assert!(pair.is_some(), "no color pair in {selector}");
        let rule = pair.unwrap();
        let found = pairs.iter().any(|row| row[0] == rule.0 && row[1] == rule.1);
        assert!(found, "{selector}: {} on {} has no row", rule.0, rule.1);
    }
}

#[test]
fn approval_rules_the_edge_of_each_diff_state_has_a_row_against_the_surface() {
    let pairs = parse_tables(DESIGN, &["Foreground", "Background", "Minimum"]).unwrap();
    for (icon, _selector) in DIFF_BADGES {
        let edge = state_tokens(icon).pop().unwrap();
        let row = pairs
            .iter()
            .find(|row| row[0] == edge && row[1] == "--color-surface");
        assert_eq!(row.map(|row| row[2].as_str()), Some("3.0"), "{edge}");
    }
    let limits = "--color-error-edge";
    assert!(
        pairs
            .iter()
            .any(|row| row[0] == limits && row[1] == "--color-surface")
    );
}

#[test]
fn approval_rules_the_limits_frame_and_the_badges_keep_a_border_in_forced_colors() {
    let sheet = components_sheet();
    let forced = rules_of(
        &sheet,
        ".table-wrap.diff-limits",
        Some("(forced-colors: active)"),
    );
    assert!(
        forced
            .iter()
            .any(|rule| rule.value("border-color") == Some("ButtonBorder"))
    );
    // A badge keeps the border of `.badge` in the forced-colors block (section 11).
    let badge = rules_of(&sheet, ".badge", Some("(forced-colors: active)"));
    assert!(
        badge
            .iter()
            .any(|rule| rule.value("border-color") == Some("ButtonBorder"))
    );
}

#[test]
fn approval_rules_only_the_limits_frame_changes_the_border_color_of_a_table_frame() {
    // The other tables keep `--color-border` (section 10.13).
    for rule in components_sheet()
        .iter()
        .filter(|rule| rule.media.is_none())
    {
        let is_frame = rule.selectors.iter().any(|s| s.contains(".table-wrap"));
        let is_limits = rule.selectors.iter().any(|s| s.contains("diff-limits"));
        if is_frame && rule.value("border-color").is_some() {
            assert!(is_limits, "{:?}", rule.selectors);
        }
    }
}

#[test]
fn approval_rules_no_diff_rule_is_sticky_and_no_diff_table_has_a_fixed_width() {
    // A sticky cell in a frame that scrolls sideways can cover a focused control (WCAG 2.4.11).
    for rule in components_sheet().iter().filter(|rule| {
        rule.selectors
            .iter()
            .any(|s| s.contains("diff") || s.contains("json"))
    }) {
        assert_ne!(
            rule.value("position"),
            Some("sticky"),
            "{:?}",
            rule.selectors
        );
        if rule.selectors.iter().any(|s| s.contains("diff-")) {
            for property in ["width", "min-width", "max-width"] {
                assert_eq!(rule.value(property), None, "{:?}", rule.selectors);
            }
        }
    }
}

#[test]
fn approval_rules_the_json_element_wraps_long_lines_and_has_no_scroll_of_its_own() {
    let sheet = components_sheet();
    for property in ["overflow", "overflow-x", "overflow-y"] {
        assert_eq!(value_of(&sheet, "pre.json", property), None, "{property}");
    }
    // `pre-wrap` keeps the line breaks of the text and wraps long lines. `anywhere` breaks a long
    // word, so a page at 320 CSS px does not scroll sideways (section 10.14).
    assert_ne!(
        value_of(&sheet, "pre.json", "word-break").as_deref(),
        Some("keep-all")
    );
}

#[test]
fn approval_rules_the_value_columns_align_right_in_the_header_cell_too() {
    // `th` sets `text-align: left` with a type selector. The class selector of `.diff-value` has the
    // higher specificity, so it wins in both kinds of cell.
    let sheet = components_sheet();
    assert_eq!(
        value_of(&sheet, "th", "text-align").as_deref(),
        Some("left")
    );
    assert_eq!(
        value_of(&sheet, ".diff-value", "text-align").as_deref(),
        Some("right")
    );
    assert!(rules_of(&sheet, "th.diff-value", None).is_empty());
}

#[test]
fn approval_rules_the_first_comment_lists_every_class_and_each_class_has_a_rule() {
    assert_classes_listed_and_used(COMPONENTS_CSS, &components_sheet(), &APPROVAL_CLASSES);
}

#[test]
fn approval_rules_no_diff_class_lives_in_app_css() {
    let app = app_sheet();
    for class in APPROVAL_CLASSES
        .iter()
        .filter(|class| **class != "table-wrap")
    {
        let found = app
            .iter()
            .any(|rule| rule.selectors.iter().any(|s| s.contains(class)));
        assert!(!found, "app.css names .{class}");
    }
}

#[test]
fn approval_rules_the_diff_rules_add_no_token_and_no_transition() {
    // The :root block is unchanged: the diff states use the tokens of the existing states.
    let tokens = root_tokens(&app_sheet());
    for (_icon, selector) in DIFF_BADGES {
        for (name, value) in rules_of(&components_sheet(), selector, None)
            .iter()
            .flat_map(|rule| rule.decls.iter())
        {
            let token = value.trim_start_matches("var(").trim_end_matches(')');
            assert!(
                tokens.iter().any(|(other, _value)| other == token),
                "{selector} {name}: {value}"
            );
            assert!(!name.starts_with("transition"), "{selector}");
        }
    }
}
