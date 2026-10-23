//! Tests of the navigation guard on the two export routes (design D11 row T13, ADR-0055).
//!
//! The data comes from `seed_approval_recipes` (design D14): `R-500` has the approved version 1,
//! and `R-600` has the retired version 1. The requests use the session of the Administrator
//! `admin`, who can export both versions.

use axum::body::Body;
use axum::http::{HeaderValue, Method, StatusCode};
use recipe_app::memory::Fault;
use recipe_domain::audit::{AuditAction, SecurityEventKind};
use recipe_domain::identity::Role;

use crate::test_support::{Fixture, Reply, TestSession, call, get_with, request};

/// The two export routes: the approved version of `R-500` and the retired version 1 of `R-600`.
const ROUTES: [&str; 2] = [
    "/export/v1/recipes/R-500/approved",
    "/export/v1/recipes/R-600/versions/1",
];
/// The name of the header `Sec-Fetch-Site`.
const SITE: &str = "sec-fetch-site";
/// The header of an HTMX request.
const HTMX: (&str, &str) = ("hx-request", "true");
/// The values of `Sec-Fetch-Site` that the guard rejects (ADR-0055 (b)).
const REJECTED_SITES: [&str; 4] = ["cross-site", "same-site", "Same-Origin", ""];

/// The number of `VersionExported` entries and security events at one moment.
#[derive(Debug, PartialEq, Eq)]
struct Counts {
    exported: usize,
    events: usize,
}

impl Counts {
    fn of(fixture: &Fixture) -> Self {
        let exported = fixture
            .store
            .audit_entries()
            .iter()
            .filter(|entry| entry.action() == AuditAction::VersionExported)
            .count();
        Self {
            exported,
            events: fixture.sink.events().len(),
        }
    }
}

/// Send `GET path` with the cookie of `session`, when there is one, and the headers.
async fn send(
    fixture: &Fixture,
    session: Option<&TestSession>,
    path: &str,
    headers: &[(&str, &str)],
) -> Reply {
    let mut all: Vec<(&str, &str)> = session
        .map(|session| ("cookie", session.cookie.as_str()))
        .into_iter()
        .collect();
    all.extend_from_slice(headers);
    get_with(&fixture.router(), path, &all).await
}

/// Check the answer `403 E-WEB-007` of ADR-0053 (a), one new `csrf_rejected` event, and no entry.
fn assert_rejected(fixture: &Fixture, before: &Counts, reply: &Reply, case: &str) {
    assert_eq!(
        reply.status,
        StatusCode::FORBIDDEN,
        "{case}: {}",
        reply.body
    );
    assert_eq!(reply.header("content-type"), "application/json", "{case}");
    assert!(
        reply
            .headers
            .keys()
            .all(|name| !name.as_str().starts_with("hx-")),
        "{case}: {:?}",
        reply.headers
    );
    let request_id = reply.header("x-request-id");
    let ending = format!("\",\"request_id\":\"{request_id}\"}}");
    assert!(
        reply
            .body
            .starts_with("{\"code\":\"E-WEB-007\",\"message\":\""),
        "{case}: {}",
        reply.body
    );
    assert!(reply.body.ends_with(&ending), "{case}: {}", reply.body);
    let events = fixture.sink.events();
    assert_eq!(
        events.len(),
        before.events.checked_add(1).unwrap(),
        "{case}"
    );
    let event = events.last().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::CsrfRejected, "{case}");
    assert_eq!(event.request_id().to_string(), request_id, "{case}");
    assert_eq!(Counts::of(fixture).exported, before.exported, "{case}");
}

/// Check a file answer, one new `VersionExported` entry, and no new security event.
fn assert_passed(fixture: &Fixture, before: &Counts, reply: &Reply, case: &str) {
    assert_eq!(reply.status, StatusCode::OK, "{case}: {}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json", "{case}");
    assert!(
        reply
            .header("content-disposition")
            .starts_with("attachment; filename="),
        "{case}"
    );
    assert!(
        reply.body.starts_with("{\n  \"schema_version\": \"1\",\n"),
        "{case}"
    );
    let expected = Counts {
        exported: before.exported.checked_add(1).unwrap(),
        events: before.events,
    };
    assert_eq!(Counts::of(fixture), expected, "{case}");
}

#[tokio::test]
async fn export_guard_rejects_a_foreign_sec_fetch_site_on_both_routes() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    for route in ROUTES {
        for site in REJECTED_SITES {
            let before = Counts::of(&fixture);
            let reply = send(&fixture, Some(&data.admin), route, &[(SITE, site)]).await;
            assert_rejected(&fixture, &before, &reply, &format!("{route} {site:?}"));
        }
    }
}

#[tokio::test]
async fn export_guard_rejects_a_sec_fetch_site_that_is_not_text() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let value = HeaderValue::from_bytes(b"same-origin\xff").unwrap();
    for route in ROUTES {
        let before = Counts::of(&fixture);
        let built = request(Method::GET, route)
            .header("cookie", data.admin.cookie.as_str())
            .header(SITE, value.clone())
            .body(Body::empty())
            .unwrap();
        let reply = call(&fixture.router(), built).await;
        assert_rejected(&fixture, &before, &reply, route);
    }
}

#[tokio::test]
async fn export_guard_rejects_an_htmx_request_on_both_routes() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let cases = [
        vec![HTMX],
        vec![HTMX, (SITE, "same-origin")],
        vec![HTMX, (SITE, "none")],
    ];
    for route in ROUTES {
        for headers in &cases {
            let before = Counts::of(&fixture);
            let reply = send(&fixture, Some(&data.admin), route, headers).await;
            assert_rejected(&fixture, &before, &reply, &format!("{route} {headers:?}"));
        }
    }
}

#[tokio::test]
async fn export_guard_passes_none_same_origin_and_an_absent_header_on_both_routes() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let cases = [vec![(SITE, "none")], vec![(SITE, "same-origin")], vec![]];
    for route in ROUTES {
        for headers in &cases {
            let before = Counts::of(&fixture);
            let reply = send(&fixture, Some(&data.admin), route, headers).await;
            assert_passed(&fixture, &before, &reply, &format!("{route} {headers:?}"));
        }
    }
}

#[tokio::test]
async fn export_guard_runs_after_the_session_layer_so_no_session_gets_the_login_redirect() {
    let fixture = Fixture::new();
    fixture.seed_approval_recipes().await;
    for route in ROUTES {
        let before = Counts::of(&fixture);
        let reply = send(&fixture, None, route, &[(SITE, "cross-site")]).await;
        assert_eq!(
            reply.status,
            StatusCode::SEE_OTHER,
            "{route}: {}",
            reply.body
        );
        assert_eq!(reply.header("location"), "/login", "{route}");
        assert_eq!(Counts::of(&fixture), before, "{route}");
    }
}

#[tokio::test]
async fn export_guard_runs_after_the_password_gate_so_a_forced_change_gets_its_redirect() {
    let fixture = Fixture::new();
    fixture.seed_approval_recipes().await;
    let session = fixture.seed_session(1, Role::Administrator, true);
    for route in ROUTES {
        let before = Counts::of(&fixture);
        let reply = send(&fixture, Some(&session), route, &[(SITE, "cross-site")]).await;
        assert_eq!(
            reply.status,
            StatusCode::SEE_OTHER,
            "{route}: {}",
            reply.body
        );
        assert_eq!(reply.header("location"), "/me/password", "{route}");
        assert_eq!(Counts::of(&fixture), before, "{route}");
    }
}

#[tokio::test]
async fn export_guard_failed_event_write_gives_the_storage_error_as_json_and_no_entry() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    fixture.sink.fail_with(Fault::Unavailable);
    for route in ROUTES {
        let before = Counts::of(&fixture);
        let reply = send(&fixture, Some(&data.admin), route, &[(SITE, "cross-site")]).await;
        assert_eq!(
            reply.status,
            StatusCode::SERVICE_UNAVAILABLE,
            "{route}: {}",
            reply.body
        );
        assert_eq!(reply.header("content-type"), "application/json", "{route}");
        assert!(
            reply
                .body
                .starts_with("{\"code\":\"E-DB-005\",\"message\":\""),
            "{route}: {}",
            reply.body
        );
        assert_eq!(Counts::of(&fixture), before, "{route}");
    }
}
