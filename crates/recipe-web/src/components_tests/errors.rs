//! Tests of the error line macro, the field with error lines, and the summary (task 7.2).
//!
//! The rules come from `design-system.md` section 10.5 and ADR-0041 (d) and (e).

use askama::Template;

use super::icon;
use crate::error::{LineView, ListView, NO_LINES};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;

#[derive(Template)]
#[template(path = "test/error-line.html")]
struct ErrorLinePage {
    id: String,
    lines: Vec<LineView>,
}

#[derive(Template)]
#[template(path = "test/checked-field.html")]
struct CheckedFieldPage<'a> {
    t: Texts<'a>,
    hint: String,
    value: String,
    lines: Vec<LineView>,
}

#[derive(Template)]
#[template(path = "test/summary.html")]
struct SummaryPage {
    code: &'static str,
    list: ListView,
    link: bool,
    reference: String,
}

fn line(code: &'static str, message: &str, target: &str) -> LineView {
    LineView {
        code,
        subject: String::new(),
        message: message.to_owned(),
        target: target.to_owned(),
    }
}

/// The markup of one error line inside its block.
fn line_markup(code: &str, message: &str) -> String {
    format!(
        "<p>{}<span class=\"field-error-code\">{code}</span> <span>{message}</span></p>",
        icon("alert-error")
    )
}

#[test]
fn errors_the_error_line_shows_the_icon_the_code_and_the_sentence_under_its_id() {
    let html = ErrorLinePage {
        id: "tag".to_owned(),
        lines: vec![line("E-WEB-008", "Field Tag is not valid.", "tag")],
    }
    .render()
    .unwrap();
    let expected = format!(
        "<div class=\"field-error\" id=\"tag-error\">{}</div>",
        line_markup("E-WEB-008", "Field Tag is not valid.")
    );
    assert_eq!(html.trim(), expected);
}

#[test]
fn errors_two_lines_of_one_input_stand_in_one_block() {
    let html = ErrorLinePage {
        id: "value-3".to_owned(),
        lines: vec![
            line("E-RCP-004", "First.", "value-3"),
            line("E-RCP-018", "Second.", "value-3"),
        ],
    }
    .render()
    .unwrap();
    assert_eq!(html.matches("id=\"value-3-error\"").count(), 1, "{html}");
    assert!(html.contains(&line_markup("E-RCP-004", "First.")), "{html}");
    assert!(
        html.contains(&line_markup("E-RCP-018", "Second.")),
        "{html}"
    );
}

#[test]
fn errors_no_line_gives_no_block() {
    let html = ErrorLinePage {
        id: "tag".to_owned(),
        lines: Vec::new(),
    }
    .render()
    .unwrap();
    assert_eq!(html.trim(), "");
}

#[test]
fn errors_the_error_line_escapes_the_message() {
    let html = ErrorLinePage {
        id: "tag".to_owned(),
        lines: vec![line("E-WEB-008", "<script>alert(1)</script>", "tag")],
    }
    .render()
    .unwrap();
    assert!(!html.contains("<script"), "{html}");
    assert!(html.contains("&#60;script&#62;"), "{html}");
}

fn checked_field(hint: &str, lines: Vec<LineView>) -> String {
    let bundles = Bundles::load().unwrap();
    CheckedFieldPage {
        t: Texts::new(&bundles, Lang::En),
        hint: hint.to_owned(),
        value: "SPEED".to_owned(),
        lines,
    }
    .render()
    .unwrap()
}

#[test]
fn errors_an_input_with_a_line_is_invalid_and_names_its_hint_and_its_line() {
    let html = checked_field("Hint.", vec![line("E-WEB-008", "Bad tag.", "tag")]);
    assert!(
        html.contains(" aria-invalid=\"true\" aria-describedby=\"tag-hint tag-error\">"),
        "{html}"
    );
    let block = format!(
        "<div class=\"field-error\" id=\"tag-error\">{}</div></div>",
        line_markup("E-WEB-008", "Bad tag.")
    );
    assert!(html.contains(&block), "{html}");
    assert!(html.contains("value=\"SPEED\""), "{html}");
}

#[test]
fn errors_an_input_with_a_line_and_no_hint_names_only_its_line() {
    let html = checked_field("", vec![line("E-WEB-008", "Bad tag.", "tag")]);
    assert!(
        html.contains(" aria-invalid=\"true\" aria-describedby=\"tag-error\">"),
        "{html}"
    );
}

#[test]
fn errors_an_input_without_a_line_keeps_the_markup_of_the_plain_field() {
    let html = checked_field("Hint.", Vec::new());
    assert!(!html.contains("aria-invalid"), "{html}");
    assert!(!html.contains("field-error"), "{html}");
    assert!(html.contains(" aria-describedby=\"tag-hint\">"), "{html}");
}

#[test]
fn errors_the_constant_without_lines_is_empty() {
    assert!(NO_LINES.is_empty());
}

fn summary(link: bool, message: &str, more: &str) -> String {
    SummaryPage {
        code: "E-RCP-004",
        list: ListView {
            count: "Errors: 2.".to_owned(),
            lines: vec![
                line("E-RCP-004", message, "value-3"),
                line("E-RCP-020", "Version line.", ""),
            ],
            more: more.to_owned(),
        },
        link,
        reference: "Reference: 1.".to_owned(),
    }
    .render()
    .unwrap()
}

#[test]
fn errors_the_summary_has_the_alert_head_the_lines_and_the_reference() {
    let html = summary(true, "Leaf line.", "");
    let expected = format!(
        "<div class=\"alert alert-error\"><div class=\"alert-head\" role=\"alert\">{}<span class=\"alert-code\">E-RCP-004</span><span>Errors: 2.</span></div><ul class=\"field-lines\"><li><a class=\"target-link\" href=\"#value-3\"><span class=\"alert-code\">E-RCP-004</span> <span>Leaf line.</span></a></li><li><span class=\"alert-code\">E-RCP-020</span> <span>Version line.</span></li></ul><span class=\"alert-ref\">Reference: 1.</span></div>",
        icon("alert-error")
    );
    assert_eq!(html.trim(), expected);
}

#[test]
fn errors_the_summary_without_links_and_with_more_lines() {
    let html = summary(false, "Leaf line.", "Other errors: 3.");
    assert!(!html.contains("<a "), "{html}");
    assert!(html.contains("<li>Other errors: 3.</li></ul>"), "{html}");
}

#[test]
fn errors_the_summary_shows_the_subject_of_a_line_as_data() {
    let mut list = ListView {
        count: "Errors: 1.".to_owned(),
        lines: vec![line("E-PLT-020", "Unit does not exist.", "")],
        more: String::new(),
    };
    if let Some(first) = list.lines.first_mut() {
        first.subject = "HOLD".to_owned();
    }
    let html = SummaryPage {
        code: "E-RCP-020",
        list,
        link: true,
        reference: "R.".to_owned(),
    }
    .render()
    .unwrap();
    assert!(
        html.contains("<li><span class=\"alert-code\">E-PLT-020</span> <span class=\"data\">HOLD</span> <span>Unit does not exist.</span></li>"),
        "{html}"
    );
}

#[test]
fn errors_the_summary_escapes_every_value() {
    let html = summary(true, "<img src=x onerror=alert(1)>", "<b>more</b>");
    assert!(!html.contains("<img"), "{html}");
    assert!(!html.contains("<b>"), "{html}");
    assert!(html.contains("&#60;img"), "{html}");
}

/// One option of the list of choices.
struct Choice {
    value: &'static str,
    text: String,
}

#[derive(Template)]
#[template(path = "test/checked-select.html")]
struct CheckedSelectPage<'a> {
    t: Texts<'a>,
    id: &'static str,
    name: &'static str,
    hint: String,
    options: Vec<Choice>,
    selected: String,
    lines: Vec<LineView>,
}

#[derive(Template)]
#[template(path = "test/checked-check.html")]
struct CheckedCheckPage<'a> {
    t: Texts<'a>,
    lines: Vec<LineView>,
}

#[derive(Template)]
#[template(path = "test/checked-listed.html")]
struct CheckedListedPage<'a> {
    t: Texts<'a>,
    value: String,
    lines: Vec<LineView>,
}

fn checked_select(hint: &str, lines: Vec<LineView>) -> String {
    let bundles = Bundles::load().unwrap();
    CheckedSelectPage {
        t: Texts::new(&bundles, Lang::En),
        id: "recipe_machine",
        name: "machine",
        hint: hint.to_owned(),
        options: vec![Choice {
            value: "M7",
            text: "M7".to_owned(),
        }],
        selected: "M7".to_owned(),
        lines,
    }
    .render()
    .unwrap()
}

#[test]
fn errors_a_named_select_takes_its_id_and_its_name_apart() {
    let html = checked_select("", Vec::new());
    assert!(
        html.contains("<label for=\"recipe_machine\">Machine</label>"),
        "{html}"
    );
    assert!(
        html.contains("<select id=\"recipe_machine\" name=\"machine\">"),
        "{html}"
    );
    assert!(
        html.contains("<option value=\"M7\" selected>M7</option>"),
        "{html}"
    );
    assert!(!html.contains("aria-"), "{html}");
}

#[test]
fn errors_a_named_select_with_a_line_is_invalid_and_shows_the_line() {
    let html = checked_select(
        "Hint.",
        vec![line("E-PLT-019", "No machine.", "recipe_machine")],
    );
    assert!(
        html.contains(
            " aria-invalid=\"true\" aria-describedby=\"recipe_machine-hint recipe_machine-error\">"
        ),
        "{html}"
    );
    let block = format!(
        "</select><div class=\"field-error\" id=\"recipe_machine-error\">{}</div></div>",
        line_markup("E-PLT-019", "No machine.")
    );
    assert!(html.contains(&block), "{html}");
}

fn checked_check(lines: Vec<LineView>) -> String {
    let bundles = Bundles::load().unwrap();
    CheckedCheckPage {
        t: Texts::new(&bundles, Lang::En),
        lines,
    }
    .render()
    .unwrap()
}

#[test]
fn errors_a_checkbox_with_a_line_is_invalid_and_shows_the_line_after_its_label() {
    let html = checked_check(vec![line("E-WEB-008", "Bad flag.", "show_deleted")]);
    assert!(
        html.contains(
            " value=\"1\" aria-invalid=\"true\" aria-describedby=\"show_deleted-error\">"
        ),
        "{html}"
    );
    let block = format!(
        "</label><div class=\"field-error\" id=\"show_deleted-error\">{}</div>",
        line_markup("E-WEB-008", "Bad flag.")
    );
    assert!(html.contains(&block), "{html}");
}

#[test]
fn errors_a_checkbox_without_a_line_keeps_its_markup() {
    let html = checked_check(Vec::new());
    assert_eq!(
        html.trim(),
        "<label class=\"check\"><input type=\"checkbox\" id=\"show_deleted\" name=\"show_deleted\" value=\"1\"><span>Show deleted recipes</span></label>"
    );
}

fn checked_listed(lines: Vec<LineView>) -> String {
    let bundles = Bundles::load().unwrap();
    CheckedListedPage {
        t: Texts::new(&bundles, Lang::En),
        value: "INT".to_owned(),
        lines,
    }
    .render()
    .unwrap()
}

#[test]
fn errors_a_listed_input_with_a_line_names_its_hint_and_its_line() {
    let html = checked_listed(vec![line("E-PLT-013", "Bad type.", "type")]);
    assert!(
        html.contains(
            " aria-describedby=\"type-hint type-error\" value=\"INT\" aria-invalid=\"true\">"
        ),
        "{html}"
    );
    let block = format!(
        "<div class=\"field-error\" id=\"type-error\">{}</div></div>",
        line_markup("E-PLT-013", "Bad type.")
    );
    assert!(html.contains(&block), "{html}");
}

#[test]
fn errors_a_listed_input_without_a_line_keeps_its_markup() {
    let html = checked_listed(Vec::new());
    assert!(
        html.contains(" aria-describedby=\"type-hint\" value=\"INT\"></div>"),
        "{html}"
    );
    assert!(!html.contains("aria-invalid"), "{html}");
}
