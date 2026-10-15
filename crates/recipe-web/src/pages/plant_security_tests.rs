//! Missing CSRF token and read-only mode on every `POST` route of the plant pages (task 11.3, D14).
//!
//! The coverage test of `routes::tests` (`route_coverage_each_mutating_route_rejects_four_bad_requests`)
//! already sends four defects to every mutating row of the route table: no token, the token of another
//! session, a foreign `Origin`, and no `Origin` with no fetch metadata. The plant routes are in that
//! table, so a plain `POST` without a token is covered. This module adds only what that test lacks:
//! the HTMX request, the answer with `add=1`, the token in the query, and read-only mode.

use std::collections::BTreeSet;

use axum::body::Body;
use axum::http::{Method, StatusCode};

use super::page_tests::administrator;
use super::test_helpers::assert_error;
use super::{machines, types, units};
use crate::routes::matrix::fill;
use crate::test_support::plant::HTMX;
use crate::test_support::{Fixture, Reply, TestSession, call, get_with, post_fields, request};

/// The body that defines a new struct with one member.
const NEW_BODY: &str = "name=ST_P&members[0].name=a&members[0].type=INT";
/// The same body with the button "Add a member".
const NEW_ADD_BODY: &str = "name=ST_P&members[0].name=a&members[0].type=INT&add=1";
/// The body that defines a new revision of `ST_Zone`.
const REVISION_BODY: &str = "members[0].name=a&members[0].type=INT";
/// The same body with the button "Add a member".
const REVISION_ADD_BODY: &str = "members[0].name=a&members[0].type=INT&add=1";
/// Every `POST` route of the plant pages with a valid body, also the two routes with `add=1`.
const COMMANDS: [(&str, &str); 10] = [
    (
        "/machines",
        "code=M99&name=C&plc_platform=S7_300&main_unit_name=I",
    ),
    ("/machines/{code}/rename", "name=Renamed"),
    ("/machines/{code}/archive", ""),
    (
        "/machines/{code}/units",
        "parent_path={path}&segment=5&name=Added",
    ),
    ("/machines/{code}/units/{path}/rename", "name=Renamed"),
    ("/machines/{code}/units/{path}/archive", ""),
    ("/machines/{code}/types", NEW_BODY),
    ("/machines/{code}/types", NEW_ADD_BODY),
    ("/machines/{code}/types/{name}/revisions", REVISION_BODY),
    ("/machines/{code}/types/{name}/revisions", REVISION_ADD_BODY),
];

/// Return the templates of the mutating routes that the three areas register.
fn mutating_templates() -> BTreeSet<&'static str> {
    let mut rows = machines::rows();
    rows.extend(units::rows());
    rows.extend(types::rows());
    rows.into_iter()
        .filter(|row| crate::csrf::is_mutating(&row.method))
        .map(|row| row.path)
        .collect()
}

/// Send a same-origin `POST` as the session with no token in the header and none in the body.
///
/// `query` is appended to the path. The request has the header of an HTMX request when `htmx` is true.
async fn post_without_token(
    fixture: &Fixture,
    session: &TestSession,
    target: (&str, &str),
    query: &str,
    htmx: bool,
) -> Reply {
    let (path, body) = target;
    let uri = format!("{}{query}", fill(path));
    let mut builder = request(Method::POST, &uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str());
    if htmx {
        builder = builder.header(HTMX.0, HTMX.1);
    }
    let body = Body::from(fill(body));
    call(&fixture.router(), builder.body(body).unwrap()).await
}

#[test]
fn plant_security_the_command_table_names_every_mutating_route_of_the_areas() {
    let listed: BTreeSet<&str> = COMMANDS.iter().map(|(path, _body)| *path).collect();
    assert_eq!(listed, mutating_templates());
}

#[tokio::test]
async fn plant_security_an_htmx_post_without_a_token_gives_403_and_writes_nothing() {
    let (fixture, session) = administrator();
    fixture.seed_plant();
    let before = fixture.snapshot();
    for target in COMMANDS {
        let what = format!("POST {} {}", target.0, target.1);
        let reply = post_without_token(&fixture, &session, target, "", true).await;
        assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-007", &what);
        assert_eq!(fixture.snapshot(), before, "{what}");
    }
}

#[tokio::test]
async fn plant_security_a_token_in_the_query_does_not_count() {
    let (fixture, session) = administrator();
    fixture.seed_plant();
    let before = fixture.snapshot();
    let query = format!("?csrf_token={}", session.csrf);
    for target in COMMANDS {
        let what = format!("POST {}{query} {}", target.0, target.1);
        let reply = post_without_token(&fixture, &session, target, &query, false).await;
        assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-007", &what);
        assert_eq!(fixture.snapshot(), before, "{what}");
    }
}

#[tokio::test]
async fn plant_security_read_only_mode_gives_503_and_writes_nothing() {
    let (fixture, session) = administrator();
    fixture.seed_plant();
    fixture.state.read_only().set(true);
    let before = fixture.snapshot();
    for (template, body) in COMMANDS {
        let path = fill(template);
        let body = fill(body);
        for headers in [Vec::new(), vec![HTMX]] {
            let what = format!("POST {path} {body} {headers:?}");
            let router = fixture.router();
            let reply = post_fields(&router, &path, Some(&session), &headers, &body).await;
            assert_error(&reply, StatusCode::SERVICE_UNAVAILABLE, "E-WEB-006", &what);
            if !headers.is_empty() {
                assert_eq!(reply.header("hx-retarget"), "#error-panel", "{what}");
            }
            assert_eq!(fixture.snapshot(), before, "{what}");
        }
    }
}

#[tokio::test]
async fn plant_security_read_only_mode_keeps_every_plant_get_page_available() {
    let (fixture, session) = administrator();
    fixture.seed_plant();
    fixture.state.read_only().set(true);
    let mut pages = machines::route_tests::sweep_pages();
    pages.extend(units::route_tests::sweep_pages());
    pages.extend(types::route_tests::sweep_pages());
    assert!(pages.len() > 10, "{pages:?}");
    for (_pattern, path) in pages {
        let cookie = [("cookie", session.cookie.as_str())];
        let reply = get_with(&fixture.router(), &path, &cookie).await;
        assert_eq!(reply.status, StatusCode::OK, "GET {path}: {}", reply.body);
    }
}
