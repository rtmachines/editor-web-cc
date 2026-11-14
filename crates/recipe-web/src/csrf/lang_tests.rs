//! Tests of the CSRF check of `POST /lang` (task 6.6, design D12, ADR-0035 e, s, t, ADR-0131).

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use super::issue_pre_session;
use super::tests::{SAME_ORIGIN, assert_rejected, post, session_headers};
use crate::test_support::{Fixture, cookie_pair};

#[tokio::test]
async fn csrf_lang_accepts_the_pre_session_token_without_a_session() {
    let fixture = Fixture::new();
    let reply = crate::test_support::post_pre_session(&fixture, "/lang", "lang=en", &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn csrf_lang_accepts_the_session_token_of_the_open_session() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let router = fixture.router();
    let with_header = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &session.csrf)]);
    let reply = post(&router, "/lang", &with_header, "lang=en").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    let with_field = session_headers(&session, &[SAME_ORIGIN]);
    let body = format!("lang=pl&csrf_token={}", session.csrf);
    let reply = post(&router, "/lang", &with_field, &body).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn csrf_lang_with_a_session_also_accepts_the_pre_session_token() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let token =
        issue_pre_session(fixture.state.csrf_keys(), fixture.state.settings().secure).unwrap();
    let cookies = format!(
        "{}; {}",
        session.cookie,
        cookie_pair(token.set_cookie().to_str().unwrap())
    );
    let body = format!("lang=en&csrf_pre={}", token.nonce());
    let reply = post(
        &fixture.router(),
        "/lang",
        &[SAME_ORIGIN, ("cookie", &cookies)],
        &body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
}

/// Scenario "Language switch with a session and the pre-session token" (ADR-0131).
#[tokio::test]
async fn csrf_lang_with_a_session_and_csrf_pre_only_sets_the_language_cookie() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let token =
        issue_pre_session(fixture.state.csrf_keys(), fixture.state.settings().secure).unwrap();
    let cookies = format!(
        "{}; {}",
        session.cookie,
        cookie_pair(token.set_cookie().to_str().unwrap())
    );
    // The body has `csrf_pre` and no `csrf_token`, and the request has no `X-CSRF-Token`.
    let body = format!("lang=en&csrf_pre={}", token.nonce());
    let reply = post(
        &fixture.router(),
        "/lang",
        &[SAME_ORIGIN, ("cookie", &cookies)],
        &body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert!(
        reply.header("set-cookie").starts_with("recipe_lang=en"),
        "{}",
        reply.header("set-cookie")
    );
    assert!(fixture.sink.events().is_empty());
}

/// Scenario "Language switch with a session and a wrong token" (ADR-0131).
#[tokio::test]
async fn csrf_lang_with_a_session_and_a_wrong_csrf_token_gives_e_web_007() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let wrong = "cd".repeat(32);
    assert_ne!(wrong, session.csrf);
    let headers = session_headers(&session, &[SAME_ORIGIN]);
    let body = format!("lang=en&csrf_token={wrong}");
    let reply = post(&fixture.router(), "/lang", &headers, &body).await;
    assert_rejected(&fixture, &reply, "wrong session token");
    assert_eq!(reply.header("set-cookie"), "");
}

#[tokio::test]
async fn csrf_lang_without_a_token_or_with_a_foreign_token_is_rejected() {
    let fixture = Fixture::new();
    let reply = post(&fixture.router(), "/lang", &[SAME_ORIGIN], "lang=en").await;
    assert_rejected(&fixture, &reply, "no token");
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let other = fixture.seed_session(2, Role::Operator, false);
    let headers = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &other.csrf)]);
    let reply = post(&fixture.router(), "/lang", &headers, "lang=en").await;
    assert_rejected(&fixture, &reply, "token of another session");
}

#[tokio::test]
async fn csrf_lang_session_token_without_a_session_is_rejected() {
    let fixture = Fixture::new();
    let reply = post(
        &fixture.router(),
        "/lang",
        &[SAME_ORIGIN, ("x-csrf-token", &"ab".repeat(32))],
        &format!("lang=en&csrf_token={}", "ab".repeat(32)),
    )
    .await;
    assert_rejected(&fixture, &reply, "no session");
}
