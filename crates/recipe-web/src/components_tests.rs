//! Tests of the askama components below `templates/components/` (design D6, `design-system.md` 8 to 10).
//!
//! A test template below `templates/test/` calls one macro and takes every text from a struct field,
//! so it holds no literal word. The expected markup is written here, in the form of the design.

mod structure;

use askama::Template;

use crate::error::FieldLine;
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::test_support::{icon_tag, text};

/// The icons of `design-system.md` section 8 with the shapes of the approved preview.
const ICONS: [(&str, &str); 11] = [
    (
        "status-draft",
        r#"<path d="M4 20h4L19 9l-4-4L4 16v4z"/><path d="M13.5 6.5l4 4"/>"#,
    ),
    (
        "status-approved",
        r#"<circle cx="12" cy="12" r="9"/><path d="M8 12.5l3 3 5-6"/>"#,
    ),
    (
        "status-retired",
        r#"<rect x="3" y="4" width="18" height="5" rx="1"/><path d="M5 9v10h14V9"/><path d="M10 13h4"/>"#,
    ),
    (
        "user-active",
        r#"<circle cx="12" cy="12" r="9"/><path d="M8 12.5l3 3 5-6"/>"#,
    ),
    (
        "user-inactive",
        r#"<circle cx="12" cy="12" r="9"/><path d="M5.6 5.6l12.8 12.8"/>"#,
    ),
    (
        "alert-error",
        r#"<path d="M8 3h8l5 5v8l-5 5H8l-5-5V8z"/><path d="M12 8v5"/><path d="M12 16.5v.01"/>"#,
    ),
    (
        "alert-warning",
        r#"<path d="M12 3l10 18H2z"/><path d="M12 10v5"/><path d="M12 18v.01"/>"#,
    ),
    (
        "alert-info",
        r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v6"/><path d="M12 7.5v.01"/>"#,
    ),
    ("alert-success", r#"<path d="M5 12.5l4.5 4.5L19 7"/>"#),
    ("tree-collapsed", r#"<path d="M9 6l6 6-6 6"/>"#),
    ("tree-expanded", r#"<path d="M6 9l6 6 6-6"/>"#),
];

/// Return the complete markup of the icon `name`.
fn icon(name: &str) -> String {
    let shapes = ICONS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, shapes)| *shapes)
        .unwrap();
    format!("{}{shapes}</svg>", icon_tag(name))
}

/// Assert that every `<svg` of `html` opens with the fixed tag of one icon.
pub(crate) fn assert_fixed_svg_tags(html: &str) {
    let fixed: Vec<String> = ICONS.iter().map(|(name, _)| icon_tag(name)).collect();
    for (at, _) in html.match_indices("<svg") {
        let rest = html.get(at..).unwrap();
        assert!(
            fixed.iter().any(|tag| rest.starts_with(tag.as_str())),
            "unknown svg tag: {}",
            rest.chars().take(160).collect::<String>()
        );
    }
}

#[derive(Template)]
#[template(path = "test/icons.html")]
struct IconsPage;

#[derive(Template)]
#[template(path = "test/badge.html")]
struct BadgePage {
    kind: &'static str,
    word: String,
    large: bool,
}

#[derive(Template)]
#[template(path = "test/user-badge.html")]
struct UserBadgePage {
    active: bool,
    word: String,
}

#[derive(Template)]
#[template(path = "test/alert.html")]
struct AlertPage {
    severity: &'static str,
    code: String,
    message: String,
    reference: String,
}

#[derive(Template)]
#[template(path = "test/alert-fields.html")]
struct AlertFieldsPage {
    severity: &'static str,
    code: String,
    message: String,
    lines: Vec<FieldLine>,
    link: bool,
    reference: String,
}

#[derive(Template)]
#[template(path = "test/empty.html")]
struct EmptyPage {
    text: String,
}

#[derive(Template)]
#[template(path = "test/toast.html")]
struct ToastPage {
    text: String,
}

#[test]
fn components_icons_every_icon_is_its_fixed_tag_and_shapes() {
    let html = IconsPage.render().unwrap();
    for (name, _) in ICONS {
        assert!(html.contains(&icon(name)), "icon {name}");
    }
    assert_eq!(html.matches("<svg").count(), 11);
    assert_eq!(html.matches("</svg>").count(), 11);
}

#[test]
fn components_icons_have_no_style_handler_text_or_title() {
    let html = IconsPage.render().unwrap();
    assert_fixed_svg_tags(&html);
    for forbidden in [" style=", " on", "<title", "<text", "<script"] {
        assert!(!html.contains(forbidden), "found {forbidden}");
    }
}

#[test]
fn components_status_badge_shows_icon_and_word_in_both_languages() {
    for lang in [Lang::Pl, Lang::En] {
        for kind in ["draft", "approved", "retired"] {
            let word = text(lang, &format!("status-{kind}"));
            let html = BadgePage {
                kind,
                word: word.clone(),
                large: false,
            }
            .render()
            .unwrap();
            let expected = format!(
                "<span class=\"badge badge-{kind}\">{}<span>{word}</span></span>",
                icon(&format!("status-{kind}"))
            );
            assert_eq!(html.trim(), expected, "{lang} {kind}");
        }
    }
}

#[test]
fn components_status_badge_words_are_the_bundle_words() {
    let pl: Vec<String> = ["draft", "approved", "retired"]
        .iter()
        .map(|kind| text(Lang::Pl, &format!("status-{kind}")))
        .collect();
    let en: Vec<String> = ["draft", "approved", "retired"]
        .iter()
        .map(|kind| text(Lang::En, &format!("status-{kind}")))
        .collect();
    assert_eq!(pl, ["Szkic", "Zatwierdzona", "Wycofana"]);
    assert_eq!(en, ["Draft", "Approved", "Retired"]);
}

#[test]
fn components_large_status_badge_has_the_large_class() {
    let html = BadgePage {
        kind: "approved",
        word: text(Lang::En, "status-approved"),
        large: true,
    }
    .render()
    .unwrap();
    let expected = format!(
        "<span class=\"badge badge-large badge-approved\">{}<span>Approved</span></span>",
        icon("status-approved")
    );
    assert_eq!(html.trim(), expected);
}

#[test]
fn components_user_badge_shows_icon_and_word_in_both_languages() {
    for lang in [Lang::Pl, Lang::En] {
        for (active, class, name, key) in [
            (true, "badge-active", "user-active", "user-status-active"),
            (
                false,
                "badge-inactive",
                "user-inactive",
                "user-status-inactive",
            ),
        ] {
            let word = text(lang, key);
            let html = UserBadgePage {
                active,
                word: word.clone(),
            }
            .render()
            .unwrap();
            let expected = format!(
                "<span class=\"badge {class}\">{}<span>{word}</span></span>",
                icon(name)
            );
            assert_eq!(html.trim(), expected, "{lang} {name}");
        }
    }
}

#[test]
fn components_badge_word_is_escaped() {
    let html = BadgePage {
        kind: "draft",
        word: "<script>x</script>".to_owned(),
        large: false,
    }
    .render()
    .unwrap();
    assert!(!html.contains("<script"));
    assert!(html.contains("&#60;script&#62;"));
}

#[test]
fn components_alert_is_one_element_with_icon_code_sentence_and_reference() {
    for (severity, name) in [
        ("error", "alert-error"),
        ("warning", "alert-warning"),
        ("info", "alert-info"),
        ("success", "alert-success"),
    ] {
        let html = AlertPage {
            severity,
            code: "E-WEB-002".to_owned(),
            message: text(Lang::En, "recipes-empty"),
            reference: text(Lang::En, "recipes-title"),
        }
        .render()
        .unwrap();
        let expected = format!(
            "<div class=\"alert alert-{severity}\" role=\"alert\">{}<span class=\"alert-code\">E-WEB-002</span><span>No recipe to show.</span><span class=\"alert-ref\">Recipes</span></div>",
            icon(name)
        );
        assert_eq!(html.trim(), expected, "{severity}");
        assert_eq!(html.matches("role=\"alert\"").count(), 1);
    }
}

/// Render the alert with one field line that has an input and one that has none.
fn render_field_lines(link: bool) -> String {
    let line = |field: &str, has_input: bool| FieldLine {
        field: field.to_owned(),
        label: field.to_owned(),
        has_input,
    };
    AlertFieldsPage {
        severity: "error",
        code: "E-WEB-008".to_owned(),
        message: "Message.".to_owned(),
        lines: vec![line("login", true), line("page", false)],
        link,
        reference: "Reference.".to_owned(),
    }
    .render()
    .unwrap()
}

#[test]
fn components_alert_field_line_link_is_a_target_link() {
    let html = render_field_lines(true);
    assert!(
        html.contains("<li><a class=\"target-link\" href=\"#login\">login</a></li>"),
        "{html}"
    );
    assert!(html.contains("<li>page</li>"), "{html}");
    assert_eq!(html.matches("<a ").count(), 1, "{html}");
}

#[test]
fn components_alert_field_lines_without_link_hold_no_anchor() {
    let html = render_field_lines(false);
    assert!(!html.contains("<a "), "{html}");
    assert!(html.contains("<li>login</li>"), "{html}");
}

#[test]
fn components_alert_escapes_every_value() {
    let html = AlertPage {
        severity: "error",
        code: "<b>".to_owned(),
        message: "<script>alert(1)</script>".to_owned(),
        reference: "a&b'c".to_owned(),
    }
    .render()
    .unwrap();
    assert!(!html.contains("<script"));
    assert!(!html.contains("<b>"));
    assert!(html.contains("&#60;script&#62;"));
    assert!(html.contains("a&#38;b&#39;c"));
}

#[test]
fn components_toast_has_icon_and_sentence_and_no_role() {
    for lang in [Lang::Pl, Lang::En] {
        let sentence = text(lang, "recipes-empty");
        let html = ToastPage {
            text: sentence.clone(),
        }
        .render()
        .unwrap();
        let expected = format!(
            "<div class=\"toast\">{}<span>{sentence}</span></div>",
            icon("alert-success")
        );
        assert_eq!(html.trim(), expected);
        assert!(!html.contains("role="));
        assert!(!html.contains("aria-live"));
    }
}

#[test]
fn components_empty_state_shows_its_sentence_in_both_languages() {
    for lang in [Lang::Pl, Lang::En] {
        let sentence = text(lang, "recipes-empty");
        let html = EmptyPage {
            text: sentence.clone(),
        }
        .render()
        .unwrap();
        assert_eq!(html.trim(), format!("<p class=\"empty\">{sentence}</p>"));
    }
}

#[test]
fn components_every_svg_of_every_component_is_a_fixed_tag() {
    let mut all = IconsPage.render().unwrap();
    all.push_str(
        &BadgePage {
            kind: "retired",
            word: "w".to_owned(),
            large: true,
        }
        .render()
        .unwrap(),
    );
    all.push_str(
        &AlertPage {
            severity: "warning",
            code: "c".to_owned(),
            message: "m".to_owned(),
            reference: "r".to_owned(),
        }
        .render()
        .unwrap(),
    );
    all.push_str(
        &ToastPage {
            text: "t".to_owned(),
        }
        .render()
        .unwrap(),
    );
    assert_fixed_svg_tags(&all);
}
