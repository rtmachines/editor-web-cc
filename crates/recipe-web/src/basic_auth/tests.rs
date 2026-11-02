//! Tests of the Basic layer and the extractor `BasicUser` (design D3, ADR-0109 (l), (mb)).
//!
//! The test route `/test/basic` has the access `Basic(ExportApprovedVersion)`. Its handler
//! reports the login, the session extension, and the session reference of the request data.

use axum::extract::FromRequestParts as _;
use axum::http::request::Parts;
use axum::http::{Request, StatusCode};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;

use super::BasicUser;
use crate::error::WebError;
use crate::test_support::{Fixture, Reply, capture_logs, get_with};

/// The route with the access `Basic`.
const ROUTE: &str = "/test/basic";
/// The password of the seeded account. It must never reach a log line.
const PASSWORD: &str = "tr4nsfer-s3cret";
/// A wrong password. It must never reach a log line either.
const WRONG_PASSWORD: &str = "wr0ng-s3cret";

fn authorization(login: &str, password: &str) -> String {
    format!("Basic {}", STANDARD.encode(format!("{login}:{password}")))
}

async fn send(fixture: &Fixture, headers: &[(&str, &str)]) -> Reply {
    get_with(&fixture.router(), ROUTE, headers).await
}

fn fixture_with_client() -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::ExportClient, PASSWORD, false);
    fixture
}

#[tokio::test]
async fn basic_route_gives_the_handler_the_account_without_session_or_csrf() {
    let fixture = fixture_with_client();
    let header = authorization("user1", PASSWORD);

    let reply = send(&fixture, &[("authorization", &header)]).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, "login=user1;session=none;session_ref=none");
}

#[tokio::test]
async fn basic_route_ignores_the_session_cookie() {
    let fixture = fixture_with_client();
    let administrator = fixture.seed_session(2, Role::Administrator, false);
    let header = authorization("user1", PASSWORD);

    let with_cookie = send(
        &fixture,
        &[
            ("authorization", &header),
            ("cookie", &administrator.cookie),
        ],
    )
    .await;
    let cookie_only = send(&fixture, &[("cookie", &administrator.cookie)]).await;

    assert_eq!(with_cookie.status, StatusCode::OK, "{}", with_cookie.body);
    assert_eq!(
        with_cookie.body,
        "login=user1;session=none;session_ref=none"
    );
    assert_eq!(cookie_only.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn basic_route_without_header_gives_e_web_012_and_runs_no_handler() {
    let fixture = fixture_with_client();

    let reply = send(&fixture, &[]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert!(reply.body.contains("E-WEB-012"), "{}", reply.body);
    assert!(!reply.body.contains("login="), "{}", reply.body);
}

#[tokio::test]
async fn basic_route_with_wrong_password_gives_e_idn_003() {
    let fixture = fixture_with_client();
    let header = authorization("user1", WRONG_PASSWORD);

    let reply = send(&fixture, &[("authorization", &header)]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert!(reply.body.contains("E-IDN-003"), "{}", reply.body);
}

#[tokio::test]
async fn basic_route_refuses_a_person_account() {
    let fixture = Fixture::new();
    fixture.seed_account(3, Role::Administrator, PASSWORD, false);
    let header = authorization("user3", PASSWORD);

    let reply = send(&fixture, &[("authorization", &header)]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert!(reply.body.contains("E-IDN-003"), "{}", reply.body);
}

#[tokio::test]
async fn basic_route_logs_neither_header_nor_password() {
    let (logs, _guard) = capture_logs();
    let fixture = fixture_with_client();
    let header = authorization("user1", PASSWORD);
    let wrong = authorization("user1", WRONG_PASSWORD);

    send(&fixture, &[("authorization", &header)]).await;
    send(&fixture, &[("authorization", &wrong)]).await;

    let text = logs.text();
    assert!(!text.contains(PASSWORD), "{text}");
    assert!(!text.contains(WRONG_PASSWORD), "{text}");
    assert!(
        !text.contains(header.trim_start_matches("Basic ")),
        "{text}"
    );
    assert!(!text.contains(wrong.trim_start_matches("Basic ")), "{text}");
}

#[tokio::test]
async fn basic_user_without_the_extension_is_a_server_fault() {
    let (mut parts, ()): (Parts, ()) = Request::builder().body(()).unwrap().into_parts();

    let error = BasicUser::from_request_parts(&mut parts, &())
        .await
        .expect_err("no BasicPrincipal in the extensions");

    assert!(matches!(error, WebError::Internal { .. }), "{error:?}");
    assert_eq!(error.code(), ErrorCode::WEB_010);
}
