//! Tests of the form answers of `draft_reply` (task 7.2, design D9, ADR-0041 (e) and (h)).

use askama::Template;
use axum::Router;
use axum::extract::State;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use axum::response::Html;
use axum::routing::{MethodRouter, get};
use recipe_app::ports::NonEmpty;
use recipe_domain::audit::RequestId;
use uuid::Uuid;

use super::{refused, with_form};
use crate::WebState;
use crate::error::{FieldError, FieldReason, FormErrors, RenderedForm, WebError};
use crate::i18n::Lang;
use crate::pages::{PageRequest, Texts};
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::{PEER, Reply, get_with, state};

/// The sent text of the field `tag`. The answer must keep it, escaped.
const SENT: &str = "<b>SENT</b>";
/// The CSRF token of the test form. `Debug` of the rendered form must not show it.
const CSRF: &str = "c0ffee00c0ffee00";

#[derive(Template)]
#[template(path = "test/oob-form.html")]
struct OobForm<'a> {
    t: Texts<'a>,
    oob: bool,
    value: String,
    row_version: i64,
    csrf: String,
    errors: FormErrors,
}

fn form(state: &WebState, lang: Lang, oob: bool, errors: FormErrors) -> OobForm<'_> {
    OobForm {
        t: Texts::new(state.bundles(), lang),
        oob,
        value: SENT.to_owned(),
        row_version: 7,
        csrf: CSRF.to_owned(),
        errors,
    }
}

fn tag_line() -> NonEmpty<FieldError> {
    NonEmpty::new(FieldError::bad_request("tag", FieldReason::Tag))
}

async fn refuse(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<&'static str, WebError> {
    let lang = request.lang();
    Err(refused(&state, &request, tag_line(), |errors| {
        form(&state, lang, true, errors)
    }))
}

async fn broken(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<&'static str, WebError> {
    let lang = request.lang();
    Err(refused(&state, &request, tag_line(), |errors| {
        form(&state, lang, false, errors)
    }))
}

async fn saved(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Html<String>, WebError> {
    let mut page = form(&state, request.lang(), true, FormErrors::default());
    page.value = "SPEED".to_owned();
    with_form(Html("<p>saved</p>".to_owned()), &page, request.request_id())
}

/// Build the router with the test routes. None of them needs a session.
fn router() -> Router {
    let route = |path: &'static str, handler: MethodRouter<WebState>| RouteEntry {
        method: Method::GET,
        path,
        access: Access::Static,
        handler,
    };
    let rows = vec![
        route("/test/refused", get(refuse)),
        route("/test/broken", get(broken)),
        route("/test/saved", get(saved)),
    ];
    assemble(state(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

async fn ask(path: &str, htmx: bool) -> Reply {
    let mut headers = vec![("cookie", "recipe_lang=en")];
    if htmx {
        headers.push(("hx-request", "true"));
    }
    get_with(&router(), path, &headers).await
}

/// Return the text between `start` and the next `end` after it.
fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let from = text.find(start).unwrap().checked_add(start.len()).unwrap();
    let rest = text.get(from..).unwrap();
    rest.get(..rest.find(end).unwrap()).unwrap()
}

#[tokio::test]
async fn draft_reply_an_htmx_error_answer_swaps_the_form_out_of_band_after_the_summary() {
    let reply = ask("/test/refused", true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(reply.header("hx-push-url"), "false");
    let summary = reply
        .body
        .find("<div class=\"alert alert-error\">")
        .unwrap();
    let form = reply
        .body
        .find("<form id=\"test-form\" method=\"post\" action=\"/test/form\" hx-swap-oob=\"true\">")
        .unwrap();
    assert!(summary < form, "{}", reply.body);
    // After the form, the answer empties the status region out of band (design-system.md 10.7).
    assert!(
        reply
            .body
            .trim_end()
            .ends_with("</form><div id=\"toast\" hx-swap-oob=\"innerHTML\"></div>"),
        "{}",
        reply.body
    );
    assert_eq!(reply.body.matches("hx-swap-oob").count(), 2);
}

#[tokio::test]
async fn draft_reply_the_form_keeps_the_sent_text_and_the_sent_row_version() {
    let reply = ask("/test/refused", true).await;
    assert!(
        reply.body.contains("value=\"&#60;b&#62;SENT&#60;/b&#62;\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains(SENT), "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<input type=\"hidden\" name=\"row_version\" value=\"7\">"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn draft_reply_the_form_shows_the_line_of_the_summary_under_its_input() {
    let reply = ask("/test/refused", true).await;
    assert!(
        reply
            .body
            .contains(" aria-invalid=\"true\" aria-describedby=\"tag-error\">"),
        "{}",
        reply.body
    );
    let summary = between(
        &reply.body,
        "<a class=\"target-link\" href=\"#tag\">",
        "</a>",
    );
    let under = between(
        &reply.body,
        "<div class=\"field-error\" id=\"tag-error\"><p>",
        "</p>",
    );
    let summary_text = between(summary, "</span> <span>", "</span>");
    let under_text = between(under, "</span> <span>", "</span>");
    assert_eq!(summary_text, under_text);
    assert!(
        summary_text.starts_with("Field Tag is not valid: "),
        "{summary_text}"
    );
}

#[tokio::test]
async fn draft_reply_a_full_page_answer_holds_no_form() {
    let reply = ask("/test/refused", false).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(!reply.body.contains("id=\"test-form\""), "{}", reply.body);
    assert!(!reply.body.contains(CSRF), "{}", reply.body);
}

#[tokio::test]
async fn draft_reply_a_form_without_the_out_of_band_attribute_is_a_defect() {
    let reply = ask("/test/broken", true).await;
    assert_eq!(
        reply.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-010"), "{}", reply.body);
    assert!(!reply.body.contains(CSRF), "{}", reply.body);
}

#[tokio::test]
async fn draft_reply_the_clean_form_after_a_success_holds_the_saved_text_and_no_error_line() {
    let reply = ask("/test/saved", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.starts_with("<p>saved</p><form id=\"test-form\""),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains(" hx-swap-oob=\"true\""),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("value=\"SPEED\""), "{}", reply.body);
    assert!(!reply.body.contains("aria-invalid"), "{}", reply.body);
    assert!(!reply.body.contains("field-error"), "{}", reply.body);
}

#[test]
fn draft_reply_the_debug_of_a_rendered_form_writes_only_the_length() {
    let state = state();
    let page = form(&state, Lang::En, true, FormErrors::default());
    let html = page.render().unwrap();
    let rendered = RenderedForm::from_template(&page, RequestId::from(Uuid::nil())).unwrap();
    assert_eq!(rendered.as_str(), html);
    let debug = format!("{rendered:?}");
    assert_eq!(debug, format!("RenderedForm {{ len: {} }}", html.len()));
    assert!(!debug.contains(CSRF));
}

#[test]
fn draft_reply_a_rendered_form_must_open_with_a_form_tag_with_an_id_and_the_oob_attribute() {
    let state = state();
    let page = form(&state, Lang::En, false, FormErrors::default());
    let result = RenderedForm::from_template(&page, RequestId::from(Uuid::nil()));
    assert!(matches!(result, Err(WebError::Internal { .. })));
}
