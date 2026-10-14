//! Tests of the code review fixes R9, R11, R16, and R17 (task 8.5, task 8.7).
//!
//! The page tests read the real markup from the router of the fixture. The component tests render
//! a test template below `templates/test/`.

use askama::Template;

use super::page_tests::{LANGS, administrator, get_page};
use super::test_helpers::assert_clean_html;
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::test_support::Fixture;

/// Return the text of `key` in `lang`.
fn text(lang: Lang, key: &str) -> String {
    let bundles = Bundles::load().unwrap();
    Texts::new(&bundles, lang).get(key)
}

/// Return the opening tag `<input id="id" ...>` of `body` without the closing `>`.
fn input_tag<'a>(body: &'a str, id: &str) -> &'a str {
    let start = body.find(&format!("<input id=\"{id}\"")).unwrap();
    body.get(start..).unwrap().split('>').next().unwrap()
}

/// Assert that the opening tag of the input `id` holds every attribute of `wanted`.
fn assert_input_has(body: &str, id: &str, wanted: &[&str]) {
    let tag = input_tag(body, id);
    for attribute in wanted {
        assert!(
            tag.contains(attribute),
            "{id}: missing {attribute} in {tag}"
        );
    }
}

/// Assert the markup of `design-system.md` section 10.5 around the input `id`.
fn assert_field(body: &str, id: &str, lang: Lang) {
    let label_start = format!("<div class=\"field\"><label for=\"{id}\">");
    assert!(
        body.contains(&label_start),
        "{id}: no field wrapper in {body}"
    );
    let word = format!(
        "<span class=\"field-required\">{}</span>",
        text(lang, "form-required")
    );
    assert!(body.contains(&word), "{id}: no required word");
    assert_input_has(body, id, &["aria-required=\"true\""]);
}

#[tokio::test]
async fn review_fix_login_page_uses_the_field_markup_with_data_inputs() {
    let fixture = Fixture::new();
    for (lang, code) in LANGS {
        let body = get_page(&fixture, None, code, "/login").await.body;
        assert_field(&body, "login", lang);
        assert_field(&body, "password", lang);
        assert_input_has(
            &body,
            "login",
            &[
                "class=\"data\"",
                "autocapitalize=\"none\"",
                "spellcheck=\"false\"",
            ],
        );
        assert!(!input_tag(&body, "password").contains("class=\"data\""));
        assert_clean_html(&body);
    }
}

#[tokio::test]
async fn review_fix_password_page_uses_the_field_markup_and_links_the_hint() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let body = get_page(&fixture, Some(&session), code, "/me/password")
            .await
            .body;
        assert_field(&body, "current_password", lang);
        assert_field(&body, "password", lang);
        let hint = format!(
            "<span class=\"field-hint\" id=\"password-hint\">{}</span>",
            text(lang, "password-hint")
        );
        assert!(body.contains(&hint), "{body}");
        assert_input_has(&body, "password", &["aria-describedby=\"password-hint\""]);
        assert!(!input_tag(&body, "current_password").contains("aria-describedby"));
        assert_clean_html(&body);
    }
}

#[tokio::test]
async fn review_fix_new_user_and_search_logins_are_data_inputs() {
    let (fixture, session) = administrator();
    for (_lang, code) in LANGS {
        let new = get_page(&fixture, Some(&session), code, "/users/new")
            .await
            .body;
        let search = get_page(&fixture, Some(&session), code, "/users")
            .await
            .body;
        for (body, id) in [(&new, "login"), (&search, "q")] {
            assert_input_has(
                body,
                id,
                &[
                    "class=\"data\"",
                    "autocapitalize=\"none\"",
                    "spellcheck=\"false\"",
                ],
            );
        }
    }
}

#[tokio::test]
async fn review_fix_profile_login_is_a_data_value() {
    let (fixture, session) = administrator();
    let body = get_page(&fixture, Some(&session), "en", "/me").await.body;
    assert!(body.contains("<dd class=\"data\">user1</dd>"), "{body}");
}

#[tokio::test]
async fn review_fix_single_action_forms_have_a_primary_button() {
    let (fixture, session) = administrator();
    for (_lang, code) in LANGS {
        for path in ["/me/password", "/users/new"] {
            let body = get_page(&fixture, Some(&session), code, path).await.body;
            assert!(
                body.contains("<button type=\"submit\" class=\"btn btn-primary\">"),
                "{path}: {body}"
            );
        }
        let login = get_page(&fixture, None, code, "/login").await.body;
        assert!(login.contains("<button type=\"submit\" class=\"btn btn-primary\">"));
    }
}

#[tokio::test]
async fn review_fix_user_list_has_no_empty_actions_paragraph() {
    let (fixture, session) = administrator();
    let body = get_page(&fixture, Some(&session), "en", "/users")
        .await
        .body;
    let squeezed: String = body.split_whitespace().collect();
    assert!(!squeezed.contains("<pclass=\"actions\"></p>"), "{body}");
}

#[tokio::test]
async fn review_fix_recipes_empty_state_has_no_panel_around_it() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let body = get_page(&fixture, Some(&session), code, "/recipes")
            .await
            .body;
        let empty = format!("<p class=\"empty\">{}</p>", text(lang, "recipes-empty"));
        assert!(body.contains(&empty), "{body}");
        assert!(!body.contains("class=\"panel\""), "{body}");
    }
}

/// Assert that `shape` reads `yyyy-mm-dd hh:mm`.
fn assert_date_shape(shape: &str) {
    assert_eq!(shape.len(), 16, "{shape}");
    for (index, c) in shape.char_indices() {
        let fits = match index {
            4 | 7 => c == '-',
            10 => c == ' ',
            13 => c == ':',
            _ => c.is_ascii_digit(),
        };
        assert!(fits, "{shape}");
    }
}

#[tokio::test]
async fn review_fix_user_page_shows_dates_to_the_minute_in_utc() {
    let (fixture, session) = administrator();
    let body = get_page(&fixture, Some(&session), "en", "/users/user1")
        .await
        .body;
    let marker = " UTC</dd>";
    let end = body.find(marker).unwrap();
    let before = body.get(..end).unwrap();
    let open = "<dd class=\"data\">";
    let start = before.rfind(open).unwrap();
    assert_date_shape(before.get(start + open.len()..).unwrap());
}

/// The test template `test/alert.html`.
#[derive(Template)]
#[template(path = "test/alert.html")]
struct AlertPage {
    severity: &'static str,
    code: String,
    message: String,
    reference: String,
}

/// The test template `test/badge.html`.
#[derive(Template)]
#[template(path = "test/badge.html")]
struct BadgePage {
    kind: &'static str,
    word: String,
    large: bool,
}

/// The test template `test/reserved.html`.
#[derive(Template)]
#[template(path = "test/reserved.html")]
struct ReservedPage {
    word: String,
    number: String,
}

fn alert(severity: &'static str) -> String {
    AlertPage {
        severity,
        code: "E-TEST-001".to_owned(),
        message: "message".to_owned(),
        reference: "reference".to_owned(),
    }
    .render()
    .unwrap()
}

fn badge(kind: &'static str) -> String {
    BadgePage {
        kind,
        word: "word".to_owned(),
        large: false,
    }
    .render()
    .unwrap()
}

#[test]
fn review_fix_each_alert_severity_has_its_own_icon() {
    for severity in ["error", "warning", "info", "success"] {
        let html = alert(severity);
        let icon = format!("data-icon=\"alert-{severity}\"");
        assert_eq!(html.matches("<svg").count(), 1, "{severity}");
        assert!(html.contains(&icon), "{severity}: {html}");
    }
}

#[test]
fn review_fix_an_unknown_alert_severity_renders_no_icon() {
    let html = alert("unknown");
    assert!(!html.contains("<svg"), "{html}");
}

#[test]
fn review_fix_each_badge_kind_has_its_own_icon() {
    for kind in ["draft", "approved", "retired"] {
        let html = badge(kind);
        let icon = format!("data-icon=\"status-{kind}\"");
        assert_eq!(html.matches("<svg").count(), 1, "{kind}");
        assert!(html.contains(&icon), "{kind}: {html}");
    }
}

#[test]
fn review_fix_an_unknown_badge_kind_renders_no_icon() {
    let html = badge("unknown");
    assert!(!html.contains("<svg"), "{html}");
}

#[test]
fn review_fix_reserved_classes_render_in_the_test_template() {
    let html = ReservedPage {
        word: "word".to_owned(),
        number: "42".to_owned(),
    }
    .render()
    .unwrap();
    for class in [
        "class=\"btn btn-secondary\"",
        "class=\"btn btn-danger\"",
        "class=\"num\"",
        "class=\"alert alert-warning\"",
        "class=\"alert alert-success\"",
    ] {
        assert!(html.contains(class), "{class}: {html}");
    }
    assert!(html.contains("data-icon=\"alert-warning\""));
    assert!(html.contains("data-icon=\"alert-success\""));
}
