//! Rule tests of `components.css` and of the split into two style sheets (design D9, ADR-0037 r).
//!
//! `COMPONENT_EXPECTED` pins the declarations of the components that the design names. The other
//! tests keep the two sheets apart: each sheet has at most 800 lines, only `app.css` holds `:root`
//! and the fonts, and each sheet has its own block for the forced-colors mode.

use super::css::Rule;
use super::rules::{assert_classes_listed_and_used, rules_of, value_of};
use super::{APP_CSS, COMPONENTS_CSS, app_sheet, components_sheet};

/// The media condition of the forced-colors blocks.
const FORCED_COLORS: &str = "(forced-colors: active)";

/// The longest style sheet in lines (ADR-0037 r).
const MAX_SHEET_LINES: usize = 800;

/// The class names of the components that the templates use. `components.css` lists them in its
/// first comment and holds at least one rule for each.
const COMPONENT_CLASSES: [&str; 34] = [
    "panel",
    "table-wrap",
    "num",
    "badge",
    "badge-draft",
    "badge-approved",
    "badge-retired",
    "badge-active",
    "badge-inactive",
    "badge-large",
    "alert",
    "alert-error",
    "alert-warning",
    "alert-info",
    "alert-success",
    "alert-code",
    "alert-ref",
    "field-lines",
    "toast",
    "tree",
    "tree-node",
    "tree-path",
    "tree-leaf",
    "tree-group",
    "tree-row",
    "empty",
    "danger",
    "pager",
    "pager-current",
    "check",
    "member-row",
    "member-fields",
    "link-list",
    "tree-type",
];

/// The selectors that the forced-colors block of `components.css` gives a `ButtonBorder` border.
const COMPONENT_FORCED: [&str; 9] = [
    "th",
    ".badge",
    ".alert",
    ".toast",
    ".panel",
    ".empty",
    ".table-wrap",
    ".tree-node",
    "fieldset",
];

/// The selectors that the forced-colors block of `app.css` gives a `ButtonBorder` border.
const APP_FORCED: [&str; 5] = [".topbar", "main form", ".btn", "input", "select"];

/// The selectors that only `components.css` names at the top level.
const COMPONENT_ONLY: [&str; 20] = [
    ".panel",
    ".table-wrap",
    "table",
    "caption",
    "th",
    "tr",
    ".num",
    ".empty",
    ".badge",
    ".alert",
    ".field-lines",
    ".toast",
    ".tree",
    ".tree-leaf",
    ".tree-row",
    ".danger",
    ".pager",
    ".check",
    "fieldset",
    ".member-row",
];

/// The expected declarations of `components.css`: selector, property, value.
const COMPONENT_EXPECTED: &[(&str, &str, &str)] = &[
    // The table (task 2.4, section 10.4).
    (".table-wrap", "overflow-x", "auto"),
    (
        ".table-wrap",
        "border",
        "var(--edge) solid var(--color-border)",
    ),
    ("table", "width", "100%"),
    ("th", "padding", "0 var(--space-3)"),
    ("td", "padding", "0 var(--space-3)"),
    ("th", "background", "var(--color-sunken)"),
    ("th", "color", "var(--color-text-muted)"),
    ("th", "font-size", "var(--text-sm)"),
    ("tr", "height", "var(--target)"),
    (
        "td",
        "border-bottom",
        "var(--edge) solid var(--color-divider)",
    ),
    ("tbody tr:hover", "background", "var(--color-sunken)"),
    (".num", "text-align", "right"),
    (".num", "font-family", "var(--font-mono)"),
    // The badge (section 10.9).
    (".badge", "border", "var(--edge-badge) solid"),
    (".badge", "font-size", "var(--text-sm)"),
    (".badge .icon", "width", "var(--icon-sm)"),
    (".badge-large", "font-size", "var(--text-base)"),
    (".badge-large .icon", "width", "var(--icon-md)"),
    (".badge-draft", "color", "var(--color-draft-fg)"),
    (".badge-draft", "background", "var(--color-draft-bg)"),
    (".badge-draft", "border-color", "var(--color-draft-edge)"),
    (
        ".badge-approved",
        "border-color",
        "var(--color-approved-edge)",
    ),
    (
        ".badge-retired",
        "border-color",
        "var(--color-retired-edge)",
    ),
    (".badge-active", "border-color", "var(--color-success-edge)"),
    (".badge-inactive", "color", "var(--color-retired-fg)"),
    // The alert, the field lines, and the toast (sections 10.6 and 10.7).
    (".alert", "border", "var(--edge) solid"),
    (".alert .icon", "width", "var(--icon-lg)"),
    (".alert-error", "color", "var(--color-error-fg)"),
    (".alert-error", "background", "var(--color-error-bg)"),
    (".alert-error", "border-color", "var(--color-error-edge)"),
    (
        ".alert-warning",
        "border-color",
        "var(--color-warning-edge)",
    ),
    (".alert-info", "border-color", "var(--color-info-edge)"),
    (
        ".alert-success",
        "border-color",
        "var(--color-success-edge)",
    ),
    (".alert-code", "font-family", "var(--font-mono)"),
    (".alert-code", "font-weight", "700"),
    (".alert-ref", "font-size", "var(--text-sm)"),
    (".field-lines", "display", "flex"),
    (".field-lines", "flex-direction", "column"),
    (".field-lines", "gap", "var(--target-gap)"),
    (".toast", "background", "var(--color-topbar)"),
    (".toast", "color", "var(--color-on-action)"),
    (".toast .icon", "color", "var(--color-success-bg)"),
    // The tree, the empty state, and the panel (sections 10.8, 10.10, 6).
    (".tree-group[hidden]", "display", "none"),
    (
        ".tree-node[aria-expanded=\"true\"] [data-icon=\"tree-collapsed\"]",
        "display",
        "none",
    ),
    (
        ".tree-node[aria-expanded=\"false\"] [data-icon=\"tree-expanded\"]",
        "display",
        "none",
    ),
    (".tree-path", "font-family", "var(--font-mono)"),
    (
        ".tree-leaf",
        "padding-left",
        "calc(var(--icon-md) + var(--space-2))",
    ),
    (".empty", "padding", "var(--space-7)"),
    (".empty", "background", "var(--color-surface)"),
    (".panel", "border", "var(--edge) solid var(--color-border)"),
    (".empty", "border", "var(--edge) solid var(--color-border)"),
    // The tree row, the group, and the path of a unit (design D9, ADR-0037 c).
    (".tree-row", "display", "flex"),
    (".tree-row", "flex-wrap", "wrap"),
    (".tree-row", "align-items", "center"),
    (".tree-row", "gap", "var(--target-gap)"),
    (".tree", "display", "flex"),
    (".tree", "flex-direction", "column"),
    // Two rows keep a larger gap than two targets of one row (design D20, U7).
    (".tree", "gap", "var(--space-4)"),
    (".tree-group", "padding-left", "var(--space-4)"),
    // A parent row and its first child row are two rows, so they keep 16 px (ADR-0037 c, RQ-1).
    (".tree-group", "margin-top", "var(--space-4)"),
    (".tree-path", "white-space", "nowrap"),
    // The danger disclosure (design D9, section 10.5).
    ("summary", "display", "flex"),
    ("summary", "align-items", "center"),
    ("summary", "min-height", "var(--target)"),
    ("summary", "min-width", "var(--target)"),
    ("summary", "cursor", "pointer"),
    (
        "details[open] > summary [data-icon=\"tree-collapsed\"]",
        "display",
        "none",
    ),
    (
        "details:not([open]) > summary [data-icon=\"tree-expanded\"]",
        "display",
        "none",
    ),
    (".danger", "margin-bottom", "var(--space-5)"),
    // The pager (design D10, section 6: adjacent targets keep the gap).
    (".pager", "display", "flex"),
    (".pager", "flex-wrap", "wrap"),
    (".pager", "align-items", "center"),
    (".pager", "gap", "var(--target-gap)"),
    (".pager-current", "min-height", "var(--target)"),
    // The select and the checkbox (section 10.5). The label is the target of the checkbox.
    ("select", "cursor", "pointer"),
    (".check", "display", "flex"),
    (".check", "align-items", "center"),
    (".check", "gap", "var(--space-2)"),
    (".check", "min-height", "var(--target)"),
    (".check", "min-width", "var(--target)"),
    (".check", "font-weight", "400"),
    (".check input", "width", "var(--icon-md)"),
    (".check input", "height", "var(--icon-md)"),
    (".check input", "min-height", "0"),
    // The member rows: one fieldset for the list, one for each row (section 10.5).
    ("fieldset", "min-width", "0"),
    (
        "fieldset",
        "border",
        "var(--edge) solid var(--color-border)",
    ),
    ("legend", "font-weight", "700"),
    (".member-row", "background", "var(--color-sunken)"),
    (".member-fields", "display", "flex"),
    (".member-fields", "flex-wrap", "wrap"),
    (".member-fields", "gap", "var(--space-4)"),
    (
        ".member-fields .field",
        "flex",
        "1 1 calc(var(--width-form) / 3)",
    ),
];

/// Return true when `rules` hold a forced-colors rule for `selector` with a `ButtonBorder` border.
fn has_forced_border(rules: &[Rule], selector: &str) -> bool {
    rules_of(rules, selector, Some(FORCED_COLORS))
        .iter()
        .any(|rule| rule.value("border-color") == Some("ButtonBorder"))
}

#[test]
fn components_each_style_sheet_has_at_most_800_lines() {
    for (name, css) in [("app.css", APP_CSS), ("components.css", COMPONENTS_CSS)] {
        let lines = css.lines().count();
        assert!(lines <= MAX_SHEET_LINES, "{name} has {lines} lines");
        assert!(lines > 0, "{name} is empty");
    }
}

#[test]
fn components_only_app_css_holds_root_and_the_font_faces() {
    let app = app_sheet();
    let roots = app
        .iter()
        .filter(|rule| rule.selectors.iter().any(|s| s == ":root"))
        .count();
    assert_eq!(roots, 1, "app.css needs exactly one :root block");
    assert!(app.iter().any(|rule| rule.selectors == ["@font-face"]));
    let components = components_sheet();
    assert!(
        components
            .iter()
            .all(|rule| rule.selectors.iter().all(|s| !s.contains(":root"))),
        "components.css holds a :root rule"
    );
    assert!(!COMPONENTS_CSS.contains(":root"));
    assert!(
        components
            .iter()
            .all(|rule| rule.selectors != ["@font-face"]),
        "components.css holds a font face"
    );
}

#[test]
fn components_css_has_its_own_forced_colors_block() {
    let components = components_sheet();
    let in_block = components
        .iter()
        .filter(|rule| rule.media.as_deref() == Some(FORCED_COLORS))
        .count();
    assert!(in_block > 0, "components.css has no forced-colors block");
    for selector in COMPONENT_FORCED {
        assert!(
            has_forced_border(&components, selector),
            "{selector} keeps no border in forced-colors mode"
        );
    }
}

#[test]
fn components_forced_colors_selectors_stay_in_their_own_sheet() {
    let (app, components) = (app_sheet(), components_sheet());
    for selector in APP_FORCED {
        assert!(
            has_forced_border(&app, selector),
            "app.css lacks {selector}"
        );
        assert!(
            rules_of(&components, selector, Some(FORCED_COLORS)).is_empty(),
            "components.css names {selector} in forced-colors mode"
        );
    }
    for selector in COMPONENT_FORCED {
        assert!(
            rules_of(&app, selector, Some(FORCED_COLORS)).is_empty(),
            "app.css names {selector} in forced-colors mode"
        );
    }
}

#[test]
fn components_forced_colors_keeps_the_badge_visible() {
    assert!(has_forced_border(&components_sheet(), ".badge"));
}

#[test]
fn components_a_component_rule_is_not_in_app_css() {
    let app = app_sheet();
    for selector in COMPONENT_ONLY {
        assert!(
            rules_of(&app, selector, None).is_empty(),
            "app.css holds a rule for {selector}"
        );
        assert!(
            !rules_of(&components_sheet(), selector, None).is_empty(),
            "components.css lacks a rule for {selector}"
        );
    }
}

#[test]
fn components_each_expected_declaration_has_the_value_of_the_design() {
    let sheet = components_sheet();
    for (selector, property, expected) in COMPONENT_EXPECTED {
        let actual = value_of(&sheet, selector, property);
        assert_eq!(actual.as_deref(), Some(*expected), "{selector} {property}");
    }
}

#[test]
fn components_a_tree_row_keeps_the_gap_of_two_targets_and_the_lists_keep_a_larger_gap() {
    let sheet = components_sheet();
    assert_eq!(
        value_of(&sheet, ".tree-row", "display").as_deref(),
        Some("flex")
    );
    assert_eq!(
        value_of(&sheet, ".tree-row", "gap").as_deref(),
        Some("var(--target-gap)")
    );
    for list in [".tree", ".tree-group"] {
        assert_eq!(
            value_of(&sheet, list, "gap").as_deref(),
            Some("var(--space-4)"),
            "{list}"
        );
    }
}

#[test]
fn components_a_tree_node_does_not_fill_the_row() {
    // The node shares its row with two links (design D17), so it has no fixed width.
    assert_eq!(value_of(&components_sheet(), ".tree-node", "width"), None);
}

#[test]
fn components_a_summary_is_a_target_that_shows_one_of_the_two_chevrons() {
    let sheet = components_sheet();
    let open = "details[open] > summary [data-icon=\"tree-collapsed\"]";
    let closed = "details:not([open]) > summary [data-icon=\"tree-expanded\"]";
    for hidden in [open, closed] {
        assert_eq!(value_of(&sheet, hidden, "display").as_deref(), Some("none"));
    }
}

#[test]
fn components_the_first_comment_lists_every_class_and_each_class_has_a_rule() {
    assert_classes_listed_and_used(COMPONENTS_CSS, &components_sheet(), &COMPONENT_CLASSES);
}
