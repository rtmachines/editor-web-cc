//! The XSS corpus through each text field of the new pages (task 6.10, design D19).
//!
//! Each field gets every payload of `tests/web/xss-corpus.txt` and one NUL payload. A rendered
//! answer holds no markup of a payload, and no `hx-*` attribute holds a payload.

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use super::sweep_tests::PAGES;
use crate::components_tests::assert_fixed_svg_tags;
use crate::pages::test_helpers::{
    assert_clean_html, assert_no_user_text_in_hx_attributes, field_value,
};
use crate::test_support::{Fixture, Reply, TestSession, get_with, post_fields, post_pre_session};

/// The payloads of the corpus file, one per line. A line that starts with `#` is a comment.
const CORPUS: &str = include_str!("../../tests/web/xss-corpus.txt");
/// A payload with a NUL character. A text file cannot hold it, so the test adds it.
const NUL_PAYLOAD: &str = "\u{0}<script>alert(1)</script>";
/// The markup that a payload brings into a page. A page holds none of it.
///
/// An `<svg` tag is not in the list. A page may hold the fixed icon tags of `design-system.md`
/// section 8, so `assert_fixed_svg_tags` checks each `<svg` instead (design D10).
const FORBIDDEN_MARKUP: [&str; 3] = ["<img", "onerror=", "onload="];

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
    assert_fixed_svg_tags(&reply.body);
    assert_no_user_text_in_hx_attributes(&reply.body, payload);
}

/// A fixture with the Administrator `user1` and a session of that user.
fn administrator() -> (Fixture, TestSession) {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    (fixture, session)
}

/// Bytes that `http` accepts raw in a query, each with a payload around it.
const RAW_QUERY_PAYLOADS: [&str; 6] = [
    "'onfocus=alert(1)'",
    "`onfocus=alert(1)`",
    "{{7*7}}",
    "\\'};alert(1);//",
    "a&b=(c)",
    "javascript:alert(1)",
];
/// The route patterns whose handler has a `Query` extractor. A foreign query name is an error there.
const PATTERNS_WITH_QUERY_EXTRACTOR: [&str; 1] = ["/users"];
/// The route pattern of the login page. It needs no session and keeps no query in the field `next`.
const LOGIN_PATTERN: &str = "/login";

/// Return the pattern and the path of every GET page that accepts any query (`sweep_tests::PAGES`).
fn query_pages() -> Vec<(&'static str, &'static str)> {
    PAGES
        .into_iter()
        .filter(|(pattern, _path)| !PATTERNS_WITH_QUERY_EXTRACTOR.contains(pattern))
        .collect()
}

/// Send `GET path` as the session, or without a session for the login page.
async fn get_query_page(
    fixture: &Fixture,
    session: &TestSession,
    pattern: &str,
    path: &str,
) -> Reply {
    let cookie = [("cookie", session.cookie.as_str())];
    let headers: &[(&str, &str)] = if pattern == LOGIN_PATTERN {
        &[]
    } else {
        &cookie
    };
    get_with(&fixture.router(), path, headers).await
}

/// Escape a text like askama does in an HTML attribute (`&`, `<`, `>`, `"`, and `'`).
fn askama_escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '&' => "&#38;".to_owned(),
            '<' => "&#60;".to_owned(),
            '>' => "&#62;".to_owned(),
            '"' => "&#34;".to_owned(),
            '\'' => "&#39;".to_owned(),
            other => other.to_string(),
        })
        .collect()
}

/// Assert that a page shows `path` escaped in the field `next` and holds no markup of `payload`.
///
/// The login page keeps no query in `next`, so it must hold no markup and no payload at all.
fn assert_reflected_safely(reply: &Reply, pattern: &str, path: &str, payload: &str) {
    assert_eq!(reply.status, StatusCode::OK, "{path}");
    assert_clean_html(&reply.body);
    assert_fixed_svg_tags(&reply.body);
    assert_no_user_text_in_hx_attributes(&reply.body, payload);
    assert!(!reply.body.contains("<img"), "{path}");
    if pattern == LOGIN_PATTERN {
        assert!(!reply.body.contains(payload), "{path}");
        return;
    }
    assert_eq!(
        field_value(&reply.body, "next"),
        askama_escape(path),
        "{path}"
    );
}

#[tokio::test]
async fn xss_next_field_of_the_switch_escapes_a_quote_of_the_query() {
    let (fixture, session) = administrator();
    let cookie = [("cookie", session.cookie.as_str())];
    let reply = get_with(&fixture.router(), "/me?x='onfocus=alert(1)'", &cookie).await;
    assert_eq!(
        field_value(&reply.body, "next"),
        "/me?x=&#39;onfocus=alert(1)&#39;"
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn xss_next_field_of_the_switch_escapes_the_raw_bytes_of_a_query() {
    let (fixture, session) = administrator();
    for (pattern, page) in query_pages() {
        for payload in RAW_QUERY_PAYLOADS {
            let path = format!("{page}?x={payload}");
            let reply = get_query_page(&fixture, &session, pattern, &path).await;
            assert_reflected_safely(&reply, pattern, &path, payload);
        }
    }
}

#[test]
fn xss_the_query_pages_are_every_page_without_a_query_extractor() {
    let patterns: Vec<&str> = query_pages().iter().map(|(pattern, _)| *pattern).collect();
    assert_eq!(
        patterns,
        [
            "/login",
            "/me",
            "/me/password",
            "/recipes",
            "/users/new",
            "/users/{login}"
        ]
    );
}

#[tokio::test]
async fn xss_corpus_in_the_query_of_the_pages_is_escaped() {
    let (fixture, session) = administrator();
    for payload in payloads() {
        for (pattern, page) in query_pages() {
            let path = format!("{page}?x={}", encode(&payload));
            let reply = get_query_page(&fixture, &session, pattern, &path).await;
            assert_reflected_safely(&reply, pattern, &path, &payload);
            assert!(!reply.body.contains(&payload), "{path}");
        }
    }
}

#[tokio::test]
async fn xss_pages_with_icons_hold_only_the_fixed_svg_tags() {
    let (fixture, session) = administrator();
    for path in ["/users", "/users/user1"] {
        let cookie = [("cookie", session.cookie.as_str())];
        let reply = get_with(&fixture.router(), path, &cookie).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        assert!(reply.body.contains("<svg"), "{path} shows no icon");
        assert_escaped(&reply, "<svg", path);
    }
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
