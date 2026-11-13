//! Tests of the lost-answer template and the HTMX timeout of `base.html` (ADR-0158 (e), (f), (j)).
//!
//! The template `#transport-error` lives in the full page only. The script `request-state.js`
//! copies it into `#error-panel` when no answer arrives.

use axum::http::StatusCode;
use recipe_domain::error_codes::ErrorCode;

use super::page_tests::{LANGS, administrator, get_page};
use crate::test_support::{get_with, text};

/// The registry of settings in `reliability.md` section 7.11.
const REGISTRY: &str = include_str!("../../../../docs/architecture/reliability.md");

/// Return the value of `htmx.request_timeout` of `reliability.md` section 7.11 in milliseconds.
fn registry_timeout_millis() -> u64 {
    let row = REGISTRY
        .lines()
        .find(|line| line.starts_with("| `htmx.request_timeout` |"))
        .unwrap();
    let value = row.split('|').nth(2).unwrap().trim();
    let seconds: u64 = value.split(" s").next().unwrap().parse().unwrap();
    seconds.checked_mul(1000).unwrap()
}

/// Return the text between `<template id="transport-error">` and its closing tag.
fn template_of(body: &str) -> &str {
    let open = "<template id=\"transport-error\">";
    assert!(body.contains(open), "no template in {body}");
    let (_, rest) = body.split_once(open).unwrap();
    rest.split_once("</template>").unwrap().0
}

#[tokio::test]
async fn full_page_sets_the_htmx_timeout_of_the_registry() {
    let (fixture, session) = administrator();
    let reply = get_page(&fixture, Some(&session), "pl", "/me").await;
    let millis = registry_timeout_millis();
    assert_eq!(millis, 45_000);
    let setting = format!("\"timeout\":{millis}");
    assert!(reply.body.contains(&setting), "{}", reply.body);
}

#[tokio::test]
async fn full_page_holds_the_lost_answer_template_after_main() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/me").await;
        assert_eq!(reply.status, StatusCode::OK);
        let body = &reply.body;
        assert!(
            body.find("</main>").unwrap() < body.find("<template id=\"transport-error\">").unwrap(),
            "the template must follow main"
        );
        let template = template_of(body);
        assert!(
            template.contains("<div data-transport-error>"),
            "{template}"
        );
        assert!(template.contains("role=\"alert\""), "{template}");
        assert!(
            template.contains(&format!(
                "<span class=\"alert-code\">{}</span>",
                ErrorCode::WEB_015.as_str()
            )),
            "{template}"
        );
        assert!(
            template.contains(&text(lang, "error-E-WEB-015")),
            "{template}"
        );
        assert!(!template.contains("alert-ref"), "{template}");
    }
}

#[tokio::test]
async fn login_page_holds_the_lost_answer_template() {
    let (fixture, _session) = administrator();
    let reply = get_page(&fixture, None, "en", "/login").await;
    assert!(template_of(&reply.body).contains("E-WEB-015"));
}

#[tokio::test]
async fn fragment_answer_holds_no_lost_answer_template() {
    let (fixture, session) = administrator();
    let cookie = format!("{}; recipe_lang=pl", session.cookie);
    let reply = get_with(
        &fixture.router(),
        "/me",
        &[
            ("cookie", cookie.as_str()),
            ("hx-request", "true"),
            ("hx-target", "content"),
        ],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!reply.body.contains("<html"), "{}", reply.body);
    assert!(!reply.body.contains("transport-error"), "{}", reply.body);
}
