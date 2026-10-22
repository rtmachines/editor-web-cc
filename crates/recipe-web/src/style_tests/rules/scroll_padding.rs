//! Tests of the scroll padding of `html` (QA15-1, ADR-0049 (m), `design-system.md` section 10.7).
//!
//! The padding keeps a focused element below the sticky toast. These tests size the toast from the
//! tokens of the rules `.toast`, `.toast .icon`, and `body`. They refuse only the properties that
//! `TOAST_FORBIDDEN` names. Browser row B28 checks the real height of the toast.

use super::super::css::Rule;
use super::super::{APP_CSS, app_sheet, components_sheet, root_tokens};
use super::{rules_of, value_of};

/// The media condition from which the toast of the longest message stands on one line.
pub(super) const WIDE_VIEW: &str = "(min-width: 48rem)";
/// The root font size of a browser in CSS px. A `rem` in a media condition uses it.
const ROOT_PX: f64 = 16.0;
/// The scroll padding below `WIDE_VIEW`: the toast of four lines and the gap below it.
const NARROW_PADDING: &str = "calc(var(--space-3) * 2 + var(--edge) * 2 + var(--text-base) * var(--leading) * 4 + var(--space-4))";
/// The scroll padding from `WIDE_VIEW`: one target and the gap below the toast.
const WIDE_PADDING: &str = "calc(var(--target) + var(--space-4))";
/// The lines of `toast-version-approved-retired` with three-digit numbers in `pl`, by view width.
///
/// QA15 measured 3 lines at 320 CSS px (row B12) and 2 lines at 481 CSS px (finding QA15-1), with
/// numbers of one digit. The other widths and the numbers of three digits are estimates. Browser
/// row B28 checks the widths 320, 481, 560, and 768 px with numbers of one digit. No test checks
/// the numbers of three digits. The text width is the view width minus the padding of `main`, the
/// padding and the border of the toast, the icon, and its gap.
const TOAST_LINES: [(f64, f64); 5] = [
    (320.0, 3.0),
    (481.0, 2.0),
    (560.0, 2.0),
    (767.0, 2.0),
    (768.0, 1.0),
];

/// Return the value of a token of `:root` in CSS px, or the number of a token without a unit.
/// A `rem` has the root font size.
fn token_px_or_number(name: &str) -> f64 {
    let tokens = root_tokens(&app_sheet());
    let text = tokens
        .iter()
        .find(|(token, _value)| token == name)
        .map(|(_token, value)| value.as_str())
        .unwrap();
    let (number, factor) = match text.strip_suffix("rem") {
        Some(rem) => (rem, ROOT_PX),
        None => (text.strip_suffix("px").unwrap_or(text), 1.0),
    };
    number.parse::<f64>().unwrap() * factor
}

/// Return the height in CSS px of a toast with `lines` lines of text. `tokens` names the vertical
/// padding, the border, the text size, the line height, and the icon size, in this order.
fn height_of(tokens: [&str; 5], lines: f64) -> f64 {
    let [padding, edge, text, leading, icon] = tokens.map(token_px_or_number);
    let content = (lines * text * leading).max(icon);
    2.0 * padding + 2.0 * edge + content
}

/// Return the token that the first word of `property` names with `var(...)` in the rule `selector`.
fn rule_token(sheet: &[Rule], selector: &str, property: &str) -> String {
    let value = value_of(sheet, selector, property).unwrap();
    let word = value.split_whitespace().next().unwrap();
    let name = word.strip_prefix("var(").and_then(|w| w.strip_suffix(')'));
    assert!(
        name.is_some(),
        "{selector} {property}: {value} has no token"
    );
    name.unwrap().to_owned()
}

/// Return the height in CSS px of the toast that its rules in the style sheets give.
///
/// It reads the padding and the border from `.toast` and the icon size from `.toast .icon`. It
/// reads the text size and the line height from `body`. A change of one of these tokens changes
/// the model.
fn toast_height(lines: f64) -> f64 {
    let (app, components) = (app_sheet(), components_sheet());
    let tokens = [
        rule_token(&components, ".toast", "padding"),
        rule_token(&components, ".toast", "border"),
        rule_token(&app, "body", "font-size"),
        rule_token(&app, "body", "line-height"),
        rule_token(&components, ".toast .icon", "height"),
    ];
    height_of(tokens.each_ref().map(String::as_str), lines)
}

/// Return the height of the wide rule: one target and the gap below the toast.
fn wide_padding_px() -> f64 {
    token_px_or_number("--target") + token_px_or_number("--space-4")
}

/// Return the height of the narrow rule: the toast of four lines from the tokens of
/// `NARROW_PADDING`, and the gap below it. The icon size is only a minimum of the content.
fn narrow_padding_px() -> f64 {
    let tokens = [
        "--space-3",
        "--edge",
        "--text-base",
        "--leading",
        "--icon-lg",
    ];
    height_of(tokens, 4.0) + token_px_or_number("--space-4")
}

#[test]
fn scroll_padding_below_48rem_is_the_height_of_a_toast_of_four_lines_and_a_gap() {
    assert_eq!(
        value_of(&app_sheet(), "html", "scroll-padding-top").as_deref(),
        Some(NARROW_PADDING)
    );
    // 2 * 12 + 2 * 1 + 4 * 17 * 1.5 + 16 = 144 CSS px.
    assert!(narrow_padding_px() > 143.9 && narrow_padding_px() < 144.1);
}

#[test]
fn scroll_padding_from_48rem_keeps_one_target_and_the_gap() {
    let sheet = app_sheet();
    let wide = rules_of(&sheet, "html", Some(WIDE_VIEW));
    assert_eq!(wide.len(), 1, "one html rule in the media block");
    assert_eq!(
        wide.first()
            .and_then(|rule| rule.value("scroll-padding-top")),
        Some(WIDE_PADDING)
    );
    assert!(wide_padding_px() >= toast_height(1.0));
}

/// Return the scroll padding in CSS px that the style sheet gives to `html` at a view width.
///
/// It reads the media block from `WIDE_VIEW` on and the top-level rule below it. It fails on a
/// value that the two constants do not name, so the test never reads a value it cannot size.
fn scroll_padding_at(width: f64) -> f64 {
    let sheet = app_sheet();
    let media = (width >= 48.0 * ROOT_PX).then_some(WIDE_VIEW);
    let value = rules_of(&sheet, "html", media)
        .iter()
        .rev()
        .find_map(|rule| rule.value("scroll-padding-top"))
        .or_else(|| {
            rules_of(&sheet, "html", None)
                .iter()
                .rev()
                .find_map(|rule| rule.value("scroll-padding-top"))
        })
        .unwrap();
    let size = match value {
        NARROW_PADDING => Some(narrow_padding_px()),
        WIDE_PADDING => Some(wide_padding_px()),
        _ => None,
    };
    assert!(
        size.is_some(),
        "scroll padding {value} at {width} px has no size"
    );
    size.unwrap()
}

#[test]
fn scroll_padding_covers_the_toast_of_each_view_width() {
    for (width, lines) in TOAST_LINES {
        let padding = scroll_padding_at(width);
        assert!(
            padding >= toast_height(lines),
            "{width} px: padding {padding}, toast {}",
            toast_height(lines)
        );
    }
}

/// The full value of `padding` of `.toast`. `toast_height` reads only its first word.
const TOAST_PADDING: &str = "var(--space-3) var(--space-4)";
/// The full value of `border` of `.toast`. `toast_height` reads only its first word.
const TOAST_BORDER: &str = "var(--edge) solid var(--color-topbar)";
/// The properties that no rule `.toast` may set, because each one makes the model of
/// `toast_height` false. Other properties can also change the height of the toast. Browser row B28
/// checks the real height.
const TOAST_FORBIDDEN: [&str; 11] = [
    "font-size",
    "line-height",
    "height",
    "min-height",
    "max-height",
    "padding-top",
    "padding-bottom",
    "padding-block",
    "border-width",
    "border-top",
    "border-bottom",
];

/// Assert that no rule of `.toast` in either sheet, at the top level or in a media block, sets a
/// property of `TOAST_FORBIDDEN`.
fn assert_toast_rule_has_no_size(sheets: [&[Rule]; 2]) {
    for rule in sheets.into_iter().flatten() {
        if rule.selectors.iter().any(|selector| selector == ".toast") {
            for property in TOAST_FORBIDDEN {
                assert_eq!(rule.value(property), None, "{property}");
            }
        }
    }
}

/// Assert that no rule of a selector that starts with `.toast` or `#toast` sets a text size.
///
/// A rule of a descendant such as `.toast span` would change the lines of text of the model.
fn assert_toast_selectors_have_no_text_size(sheets: [&[Rule]; 2]) {
    for rule in sheets.into_iter().flatten() {
        let on_toast = rule
            .selectors
            .iter()
            .any(|selector| selector.starts_with(".toast") || selector.starts_with("#toast"));
        if on_toast {
            for property in ["font-size", "line-height"] {
                assert_eq!(
                    rule.value(property),
                    None,
                    "{:?} {property}",
                    rule.selectors
                );
            }
        }
    }
}

#[test]
fn scroll_padding_toast_rule_has_the_tokens_and_no_size_of_the_model() {
    let (app, components) = (app_sheet(), components_sheet());
    // A fixed size on `.toast` would make the model of `toast_height` false.
    assert_toast_rule_has_no_size([&app, &components]);
    assert_toast_selectors_have_no_text_size([&app, &components]);
    // The model reads the first word of each shorthand, so the test compares the full value.
    assert_eq!(
        value_of(&components, ".toast", "padding").as_deref(),
        Some(TOAST_PADDING)
    );
    assert_eq!(
        value_of(&components, ".toast", "border").as_deref(),
        Some(TOAST_BORDER)
    );
    let expected = [
        (&components, ".toast .icon", "height", "--icon-lg"),
        (&app, "body", "font-size", "--text-base"),
        (&app, "body", "line-height", "--leading"),
    ];
    for (sheet, selector, property, token) in expected {
        assert_eq!(
            rule_token(sheet, selector, property),
            token,
            "{selector} {property}"
        );
    }
}

#[test]
fn scroll_padding_comment_names_the_design_section_and_the_wcag_rule() {
    let end = APP_CSS.find("\nhtml {").unwrap();
    let comment = APP_CSS.get(..end).unwrap();
    let start = comment.rfind("/*").unwrap();
    let comment = comment.get(start..).unwrap();
    assert!(comment.contains("design-system.md 10.7"), "{comment}");
    assert!(comment.contains("WCAG 2.4.11"), "{comment}");
}
