//! Tests of the error fragments: one alert, field lines, and `HX-Reswap` (task 5.5, design D5, D6).
//!
//! Each test names a scenario of the delta spec `web-ui`. The answers come from the router of the
//! fixture, so the tests read the real markup.

use std::fs;
use std::path::{Path, PathBuf};

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use super::view::field_key;
use crate::i18n::{Bundles, Lang};
use crate::settings::{CidrList, WebSettings};
use crate::test_support::source_walk::{files_below, is_test_path};
use crate::test_support::visible_text::visible_text;
use crate::test_support::{
    CSRF_KEY_NEW, Fixture, Reply, TestSession, get_with, icon_tag, post_fields, post_pre_session,
    text,
};

/// The value of `HX-Reswap` for an HTMX error (ADR-0036 p, `api.md` section 5.3).
const RESWAP: &str = "innerHTML show:#error-panel:top";
/// The number of characters that is one more than a password may have.
const TOO_LONG: usize = 129;
/// The start of a list. An error without field lines holds none.
const LIST_OPEN: &str = "<ul";

/// Send `POST /me/password` with a current password that is too long.
async fn too_long_password(
    fixture: &Fixture,
    session: &TestSession,
    code: &str,
    htmx: bool,
) -> Reply {
    let cookie = format!("recipe_lang={code}");
    let mut headers = vec![("cookie", cookie.as_str())];
    if htmx {
        headers.push(("hx-request", "true"));
    }
    let body = format!("current_password={}&password=x", "a".repeat(TOO_LONG));
    post_fields(
        &fixture.router(),
        "/me/password",
        Some(session),
        &headers,
        &body,
    )
    .await
}

/// A fixture with the Operator `user2` and a session of that user.
fn operator() -> (Fixture, TestSession) {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    (fixture, session)
}

// --- Spec "Field error lines" -------------------------------------------------------------------

#[tokio::test]
async fn fragment_too_long_current_password_links_the_label_in_english() {
    let (fixture, session) = operator();
    let reply = too_long_password(&fixture, &session, "en", true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-008"));
    let lines = "<ul class=\"field-lines\"><li><a class=\"target-link\" href=\"#current_password\">Current password</a></li></ul>";
    assert!(reply.body.contains(lines), "{}", reply.body);
    assert!(!reply.body.contains("<span class=\"field\">"));
}

#[tokio::test]
async fn fragment_too_long_current_password_links_the_label_in_polish() {
    let (fixture, session) = operator();
    let reply = too_long_password(&fixture, &session, "pl", true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("<a class=\"target-link\" href=\"#current_password\">Obecne hasło</a>")
    );
}

#[tokio::test]
async fn fragment_full_error_page_shows_the_label_without_a_link() {
    let (fixture, session) = operator();
    let reply = too_long_password(&fixture, &session, "en", false).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("<ul class=\"field-lines\"><li>Current password</li></ul>"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("href=\"#current_password\""));
}

#[tokio::test]
async fn fragment_login_page_links_the_label_to_the_input() {
    let fixture = Fixture::new();
    let reply = post_pre_session(&fixture, "/login", "login=&password=x", &[]).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let expected = format!(
        "<ul class=\"field-lines\"><li><a class=\"target-link\" href=\"#login\">{}</a></li></ul>",
        text(Lang::Pl, "field-login")
    );
    assert!(reply.body.contains(&expected), "{}", reply.body);
    assert!(
        reply.body.contains("id=\"login\""),
        "the link needs its input"
    );
}

#[tokio::test]
async fn fragment_sentence_of_bad_request_names_the_label_in_english() {
    let (fixture, session) = operator();
    let reply = too_long_password(&fixture, &session, "en", true).await;
    let text = visible_text(&reply.body);
    assert!(
        text.contains("Field Current password is not valid"),
        "{text}"
    );
    assert!(!text.contains("current_password"), "{text}");
    assert!(reply.body.contains("href=\"#current_password\""));
}

#[tokio::test]
async fn fragment_sentence_of_bad_request_names_the_label_in_polish() {
    let (fixture, session) = operator();
    let reply = too_long_password(&fixture, &session, "pl", true).await;
    let text = visible_text(&reply.body);
    assert!(text.contains("Obecne hasło"), "{text}");
    assert!(!text.contains("current_password"), "{text}");
}

/// Send an HTMX login from a trusted proxy with an `X-Forwarded-For` entry that is no address.
async fn forwarded_reply() -> Reply {
    let web = WebSettings {
        trusted_proxies: CidrList::parse_all(["0.0.0.0/0"]).unwrap(),
        ..WebSettings::default()
    };
    let fixture = Fixture::with_settings(web, CSRF_KEY_NEW);
    let headers = [
        ("hx-request", "true"),
        ("x-forwarded-for", "not-an-address"),
    ];
    post_pre_session(&fixture, "/login", "login=abc&password=x", &headers).await
}

/// Send an HTMX request that gives `E-WEB-008` for the field `kind`.
async fn bad_field_reply(kind: &str) -> Reply {
    let fixture = Fixture::new();
    let admin = fixture.seed_session(1, Role::Administrator, false);
    let htmx = [
        ("cookie", admin.cookie.as_str()),
        ("hx-request", "true"),
        ("x-forwarded-for", "not-an-address"),
    ];
    let router = fixture.router();
    match kind {
        "page" => get_with(&router, "/users?page=0", &htmx[..2]).await,
        "q" => get_with(&router, "/users?q=BAD!", &htmx[..2]).await,
        "query" => get_with(&router, "/users?unknown=1", &htmx[..2]).await,
        "role" => {
            post_fields(
                &router,
                "/users",
                Some(&admin),
                &htmx[1..2],
                "login=abc&role=bad",
            )
            .await
        }
        "X-Forwarded-For" => forwarded_reply().await,
        "lang" => post_pre_session(&fixture, "/lang", "lang=xx&next=%2F", &htmx[1..2]).await,
        _form => post_pre_session(&fixture, "/lang", "lang=en&next=%2F&extra=1", &htmx[1..2]).await,
    }
}

#[tokio::test]
async fn fragment_field_with_an_input_links_to_it() {
    for kind in ["q", "role"] {
        let reply = bad_field_reply(kind).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{kind}");
        let link = format!("<li><a class=\"target-link\" href=\"#{kind}\">");
        assert!(reply.body.contains(&link), "{kind}: {}", reply.body);
    }
}

#[tokio::test]
async fn fragment_field_without_an_input_shows_the_label_without_a_link() {
    for kind in ["page", "query", "lang", "form", "X-Forwarded-For"] {
        let reply = bad_field_reply(kind).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{kind}");
        assert!(reply.body.contains("field-lines"), "{kind}: {}", reply.body);
        assert!(!reply.body.contains("href=\"#"), "{kind}: {}", reply.body);
        assert!(!reply.body.contains(&format!("<li>{kind}</li>")), "{kind}");
    }
}

#[tokio::test]
async fn fragment_wrong_credentials_render_no_field_list() {
    let fixture = Fixture::new();
    let reply = post_pre_session(
        &fixture,
        "/login",
        "login=nobody&password=wrong+password+here",
        &[],
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
    assert!(reply.body.contains("E-IDN-003"));
    assert!(!reply.body.contains("field-lines"), "{}", reply.body);
}

// --- Spec "Error fragment as one alert" ---------------------------------------------------------

#[tokio::test]
async fn fragment_error_is_one_alert_with_icon_code_sentence_and_reference() {
    let (fixture, session) = operator();
    let cookie = format!("{}; recipe_lang=en", session.cookie);
    let reply = get_with(
        &fixture.router(),
        "/users",
        &[("cookie", cookie.as_str()), ("hx-request", "true")],
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    // The answer ends with the empty status region out of band (design-system.md 10.7).
    let body = reply
        .body
        .trim()
        .strip_suffix("<div id=\"toast\" hx-swap-oob=\"innerHTML\"></div>")
        .unwrap();
    let open = format!(
        "<div class=\"alert alert-error\" role=\"alert\">{}",
        icon_tag("alert-error")
    );
    assert!(body.starts_with(&open), "{body}");
    assert!(body.ends_with("</span></div>"), "{body}");
    assert_eq!(body.matches("role=\"alert\"").count(), 1);
    let code = body
        .find("<span class=\"alert-code\">E-WEB-002</span>")
        .unwrap();
    let reference = body.find("<span class=\"alert-ref\">Reference: ").unwrap();
    assert!(code < reference);
    assert!(!body.contains(LIST_OPEN), "{body}");
}

#[tokio::test]
async fn fragment_field_lines_stand_between_the_sentence_and_the_reference() {
    let (fixture, session) = operator();
    let reply = too_long_password(&fixture, &session, "en", true).await;
    let body = reply.body.trim();
    assert_eq!(body.matches("role=\"alert\"").count(), 1);
    let code = body.find("<span class=\"alert-code\">").unwrap();
    let lines = body.find("<ul class=\"field-lines\">").unwrap();
    let reference = body.find("<span class=\"alert-ref\">").unwrap();
    assert!(code < lines && lines < reference, "{body}");
}

// --- Spec "HTMX error headers" ------------------------------------------------------------------

#[tokio::test]
async fn fragment_htmx_error_scrolls_the_error_panel_into_view() {
    let (fixture, session) = operator();
    let reply = too_long_password(&fixture, &session, "en", true).await;
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(reply.header("hx-reswap"), RESWAP);
}

#[tokio::test]
async fn fragment_htmx_conflict_409_scrolls_the_error_panel_into_view() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    fixture.seed_account(2, Role::Operator, "correct horse battery", false);
    let headers = [("hx-request", "true")];
    let path = "/users/user2/activate";
    let reply = post_fields(&fixture.router(), path, Some(&session), &headers, "").await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(reply.body.contains("E-IDN-009"), "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(reply.header("hx-reswap"), RESWAP);
}

#[tokio::test]
async fn fragment_a_page_error_sets_no_reswap() {
    let (fixture, session) = operator();
    let reply = too_long_password(&fixture, &session, "en", false).await;
    assert_eq!(reply.header("hx-reswap"), "");
}

// --- Decision D8: every field name of a BadRequest or a BadQuery has its label -----------------

/// Collect the `.rs` files below `src/` that hold no test code.
fn source_files() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    files_below(&manifest.join("src"))
        .into_iter()
        .filter(|path| path.extension().is_some_and(|suffix| suffix == "rs"))
        .filter(|path| !is_test_path(path.strip_prefix(manifest).unwrap()))
        .collect()
}

/// Return the value of `const NAME: &str = "value";` in `source`.
fn constant_value(source: &str, name: &str) -> String {
    let marker = format!("const {name}: &str = \"");
    let (_before, rest) = source.split_once(&marker).unwrap();
    rest.split('"').next().unwrap().to_owned()
}

/// Return the field names that `BadRequest { field: ... }` and `BadQuery { field: ... }` name in
/// the code of `source`.
fn bad_request_fields(source: &str) -> Vec<String> {
    let code = source.split("#[cfg(test)]").next().unwrap();
    let mut names = Vec::new();
    let starts = ["BadRequest {", "BadQuery {"]
        .into_iter()
        .flat_map(|marker| code.split(marker).skip(1));
    for rest in starts {
        let Some((between, after)) = rest.split_once("field:") else {
            continue;
        };
        if between.contains('}') {
            continue;
        }
        let value = after.trim_start();
        if let Some(literal) = value.strip_prefix('"') {
            names.push(literal.split('"').next().unwrap().to_owned());
        } else if !value.starts_with('&') {
            let ident: String = value
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            names.push(constant_value(code, &ident));
        }
    }
    names
}

#[test]
fn fragment_every_bad_request_field_has_a_label_in_both_bundles() {
    let files = source_files();
    let mut names: Vec<String> = files
        .iter()
        .flat_map(|path| bad_request_fields(&fs::read_to_string(path).unwrap()))
        .collect();
    names.sort();
    names.dedup();
    for known in [
        "lang",
        "login",
        "password",
        "current_password",
        "q",
        "page",
        "form",
        "kind",
    ] {
        assert!(
            names.iter().any(|name| name == known),
            "scan lost {known}: {names:?}"
        );
    }
    let bundles = Bundles::load().unwrap();
    for name in &names {
        for lang in [Lang::Pl, Lang::En] {
            let key = field_key(name);
            assert!(
                bundles.message(lang, &key, None).is_some(),
                "missing {key} in {lang}"
            );
        }
    }
}

#[test]
fn fragment_field_key_replaces_each_underscore_with_a_hyphen() {
    assert_eq!(field_key("current_password"), "field-current-password");
    assert_eq!(field_key("q"), "field-q");
}
