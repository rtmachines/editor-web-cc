//! The XSS corpus through each text field of the new pages (task 6.10, design D19).
//!
//! Each field gets every payload of `tests/web/xss-corpus.txt` and one NUL payload. A rendered
//! answer holds no markup of a payload, and no `hx-*` attribute holds a payload.

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::{Fixture, Reply, TestSession, get_with, post_fields, post_pre_session};

/// The payloads of the corpus file, one per line. A line that starts with `#` is a comment.
const CORPUS: &str = include_str!("../../tests/web/xss-corpus.txt");
/// A payload with a NUL character. A text file cannot hold it, so the test adds it.
const NUL_PAYLOAD: &str = "\u{0}<script>alert(1)</script>";
/// The markup that a payload brings into a page. A page holds none of it.
const FORBIDDEN_MARKUP: [&str; 4] = ["<img", "<svg", "onerror=", "onload="];

/// Return the payloads of the corpus file and the NUL payload.
fn payloads() -> Vec<String> {
    let mut payloads: Vec<String> = CORPUS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    payloads.push(NUL_PAYLOAD.to_owned());
    payloads
}

/// Percent-encode every byte that is not an unreserved character, so a payload fits a form or a path.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// Assert that an answer holds no markup of `payload` (design D19).
///
/// The page keeps its own scripts with `src`. No inline script, no image, no SVG, and no event
/// handler may appear, and no `hx-*` attribute may hold the payload.
fn assert_escaped(reply: &Reply, payload: &str, what: &str) {
    assert_clean_html(&reply.body);
    for marker in FORBIDDEN_MARKUP {
        assert!(
            !reply.body.contains(marker),
            "{what}: {marker} in {}",
            reply.body
        );
    }
    assert_no_user_text_in_hx_attributes(&reply.body, payload);
}

/// A fixture with the Administrator `user1` and a session of that user.
fn administrator() -> (Fixture, TestSession) {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    (fixture, session)
}

#[tokio::test]
async fn xss_corpus_login_field_of_post_login_is_escaped() {
    for payload in payloads() {
        let fixture = Fixture::new();
        let fields = format!("login={}&password=wrong", encode(&payload));
        let reply = post_pre_session(&fixture, "/login", &fields, &[]).await;
        assert_escaped(&reply, &payload, &format!("login {payload:?}"));
    }
}

#[tokio::test]
async fn xss_corpus_search_field_of_get_users_is_escaped() {
    for payload in payloads() {
        let (fixture, session) = administrator();
        let path = format!("/users?q={}", encode(&payload));
        let cookie = [("cookie", session.cookie.as_str())];
        let reply = get_with(&fixture.router(), &path, &cookie).await;
        assert_escaped(&reply, &payload, &format!("q {payload:?}"));
    }
}

#[tokio::test]
async fn xss_corpus_login_field_of_post_users_is_escaped() {
    for payload in payloads() {
        let (fixture, session) = administrator();
        let fields = format!("login={}&role=operator", encode(&payload));
        let reply = post_fields(&fixture.router(), "/users", Some(&session), &[], &fields).await;
        assert_escaped(&reply, &payload, &format!("login {payload:?}"));
    }
}

#[tokio::test]
async fn xss_corpus_next_field_of_post_lang_is_escaped() {
    for payload in payloads() {
        let fixture = Fixture::new();
        let next = format!("/{payload}");
        let fields = format!("lang=en&next={}", encode(&next));
        let reply = post_pre_session(&fixture, "/lang", &fields, &[]).await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "next {payload:?}");
        assert_escaped(&reply, &payload, &format!("next {payload:?}"));
    }
}

#[tokio::test]
async fn xss_corpus_path_value_of_get_users_login_is_escaped() {
    for payload in payloads() {
        let (fixture, session) = administrator();
        let path = format!("/users/{}", encode(&payload));
        let cookie = [("cookie", session.cookie.as_str())];
        let reply = get_with(&fixture.router(), &path, &cookie).await;
        assert_escaped(&reply, &payload, &format!("path {payload:?}"));
    }
}
