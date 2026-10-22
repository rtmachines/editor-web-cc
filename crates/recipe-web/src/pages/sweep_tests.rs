//! One sweep over every full GET page of the route table (task 7.2, design D10).
//!
//! The test reads the rows of `routes::table()`. A row without a request in `pages()` fails the
//! test, so a new GET page cannot skip the checks. The three areas of design D12 add their own
//! pages to the list, and the sweep fixture holds the plant data of `seed_plant`.

use axum::http::{Method, StatusCode};

use super::page_tests::{LANGS, administrator, assert_assets, assert_layout, get_page};
use super::test_helpers::{CSP, assert_clean_html};
use super::{machines, parameters, recipes, types, units, versions};
use crate::components_tests::assert_fixed_svg_tags;
use crate::routes::{Access, table};
use crate::test_support::{Fixture, Reply, TestSession, deactivate_user};
use recipe_domain::identity::Role;

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
/// The six full GET pages of step P3.1 with the path that the test requests.
///
/// The page `/recipes` moved to the area `recipes` (design D12). `{login}` is the seeded user
/// `user1`.
const P31_PAGES: [(&str, &str); 6] = [
    ("/login", "/login"),
    ("/me", "/me"),
    ("/me/password", "/me/password"),
    ("/users", "/users"),
    ("/users/new", "/users/new"),
    ("/users/{login}", "/users/user1"),
];

/// The history page, the audit pages, and the security events page of P3.4 (ADR-0051).
///
/// The audit entry `1` is the first entry that the sweep world writes.
const P34_PAGES: [(&str, &str); 4] = [
    ("/recipes/{ref}/history", "/recipes/R-100/history"),
    ("/audit", "/audit"),
    ("/audit/{id}", "/audit/1"),
    ("/security-events", "/security-events"),
];

/// Return each full GET page of the route table with the path that the test requests.
///
/// The six pages of P3.1 come first. The test modules of the six areas of design D12 add the
/// pages of the machines, the units, the types, the recipes, the versions, and the parameters.
/// The four pages of P3.4 come last.
fn pages() -> Vec<(&'static str, String)> {
    let mut pages: Vec<(&'static str, String)> = P31_PAGES
        .iter()
        .map(|(pattern, path)| (*pattern, (*path).to_owned()))
        .collect();
    pages.extend(machines::route_tests::sweep_pages());
    pages.extend(units::route_tests::sweep_pages());
    pages.extend(types::route_tests::sweep_pages());
    pages.extend(recipes::route_tests::sweep_pages());
    pages.extend(versions::route_tests::sweep_pages());
    pages.extend(parameters::route_tests::sweep_pages());
    pages.extend(
        P34_PAGES
            .iter()
            .map(|(pattern, path)| (*pattern, (*path).to_owned())),
    );
    pages
}

/// Write the audit entry `1` through a use case, so that `GET /audit/1` answers `200`.
///
/// The function solves one problem: the sweep fixtures hold no audit entry, and a page of the list
/// `P34_PAGES` shows one. The use case `DeactivateUser` writes the entry of the user `user8`.
pub(super) async fn seed_audit_entry(fixture: &Fixture) {
    let _operator = fixture.seed_session(8, Role::Operator, false);
    deactivate_user(fixture, "user8").await;
}

/// Return the patterns of the rows `GET` with an operation. Static files are not pages.
pub(super) fn get_page_patterns() -> Vec<&'static str> {
    table()
        .into_iter()
        .filter(|row| row.method == Method::GET && matches!(row.access, Access::Operation(_)))
        .map(|row| row.path)
        .collect()
}

/// Return the path that the test requests for `pattern`, or `None` for a page without a request.
pub(super) fn path_of(pattern: &str) -> Option<String> {
    pages()
        .into_iter()
        .find(|(known, _)| *known == pattern)
        .map(|(_, path)| path)
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
    let listed = pages();
    let stale: Vec<&str> = listed
        .iter()
        .map(|(pattern, _path)| *pattern)
        .filter(|pattern| !patterns.contains(pattern))
        .collect();
    assert!(stale.is_empty(), "pages() holds a stale row: {stale:?}");
    let missing: Vec<&str> = patterns
        .iter()
        .copied()
        .filter(|pattern| path_of(pattern).is_none())
        .collect();
    assert!(missing.is_empty(), "no request in pages() for: {missing:?}");
}

#[test]
fn sweep_pages_collect_the_pages_of_p31_and_of_each_area() {
    let listed = pages();
    for (pattern, path) in P31_PAGES {
        assert!(listed.contains(&(pattern, path.to_owned())), "{pattern}");
    }
    let areas = [
        ("machines", machines::route_tests::sweep_pages()),
        ("units", units::route_tests::sweep_pages()),
        ("types", types::route_tests::sweep_pages()),
        ("recipes", recipes::route_tests::sweep_pages()),
        ("versions", versions::route_tests::sweep_pages()),
        ("parameters", parameters::route_tests::sweep_pages()),
    ];
    for (area, area_pages) in &areas {
        for page in area_pages {
            assert!(listed.contains(page), "{area}: {page:?} is not in pages()");
        }
    }
    let area_count: usize = areas.iter().map(|(_area, rows)| rows.len()).sum();
    for (pattern, path) in P34_PAGES {
        assert!(listed.contains(&(pattern, path.to_owned())), "{pattern}");
    }
    let expected = P31_PAGES
        .len()
        .checked_add(area_count)
        .and_then(|count| count.checked_add(P34_PAGES.len()))
        .unwrap();
    assert_eq!(listed.len(), expected);
}

#[tokio::test]
async fn sweep_every_get_page_has_layout_headers_and_clean_markup_in_both_languages() {
    let (fixture, session) = administrator();
    fixture.seed_recipes();
    seed_audit_entry(&fixture).await;
    for pattern in get_page_patterns() {
        let path = path_of(pattern).unwrap();
        let session_of_page = (pattern != LOGIN_PATTERN).then_some(&session);
        for (lang, code) in LANGS {
            let reply = get_page(&fixture, session_of_page, code, &path).await;
            assert_headers(&reply, &path);
            assert_layout(&reply.body, lang, code);
            assert_markup(&reply, &path);
            assert_csrf_header(&reply, &session, pattern);
        }
    }
}
