//! Tests of the end of a session through the routes (task 6.11, design D10, `api.md` section 7).

use std::net::IpAddr;

use axum::http::StatusCode;
use recipe_app::ports::Clock as _;
use recipe_domain::identity::{Role, TypedLogin};
use recipe_domain::timestamp::Timestamp;
use time::Duration;

use crate::pages::test_helpers::set_cookie_of;
use crate::test_support::{
    Fixture, PEER, Reply, TestSession, get_with, post_fields, post_pre_session,
};

const PASSWORD: &str = "correct horse battery";
const PASSWORD_FIELD: &str = "correct+horse+battery";
const NEW_PASSWORD_FIELD: &str = "another+long+passphrase";
const SESSION_COOKIE: &str = "__Host-recipe_session";
const HTMX: (&str, &str) = ("hx-request", "true");

// --- Helpers ------------------------------------------------------------------------------------

/// A fixture with the Administrator `user1` and the Operator `user2` who has a session.
fn world() -> (Fixture, TestSession, TestSession) {
    let fixture = Fixture::new();
    let admin = fixture.seed_session(1, Role::Administrator, false);
    fixture.seed_account(2, Role::Operator, PASSWORD, false);
    let target = fixture.session_of(2);
    (fixture, admin, target)
}

async fn me(fixture: &Fixture, cookie: &str, htmx: bool) -> Reply {
    let mut headers = vec![("cookie", cookie)];
    if htmx {
        headers.push(HTMX);
    }
    get_with(&fixture.router(), "/me", &headers).await
}

/// Assert that the session behind `cookie` is over, for an HTMX request and for a page request.
async fn assert_session_is_over(fixture: &Fixture, cookie: &str) {
    let htmx = me(fixture, cookie, true).await;
    assert_eq!(htmx.status, StatusCode::UNAUTHORIZED, "{}", htmx.body);
    assert!(htmx.body.contains("E-WEB-004"), "{}", htmx.body);
    assert_eq!(htmx.header("hx-redirect"), "/login");
    assert_cookie_cleared(&htmx);
    let page = me(fixture, cookie, false).await;
    assert_eq!(page.status, StatusCode::SEE_OTHER, "{}", page.body);
    assert_eq!(page.header("location"), "/login");
    assert_cookie_cleared(&page);
}

fn assert_cookie_cleared(reply: &Reply) {
    let cookie = set_cookie_of(reply, SESSION_COOKIE).expect("the session cookie is cleared");
    assert!(
        cookie.starts_with(&format!("{SESSION_COOKIE}=;")),
        "{cookie}"
    );
    assert!(cookie.contains("Max-Age=0"), "{cookie}");
}

async fn as_admin(fixture: &Fixture, admin: &TestSession, path: &str, body: &str) -> Reply {
    post_fields(&fixture.router(), path, Some(admin), &[HTMX], body).await
}

// --- Administrator commands ---------------------------------------------------------------------

#[tokio::test]
async fn session_end_set_user_role_ends_the_session_of_the_target() {
    let (fixture, admin, target) = world();
    assert_eq!(
        me(&fixture, &target.cookie, true).await.status,
        StatusCode::OK
    );
    let reply = as_admin(
        &fixture,
        &admin,
        "/users/user2/role",
        "role=process_engineer",
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_session_is_over(&fixture, &target.cookie).await;
    assert_eq!(
        me(&fixture, &admin.cookie, true).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn session_end_deactivate_user_ends_the_session_of_the_target() {
    let (fixture, admin, target) = world();
    assert_eq!(
        me(&fixture, &target.cookie, true).await.status,
        StatusCode::OK
    );
    let reply = as_admin(&fixture, &admin, "/users/user2/deactivate", "").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_session_is_over(&fixture, &target.cookie).await;
}

#[tokio::test]
async fn session_end_reset_password_ends_the_session_of_the_target() {
    let (fixture, admin, target) = world();
    assert_eq!(
        me(&fixture, &target.cookie, true).await.status,
        StatusCode::OK
    );
    let reply = as_admin(&fixture, &admin, "/users/user2/reset-password", "").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_session_is_over(&fixture, &target.cookie).await;
}

// --- Own password change ------------------------------------------------------------------------

#[tokio::test]
async fn session_end_change_own_password_ends_the_old_cookie() {
    let (fixture, _admin, target) = world();
    let body = format!("current_password={PASSWORD_FIELD}&password={NEW_PASSWORD_FIELD}");
    let reply = post_fields(
        &fixture.router(),
        "/me/password",
        Some(&target),
        &[HTMX],
        &body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let fresh = set_cookie_of(&reply, SESSION_COOKIE).expect("a new session cookie");
    let fresh_pair = crate::test_support::cookie_pair(&fresh);
    assert_ne!(fresh_pair, target.cookie);
    assert_eq!(me(&fixture, &fresh_pair, true).await.status, StatusCode::OK);
    assert_session_is_over(&fixture, &target.cookie).await;
}

// --- Login throttle -----------------------------------------------------------------------------

#[tokio::test]
async fn session_end_throttled_login_gives_429_with_retry_after() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let now = fixture.store.clock().now();
    let login = TypedLogin::parse("user1").unwrap();
    let address: IpAddr = PEER.ip();
    for seconds_ago in [4_i64, 3, 2, 1] {
        let at = now
            .as_offset_date_time()
            .checked_sub(Duration::seconds(seconds_ago))
            .unwrap();
        fixture
            .store
            .seed_attempt(&login, address, Timestamp::from(at), false);
    }
    let fields = format!("login=user1&password={PASSWORD_FIELD}");
    let reply = post_pre_session(&fixture, "/login", &fields, &[]).await;
    assert_eq!(
        reply.status,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-IDN-005"), "{}", reply.body);
    let seconds: u64 = reply.header("retry-after").parse().unwrap();
    assert!(seconds >= 1, "{seconds}");
    assert!(set_cookie_of(&reply, SESSION_COOKIE).is_none());
}
