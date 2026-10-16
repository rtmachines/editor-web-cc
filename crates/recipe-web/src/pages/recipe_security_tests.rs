//! Missing CSRF token and read-only mode on every `POST` route of the recipe pages (task 14.3, D14).
//!
//! The coverage test of `routes::tests` sends four defects with the body `a=b` to every mutating
//! row of the route table. This module sends the real body of each row of the three areas (design
//! D12). It adds the HTMX request, the token in the query, the security event, and read-only mode
//! with the buttons `fields` and `limit_mode`. The list of routes comes from `route_rows()` and
//! `sweep_pages()`, so a route that an area adds later joins the tests.

use std::collections::BTreeSet;

use axum::body::Body;
use axum::http::{Method, StatusCode};
use recipe_domain::audit::SecurityEventKind;

use super::recipe_sweep_tests::{post_endpoints, sweep_pages};
use super::test_helpers::assert_error;
use super::{parameters, recipes, versions};
use crate::csrf::is_mutating;
use crate::routes::matrix::{Endpoint, Principal, fill, world};
use crate::test_support::plant::{HTMX, TARGET_CONTENT};
use crate::test_support::{Fixture, Reply, TestSession, call, get_with, post_fields, request};

/// The button that asks a parameter route for the form again.
const FIELDS_BUTTON: &str = "fields=1";
/// The button that switches the limit mode. Its value is the leaf path of an array level.
const MODE_BUTTON: &str = "limit_mode=%5B*%5D";

/// Return the path templates of the mutating routes that the three areas register.
fn registered_templates() -> BTreeSet<&'static str> {
    let mut rows = recipes::rows();
    rows.extend(versions::rows());
    rows.extend(parameters::rows());
    rows.into_iter()
        .filter(|row| is_mutating(&row.method))
        .map(|row| row.path)
        .collect()
}

/// Return the Administrator of a fresh world with the data of `seed_recipes`.
async fn admin_world() -> (Fixture, TestSession) {
    let (fixture, session) = world(Principal::Administrator).await;
    (fixture, session.unwrap())
}

/// Send a same-origin `POST` with the session cookie and no token in the header or in the body.
///
/// `query` follows the path. The request has the header of an HTMX request when `htmx` is true.
async fn post_without_token(
    fixture: &Fixture,
    session: &TestSession,
    endpoint: &Endpoint,
    (query, htmx): (&str, bool),
) -> Reply {
    let uri = format!("{}{query}", fill(endpoint.path));
    let mut builder = request(Method::POST, &uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str());
    if htmx {
        builder = builder.header(HTMX.0, HTMX.1);
    }
    let body = Body::from(fill(endpoint.body));
    call(&fixture.router(), builder.body(body).unwrap()).await
}

/// Assert `403 E-WEB-007`, one new `csrf_rejected` event, and no change of the data.
async fn assert_rejected_without_token(
    world: (&Fixture, &TestSession),
    endpoint: &Endpoint,
    how: (&str, bool),
) {
    let (fixture, session) = world;
    let what = format!("POST {}{} htmx={}", endpoint.path, how.0, how.1);
    let (state, audit) = fixture.snapshot();
    let events = fixture.sink.events().len();
    let reply = post_without_token(fixture, session, endpoint, how).await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-007", &what);
    assert_eq!(fixture.snapshot(), (state, audit), "{what}");
    let after = fixture.sink.events();
    assert_eq!(after.len(), events.checked_add(1).unwrap(), "{what}");
    let last = after.last().map(|event| event.kind());
    assert_eq!(last, Some(SecurityEventKind::CsrfRejected), "{what}");
}

#[test]
fn recipe_security_the_endpoint_table_names_every_mutating_route_of_the_areas() {
    let listed: BTreeSet<&str> = post_endpoints()
        .iter()
        .map(|(endpoint, _want)| endpoint.path)
        .collect();
    assert!(listed.len() >= 10, "{listed:?}");
    assert_eq!(listed, registered_templates());
}

#[test]
fn recipe_security_the_endpoint_table_holds_a_body_for_every_command() {
    for (endpoint, _want) in post_endpoints() {
        assert!(is_mutating(&endpoint.method), "{}", endpoint.path);
        assert!(!endpoint.body.is_empty(), "no body for {}", endpoint.path);
    }
}

#[tokio::test]
async fn recipe_security_a_post_without_a_token_gives_403_and_writes_nothing() {
    let (fixture, session) = admin_world().await;
    for (endpoint, _want) in post_endpoints() {
        for htmx in [false, true] {
            let world = (&fixture, &session);
            assert_rejected_without_token(world, &endpoint, ("", htmx)).await;
        }
    }
}

#[tokio::test]
async fn recipe_security_a_token_in_the_query_does_not_count() {
    let (fixture, session) = admin_world().await;
    let query = format!("?csrf_token={}", session.csrf);
    for (endpoint, _want) in post_endpoints() {
        let world = (&fixture, &session);
        assert_rejected_without_token(world, &endpoint, (&query, false)).await;
    }
}

#[tokio::test]
async fn recipe_security_a_token_of_another_session_does_not_count() {
    let (fixture, session) = admin_world().await;
    let other = fixture.session_of(2);
    for (endpoint, _want) in post_endpoints() {
        let router = fixture.router();
        let (state, audit) = fixture.snapshot();
        let what = format!("POST {}", endpoint.path);
        let headers = [("x-csrf-token", other.csrf.as_str())];
        let body = fill(endpoint.body);
        let reply = post_fields_with_cookie(&router, (&session, &endpoint), &headers, &body).await;
        assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-007", &what);
        assert_eq!(fixture.snapshot(), (state, audit), "{what}");
    }
}

/// Send the body to the route of `endpoint` with the cookie of `session` and only the given headers.
async fn post_fields_with_cookie(
    router: &axum::Router,
    (session, endpoint): (&TestSession, &Endpoint),
    headers: &[(&str, &str)],
    body: &str,
) -> Reply {
    let mut builder = request(Method::POST, &fill(endpoint.path))
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str());
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    call(router, builder.body(Body::from(body.to_owned())).unwrap()).await
}

// --- Read-only mode ---------------------------------------------------------------------------------

/// Assert `503 E-WEB-006`, the retarget of an HTMX request, and no change of the data.
async fn assert_read_only(fixture: &Fixture, session: &TestSession, post: (&str, &str, bool)) {
    let (path, body, htmx) = post;
    let what = format!("POST {path} {body} htmx={htmx}");
    let (state, audit) = fixture.snapshot();
    let headers: Vec<(&str, &str)> = if htmx { vec![HTMX] } else { Vec::new() };
    let router = fixture.router();
    let reply = post_fields(&router, path, Some(session), &headers, body).await;
    assert_error(&reply, StatusCode::SERVICE_UNAVAILABLE, "E-WEB-006", &what);
    if htmx {
        assert_eq!(reply.header("hx-retarget"), "#error-panel", "{what}");
    }
    assert_eq!(fixture.snapshot(), (state, audit), "{what}");
}

/// Return true when the route is a parameter form that answers the buttons `fields` and `limit_mode`.
fn takes_buttons(endpoint: &Endpoint) -> bool {
    let path = endpoint.path;
    path.ends_with("/parameters") || path.ends_with("/definition")
}

#[tokio::test]
async fn recipe_security_read_only_mode_gives_503_and_writes_nothing() {
    let (fixture, session) = admin_world().await;
    fixture.state.read_only().set(true);
    for (endpoint, _want) in post_endpoints() {
        let (path, body) = (fill(endpoint.path), fill(endpoint.body));
        for htmx in [false, true] {
            assert_read_only(&fixture, &session, (&path, &body, htmx)).await;
        }
    }
}

#[tokio::test]
async fn recipe_security_read_only_mode_refuses_the_form_buttons_of_the_parameter_routes() {
    let (fixture, session) = admin_world().await;
    fixture.state.read_only().set(true);
    let mut seen = 0_usize;
    for (endpoint, _want) in post_endpoints().iter().filter(|(e, _w)| takes_buttons(e)) {
        seen = seen.checked_add(1).unwrap();
        let path = fill(endpoint.path);
        for button in [FIELDS_BUTTON, MODE_BUTTON] {
            let body = format!("{}&{button}", fill(endpoint.body));
            assert_read_only(&fixture, &session, (&path, &body, true)).await;
        }
    }
    assert_eq!(seen, 2, "the add route and the definition route");
}

#[tokio::test]
async fn recipe_security_read_only_mode_keeps_every_get_page_available() {
    let (fixture, session) = admin_world().await;
    fixture.state.read_only().set(true);
    let pages = sweep_pages();
    assert!(pages.len() >= 10, "{pages:?}");
    for (_pattern, path) in pages {
        let cookie = [("cookie", session.cookie.as_str())];
        let page = get_with(&fixture.router(), &path, &cookie).await;
        assert_eq!(page.status, StatusCode::OK, "GET {path}: {}", page.body);
        let headers = [cookie[0], HTMX, TARGET_CONTENT];
        let fragment = get_with(&fixture.router(), &path, &headers).await;
        assert_eq!(fragment.status, StatusCode::OK, "GET {path} content");
    }
}
