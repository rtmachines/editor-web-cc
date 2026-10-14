//! One sweep over every full GET page of the route table (task 7.2, design D10).
//!
//! The test reads the rows of `routes::table()`. A row without a request in `PAGES` fails the
//! test, so a new GET page cannot skip the checks.

use axum::http::{Method, StatusCode};

use super::page_tests::{LANGS, administrator, assert_assets, assert_layout, get_page};
use super::test_helpers::assert_clean_html;
use crate::components_tests::assert_fixed_svg_tags;
use crate::routes::{Access, table};
use crate::test_support::{Reply, TestSession};

/// The `Content-Security-Policy` of every response (`api.md` section 9).
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";
/// The settings of the `htmx-config` meta tag that a page must hold (`api.md` section 9).
const HTMX_CONFIG: [&str; 7] = [
    "\"allowEval\":false",
    "\"allowScriptTags\":false",
    "\"selfRequestsOnly\":true",
    "\"historyCacheSize\":0",
    "\"methodsThatUseUrlParams\":[\"get\"]",
    "\"includeIndicatorStyles\":false",
    "\"responseHandling\":[{\"code\":\"204\",\"swap\":false},{\"code\":\"[23]..\",\"swap\":true},{\"code\":\"[45]..\",\"swap\":true,\"error\":true},{\"code\":\"...\",\"swap\":false}]",
];
/// The route pattern of the login page. It is the one page that needs no session.
const LOGIN_PATTERN: &str = "/login";
/// Each full GET page of the route table with the path that the test requests.
///
/// `{login}` is the seeded user `user1`.
pub(super) const PAGES: [(&str, &str); 7] = [
    ("/login", "/login"),
    ("/me", "/me"),
    ("/me/password", "/me/password"),
    ("/recipes", "/recipes"),
    ("/users", "/users"),
    ("/users/new", "/users/new"),
    ("/users/{login}", "/users/user1"),
];

/// Return the patterns of the rows `GET` with an operation. Static files are not pages.
fn get_page_patterns() -> Vec<&'static str> {
    table()
        .into_iter()
        .filter(|row| row.method == Method::GET && matches!(row.access, Access::Operation(_)))
        .map(|row| row.path)
        .collect()
}

/// Return the path that the test requests for `pattern`, or `None` for a page without a request.
fn path_of(pattern: &str) -> Option<&'static str> {
    PAGES
        .iter()
        .find(|(known, _)| *known == pattern)
        .map(|(_, path)| *path)
}

/// Assert the headers of every page answer.
fn assert_headers(reply: &Reply, path: &str) {
    assert_eq!(reply.status, StatusCode::OK, "{path}");
    assert_eq!(reply.header("content-security-policy"), CSP, "{path}");
    assert_eq!(reply.header("cache-control"), "no-store", "{path}");
}

/// Assert the markup rules of every full page.
fn assert_markup(reply: &Reply, path: &str) {
    let body = &reply.body;
    for setting in HTMX_CONFIG {
        assert!(body.contains(setting), "{path}: missing {setting}");
    }
    assert_assets(body);
    assert_clean_html(body);
    assert_fixed_svg_tags(body);
    assert!(!body.contains("<style"), "{path}");
    assert!(!body.contains(" style="), "{path}");
}

/// Assert that the page holds the CSRF header of the session, or none for the login page.
fn assert_csrf_header(reply: &Reply, session: &TestSession, pattern: &str) {
    let header = format!("hx-headers='{{\"X-CSRF-Token\":\"{}\"}}'", session.csrf);
    if pattern == LOGIN_PATTERN {
        assert!(!reply.body.contains("hx-headers"), "{pattern}");
    } else {
        assert!(reply.body.contains(&header), "{pattern}");
    }
}

#[test]
fn sweep_every_get_page_of_the_route_table_has_a_request() {
    let patterns = get_page_patterns();
    let stale: Vec<&str> = PAGES
        .iter()
        .map(|(pattern, _path)| *pattern)
        .filter(|pattern| !patterns.contains(pattern))
        .collect();
    assert!(stale.is_empty(), "PAGES holds a stale row: {stale:?}");
    let missing: Vec<&str> = patterns
        .iter()
        .copied()
        .filter(|pattern| path_of(pattern).is_none())
        .collect();
    assert!(missing.is_empty(), "no request in PAGES for: {missing:?}");
}

#[tokio::test]
async fn sweep_every_get_page_has_layout_headers_and_clean_markup_in_both_languages() {
    let (fixture, session) = administrator();
    for pattern in get_page_patterns() {
        let path = path_of(pattern).unwrap();
        let session_of_page = (pattern != LOGIN_PATTERN).then_some(&session);
        for (lang, code) in LANGS {
            let reply = get_page(&fixture, session_of_page, code, path).await;
            assert_headers(&reply, path);
            assert_layout(&reply.body, lang, code);
            assert_markup(&reply, path);
            assert_csrf_header(&reply, &session, pattern);
        }
    }
}
