//! Rule tests of `app.css` (design D4, tasks 2.3 and 2.4, ADR-0037 r).
//!
//! `APP_EXPECTED` pins the declarations of the base rules that the design names. The rules of the
//! components live in `component_rules`. The other tests check the format rules of
//! `design-system.md` sections 5, 7, 11, and 12 on every rule of both style sheets.
//! The module `scroll_padding` holds the tests of the scroll padding of `html` (QA15-1, ADR-0049 (m)).

mod scroll_padding;

use super::css::{Rule, is_named_color, parse_sheet, parse_tables};
use super::{APP_CSS, COMPONENTS_CSS, DESIGN, app_sheet, components_sheet, sheet};
use scroll_padding::WIDE_VIEW;

/// The system colors of forced-colors mode (`design-system.md` section 12).
const SYSTEM_COLORS: [&str; 8] = [
    "Canvas",
    "CanvasText",
    "LinkText",
    "ButtonText",
    "ButtonBorder",
    "Highlight",
    "HighlightText",
    "GrayText",
];

/// The words that a color property may hold outside `var(...)`, besides numbers.
const NEUTRAL_WORDS: [&str; 10] = [
    "transparent",
    "currentColor",
    "none",
    "solid",
    "inset",
    "auto",
    "inherit",
    "calc",
    "*",
    "+",
];

/// The properties that take a color.
const COLOR_PROPERTIES: [&str; 18] = [
    "color",
    "background",
    "background-color",
    "border",
    "border-color",
    "border-top",
    "border-right",
    "border-bottom",
    "border-left",
    "border-bottom-color",
    "border-left-color",
    "outline",
    "outline-color",
    "box-shadow",
    "fill",
    "stroke",
    "text-decoration-color",
    "accent-color",
];

/// The class names of the base rules that the templates use. `app.css` lists them in its first
/// comment and holds at least one rule for each. `components.css` lists its own classes.
const APP_CLASSES: [&str; 19] = [
    "skip-link",
    "topbar",
    "app-name",
    "nav-list",
    "nav-link",
    "lang-switch",
    "user-login",
    "actions",
    "btn",
    "btn-primary",
    "btn-secondary",
    "btn-danger",
    "target-link",
    "data",
    "field",
    "field-hint",
    "field-required",
    "icon",
    "htmx-indicator",
];

/// The expected declarations of `app.css`: selector, property, value. A rule at the top level.
const APP_EXPECTED: &[(&str, &str, &str)] = &[
    // The page (task 2.3).
    ("body", "font-family", "var(--font-sans)"),
    ("body", "font-size", "var(--text-base)"),
    ("body", "line-height", "var(--leading)"),
    ("body", "background", "var(--color-canvas)"),
    ("body", "color", "var(--color-text)"),
    ("body", "font-variant-numeric", "tabular-nums"),
    ("main", "max-width", "var(--width-page)"),
    ("main", "padding", "var(--space-6)"),
    ("h1", "font-size", "var(--text-xl)"),
    ("h2", "font-size", "var(--text-lg)"),
    ("h1", "font-weight", "700"),
    ("a", "text-decoration", "underline"),
    (".skip-link", "position", "absolute"),
    (".skip-link:focus", "transform", "none"),
    // The top bar and the navigation.
    (".topbar", "background", "var(--color-topbar)"),
    (".topbar", "color", "var(--color-on-action)"),
    (".topbar", "min-height", "var(--topbar-height)"),
    (".nav-list", "gap", "var(--target-gap)"),
    (".lang-switch", "gap", "var(--target-gap)"),
    (".actions", "gap", "var(--target-gap)"),
    (".nav-link", "text-decoration", "none"),
    (".btn", "text-decoration", "none"),
    (".nav-link:hover", "background", "var(--color-topbar-hover)"),
    (".user-login", "overflow-wrap", "anywhere"),
    (".data", "font-family", "var(--font-mono)"),
    (".topbar [aria-current]", "font-weight", "700"),
    (
        ".topbar [aria-current]",
        "border-bottom",
        "var(--edge-current) solid var(--color-on-action)",
    ),
    // Focus and data.
    (
        ":focus-visible",
        "outline",
        "var(--focus-width) solid transparent",
    ),
    (".data", "font-family", "var(--font-mono)"),
    (".data", "font-size", "var(--text-base)"),
    (".data", "overflow-wrap", "anywhere"),
    // The form field and the buttons (section 10.5).
    (".field-hint", "color", "var(--color-text-muted)"),
    (".field-hint", "font-size", "var(--text-sm)"),
    ("input", "background", "var(--color-surface)"),
    (
        "input",
        "border",
        "var(--edge-control) solid var(--color-border)",
    ),
    (
        "select",
        "border",
        "var(--edge-control) solid var(--color-border)",
    ),
    (
        "button",
        "border",
        "var(--edge-control) solid var(--color-border)",
    ),
    ("input", "font-size", "var(--text-base)"),
    ("input:disabled", "background", "var(--color-sunken)"),
    (".btn-primary", "background", "var(--color-action)"),
    (".btn-primary", "color", "var(--color-on-action)"),
    (
        ".btn-primary:hover",
        "background",
        "var(--color-action-hover)",
    ),
    (".btn-secondary", "background", "var(--color-surface)"),
    (".btn-secondary", "color", "var(--color-action)"),
    (".btn-danger", "color", "var(--color-error-fg)"),
    (".btn-danger", "border-color", "var(--color-error-edge)"),
    (".btn:disabled", "background", "var(--color-sunken)"),
    // The indicator of a request (`api.md` section 9).
    (".htmx-indicator", "opacity", "0"),
    (".htmx-request .htmx-indicator", "opacity", "1"),
];

/// Return the rules at the top level or in the media block, whose selector list holds `selector`.
pub(super) fn rules_of<'a>(
    sheet: &'a [Rule],
    selector: &str,
    media: Option<&str>,
) -> Vec<&'a Rule> {
    sheet
        .iter()
        .filter(|rule| rule.media.as_deref() == media)
        .filter(|rule| rule.selectors.iter().any(|s| s == selector))
        .collect()
}

/// Return the value of a property in the last matching rule.
pub(super) fn value_of(sheet: &[Rule], selector: &str, property: &str) -> Option<String> {
    rules_of(sheet, selector, None)
        .iter()
        .rev()
        .find_map(|rule| rule.value(property))
        .map(str::to_owned)
}

/// Remove every `var(...)` group, then split the rest into words.
fn words_outside_var(value: &str) -> Vec<String> {
    let mut out = String::new();
    let mut rest = value;
    while let Some((before, after)) = rest.split_once("var(") {
        out.push_str(before);
        out.push(' ');
        rest = after.split_once(')').map_or("", |(_inside, tail)| tail);
    }
    out.push_str(rest);
    out.replace(['(', ')', ','], " ")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Return the declarations of every rule except `:root` and `@font-face`.
fn rule_decls(sheet: &[Rule]) -> Vec<(&Rule, &str, &str)> {
    sheet
        .iter()
        .filter(|rule| {
            !rule
                .selectors
                .iter()
                .any(|s| s == ":root" || s == "@font-face")
        })
        .flat_map(|rule| {
            rule.decls
                .iter()
                .map(move |(name, value)| (rule, name.as_str(), value.as_str()))
        })
        .collect()
}

/// Return the class names in a text: every identifier after a dot.
pub(super) fn classes_in(text: &str) -> Vec<String> {
    let mut classes = Vec::new();
    let mut parts = text.split('.');
    parts.next();
    for part in parts {
        let name: String = part
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !name.is_empty() {
            classes.push(name);
        }
    }
    classes
}

/// Return true when a word is a number or a length written with a unit.
fn is_literal_length(word: &str) -> bool {
    let digits = word.trim_start_matches('-');
    let number: String = digits
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let unit = digits.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
    !number.is_empty()
        && number.chars().any(|c| c != '0' && c != '.')
        && ["px", "rem", "em", "ch", "pt", "vh", "vw"].contains(&unit)
}

/// Return the problems of the words of one declaration outside `var(...)`.
///
/// A named color is a problem in every property. A color property may hold only numbers, the
/// neutral words, and the system colors.
fn color_word_problems(property: &str, value: &str) -> Vec<String> {
    let is_color_property = COLOR_PROPERTIES.contains(&property);
    let mut problems = Vec::new();
    for word in words_outside_var(value) {
        let system = SYSTEM_COLORS.contains(&word.as_str());
        let numeric = word.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.');
        let allowed = numeric || system || NEUTRAL_WORDS.contains(&word.as_str());
        if (is_named_color(&word) && !system) || (is_color_property && !allowed) {
            problems.push(format!("{property}: word {word}"));
        }
    }
    problems
}

/// Return the token name at the start of each `var(` group in a value.
fn var_names(value: &str) -> Vec<String> {
    value
        .split("var(")
        .skip(1)
        .map(|after| {
            after
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect()
        })
        .collect()
}

/// Return the tokens that a `var(...)` names and that no `:root` rule defines.
fn undefined_tokens(sheet: &[Rule]) -> Vec<String> {
    let defined: Vec<&str> = sheet
        .iter()
        .filter(|rule| rule.selectors.iter().any(|s| s == ":root"))
        .flat_map(|rule| rule.decls.iter().map(|(name, _value)| name.as_str()))
        .collect();
    sheet
        .iter()
        .flat_map(|rule| rule.decls.iter())
        .flat_map(|(_name, value)| var_names(value))
        .filter(|name| !defined.contains(&name.as_str()))
        .collect()
}

#[test]
fn rules_the_sheets_are_flat_lists_with_at_most_one_media_level() {
    let sheet = sheet();
    assert!(sheet.len() > 60, "found {} rules", sheet.len());
    assert!(app_sheet().len() > 30, "app.css holds the base rules");
    assert!(components_sheet().len() > 30, "components.css holds rules");
    assert!(!APP_CSS.contains("@import"));
    assert!(!COMPONENTS_CSS.contains("@import"));
    for rule in &sheet {
        if let Some(media) = &rule.media {
            assert!(
                media == "(forced-colors: active)" || is_scroll_padding_rule(rule),
                "{media} {:?}",
                rule.selectors
            );
        }
    }
    // `WIDE_VIEW` belongs to `app.css`. No rule of `components.css` may use it.
    for rule in components_sheet() {
        assert_ne!(
            rule.media.as_deref(),
            Some(WIDE_VIEW),
            "{:?}",
            rule.selectors
        );
    }
}

/// Return whether `rule` is the one rule of `WIDE_VIEW`: the scroll padding of `html` (QA15-1).
fn is_scroll_padding_rule(rule: &Rule) -> bool {
    rule.media.as_deref() == Some(WIDE_VIEW)
        && rule.selectors == ["html"]
        && rule.decls.len() == 1
        && rule.value("scroll-padding-top").is_some()
}

#[test]
fn rules_each_expected_declaration_has_the_value_of_the_design() {
    let sheet = app_sheet();
    for (selector, property, expected) in APP_EXPECTED {
        let actual = value_of(&sheet, selector, property);
        assert_eq!(actual.as_deref(), Some(*expected), "{selector} {property}");
    }
}

#[test]
fn rules_outside_root_use_no_color_value_except_the_permitted_ones() {
    for (rule, property, value) in rule_decls(&sheet()) {
        let place = format!("{:?} {property}: {value}", rule.selectors);
        assert!(!value.contains('#'), "{place}");
        let functions = [
            "rgb(",
            "rgba(",
            "hsl(",
            "hsla(",
            "hwb(",
            "lab(",
            "lch(",
            "oklab(",
            "oklch(",
            "color(",
            "color-mix(",
        ];
        for function in functions {
            assert!(!value.contains(function), "{place}");
        }
        let problems = color_word_problems(property, value);
        assert!(problems.is_empty(), "{place}: {problems:?}");
    }
}

#[test]
fn color_words_a_named_color_is_found_in_every_property() {
    for (property, value) in [
        ("color", "red"),
        ("display", "Red"),
        ("caret-color", "blue"),
        ("border", "var(--edge) solid RebeccaPurple"),
        ("text-shadow", "0 0 var(--space-1) black"),
        ("scrollbar-color", "gray auto"),
        ("margin", "teal"),
    ] {
        assert_eq!(
            color_word_problems(property, value).len(),
            1,
            "{property}: {value}"
        );
    }
}

#[test]
fn color_words_a_color_property_allows_only_the_neutral_words_and_system_colors() {
    assert_eq!(color_word_problems("color", "banana").len(), 1);
    assert_eq!(
        color_word_problems("outline", "2 dashed Highlight").len(),
        1
    );
    assert!(color_word_problems("border", "var(--edge) solid transparent").is_empty());
    assert!(color_word_problems("outline-color", "Highlight").is_empty());
    assert!(color_word_problems("color", "CanvasText").is_empty());
    assert!(color_word_problems("box-shadow", "none").is_empty());
}

#[test]
fn color_words_other_properties_may_hold_layout_words() {
    assert!(color_word_problems("display", "inline-flex").is_empty());
    assert!(color_word_problems("margin", "0 auto").is_empty());
    assert!(color_word_problems("overflow-wrap", "anywhere").is_empty());
    assert!(color_word_problems("font-variant-numeric", "tabular-nums").is_empty());
}

#[test]
fn tokens_each_var_names_a_token_of_root() {
    let ok = parse_sheet(":root { --a: 1; --b: var(--a) } p { x: var(--a); y: var(--b, 2) }");
    assert_eq!(undefined_tokens(&ok.unwrap()), Vec::<String>::new());
}

#[test]
fn tokens_an_unknown_or_misspelled_token_is_found() {
    let sheet = parse_sheet(
        ":root { --a: 1; --b: var(--missing) } p { x: calc(var(--a) + var(--aa)); y: var( --a ) }",
    )
    .unwrap();
    assert_eq!(undefined_tokens(&sheet), ["--missing", "--aa"]);
}

#[test]
fn tokens_a_sheet_without_root_has_no_token() {
    let sheet = parse_sheet("p { x: var(--a) }").unwrap();
    assert_eq!(undefined_tokens(&sheet), ["--a"]);
}

#[test]
fn tokens_every_var_of_the_sheet_names_a_token_of_root() {
    assert_eq!(undefined_tokens(&sheet()), Vec::<String>::new());
}

#[test]
fn rules_outside_root_write_no_literal_length_and_name_fonts_and_sizes_by_token() {
    for (rule, property, value) in rule_decls(&sheet()) {
        let place = format!("{:?} {property}: {value}", rule.selectors);
        for word in words_outside_var(value) {
            assert!(!is_literal_length(&word), "{place}: literal {word}");
        }
        if property == "font-family" {
            assert!(value.starts_with("var(--font-"), "{place}");
        }
        if property == "font-size" {
            assert!(value.starts_with("var(--text-"), "{place}");
        }
    }
}

#[test]
fn rules_edge_badge_is_only_for_the_badge() {
    for rule in sheet() {
        let on_badge = rule.selectors.iter().all(|s| s.starts_with(".badge"));
        for (name, value) in &rule.decls {
            if rule.selectors.iter().any(|s| s == ":root") {
                continue;
            }
            assert!(
                on_badge || !value.contains("--edge-badge"),
                "{:?} {name}",
                rule.selectors
            );
        }
    }
}

#[test]
fn rules_nothing_moves() {
    for rule in sheet() {
        for (name, _value) in &rule.decls {
            assert!(!name.starts_with("animation"), "{:?}", rule.selectors);
            assert!(!name.starts_with("transition"), "{:?}", rule.selectors);
        }
    }
    assert!(!APP_CSS.contains("@keyframes"));
    assert!(!COMPONENTS_CSS.contains("@keyframes"));
}

#[test]
fn rules_only_nav_link_and_btn_remove_the_underline() {
    let sheet = sheet();
    for rule in &sheet {
        if rule.value("text-decoration") == Some("none") {
            for selector in &rule.selectors {
                assert!(
                    selector == ".nav-link" || selector == ".btn",
                    "text-decoration: none on {selector}"
                );
            }
        }
    }
}

#[test]
fn rules_each_target_has_the_minimum_height_and_the_flex_display() {
    let sheet = sheet();
    let all = [
        "button",
        "input",
        "select",
        ".btn",
        ".nav-link",
        ".tree-node",
    ];
    let links = [".target-link", ".table-wrap a"];
    for selector in all.iter().chain(links.iter()) {
        let height = value_of(&sheet, selector, "min-height");
        assert_eq!(height.as_deref(), Some("var(--target)"), "{selector}");
    }
    for selector in ["button", ".btn", ".nav-link", ".tree-node"]
        .iter()
        .chain(links.iter())
    {
        let width = value_of(&sheet, selector, "min-width");
        assert_eq!(width.as_deref(), Some("var(--target)"), "{selector}");
        let display = value_of(&sheet, selector, "display").unwrap_or_default();
        assert!(display == "flex" || display == "inline-flex", "{selector}");
    }
    for selector in links {
        let display = value_of(&sheet, selector, "display");
        assert_eq!(display.as_deref(), Some("inline-flex"), "{selector}");
    }
}

#[test]
fn rules_focus_visible_draws_two_rings_and_a_transparent_outline() {
    let sheet = sheet();
    let shadow = value_of(&sheet, ":focus-visible", "box-shadow").unwrap();
    assert!(shadow.contains("var(--color-focus)"), "{shadow}");
    assert!(shadow.contains("var(--color-focus-ring)"), "{shadow}");
    assert!(shadow.contains("var(--focus-width)"), "{shadow}");
}

#[test]
fn rules_flex_items_with_text_shrink() {
    let sheet = sheet();
    for selector in [".topbar > *", ".alert > *", ".tree-node > *"] {
        let min_width = value_of(&sheet, selector, "min-width");
        assert_eq!(min_width.as_deref(), Some("0"), "{selector}");
    }
}

#[test]
fn rules_forced_colors_keeps_the_focus_and_the_current_link_visible() {
    let sheet = app_sheet();
    let focus = rules_of(&sheet, ":focus-visible", Some("(forced-colors: active)"));
    assert!(
        focus
            .iter()
            .any(|r| r.value("outline-color") == Some("Highlight"))
    );
    let current = rules_of(
        &sheet,
        ".topbar [aria-current]",
        Some("(forced-colors: active)"),
    );
    assert!(
        current
            .iter()
            .any(|r| r.value("border-bottom-color") == Some("Highlight"))
    );
    let framed = rules_of(&sheet, ".topbar", Some("(forced-colors: active)"));
    assert!(
        framed
            .iter()
            .any(|r| r.value("border-color") == Some("ButtonBorder"))
    );
}

#[test]
fn rules_htmx_indicator_changes_only_opacity() {
    let sheet = sheet();
    let mut found = false;
    for rule in &sheet {
        if rule.selectors.iter().any(|s| s.contains(".htmx-indicator")) {
            found = true;
            assert!(rule.decls.iter().all(|(name, _v)| name == "opacity"));
        }
    }
    assert!(found);
}

#[test]
fn rules_font_faces_match_the_font_table_of_the_design() {
    let sheet = sheet();
    let faces: Vec<&Rule> = sheet
        .iter()
        .filter(|r| r.selectors == ["@font-face"])
        .collect();
    let files = parse_tables(DESIGN, &["File", "Face", "Subset", "Weight"]).unwrap();
    assert_eq!(files.len(), 8);
    assert_eq!(faces.len(), 8);
    for row in files {
        let url = format!("url(/static/fonts/{}) format(\"woff2\")", row[0]);
        let face = faces.iter().find(|f| f.value("src") == Some(url.as_str()));
        assert!(face.is_some(), "no @font-face for {}", row[0]);
        let face = face.unwrap();
        let family = format!("\"Atkinson Hyperlegible {}\"", row[1]);
        assert_eq!(
            face.value("font-family"),
            Some(family.as_str()),
            "{}",
            row[0]
        );
        assert_eq!(
            face.value("font-weight"),
            Some(row[3].as_str()),
            "{}",
            row[0]
        );
        assert_eq!(face.value("font-style"), Some("normal"), "{}", row[0]);
        assert_eq!(face.value("font-display"), Some("swap"), "{}", row[0]);
        let range = if row[2] == "latin" { LATIN } else { LATIN_EXT };
        assert_eq!(face.value("unicode-range"), Some(range), "{}", row[0]);
    }
}

/// The `unicode-range` of the subset `latin`, as the package declares it.
const LATIN: &str = "U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD";
/// The `unicode-range` of the subset `latin-ext`, as the package declares it.
const LATIN_EXT: &str = "U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,U+A720-A7FF";

/// Assert that the first comment of `css` lists every class of `classes` and that a rule of
/// `sheet` names each one.
pub(super) fn assert_classes_listed_and_used(css: &str, sheet: &[Rule], classes: &[&str]) {
    let comment = css
        .trim_start()
        .strip_prefix("/*")
        .and_then(|rest| rest.split_once("*/"))
        .map(|(inside, _tail)| inside)
        .unwrap();
    let listed = classes_in(comment);
    let used: Vec<String> = sheet
        .iter()
        .flat_map(|rule| rule.selectors.iter().flat_map(|s| classes_in(s)))
        .collect();
    for class in classes {
        assert!(listed.iter().any(|c| c == class), "comment lacks .{class}");
        assert!(used.iter().any(|c| c == class), "no rule for .{class}");
    }
}

#[test]
fn rules_the_first_comment_lists_every_class_and_each_class_has_a_rule() {
    assert_classes_listed_and_used(APP_CSS, &app_sheet(), &APP_CLASSES);
}
