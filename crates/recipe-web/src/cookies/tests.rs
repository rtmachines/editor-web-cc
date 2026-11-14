//! Tests of the cookie names and attributes for both values of `session.secure` (ADR-0171).

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use super::{csrf_cookie_name, session_cookie_name};
use crate::pages::test_helpers::{field_value, set_cookie_of};
use crate::settings::WebSettings;
use crate::test_support::{Fixture, cookie_pair, get_with, post_fields, post_pre_session};

const PASSWORD: &str = "correct horse battery";
const PASSWORD_FIELD: &str = "correct+horse+battery";
const CSRF_KEY: &str = crate::test_support::CSRF_KEY_NEW;

fn fixture(secure: bool) -> Fixture {
    let settings = WebSettings {
        secure,
        ..WebSettings::default()
    };
    let fixture = Fixture::with_settings(settings, CSRF_KEY);
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    fixture
}

fn has_attribute(set_cookie: &str, attribute: &str) -> bool {
    set_cookie.split("; ").skip(1).any(|part| part == attribute)
}

/// Log in, read `/me` with the new cookie, and end a seeded session with `POST /logout`.
async fn session_round_trip(secure: bool, name: &str) {
    let fixture = fixture(secure);
    assert_eq!(session_cookie_name(secure), name);
    let login = post_pre_session(
        &fixture,
        "/login",
        &format!("login=user1&password={PASSWORD_FIELD}"),
        &[],
    )
    .await;
    assert_eq!(login.status, StatusCode::SEE_OTHER, "{}", login.body);
    let cookie = set_cookie_of(&login, name).expect("the session cookie has the expected name");
    assert_eq!(has_attribute(&cookie, "Secure"), secure, "{cookie}");
    assert!(has_attribute(&cookie, "HttpOnly"), "{cookie}");
    assert!(has_attribute(&cookie, "SameSite=Strict"), "{cookie}");
    assert!(has_attribute(&cookie, "Path=/"), "{cookie}");
    let me = get_with(
        &fixture.router(),
        "/me",
        &[("cookie", &cookie_pair(&cookie))],
    )
    .await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.body);
    let seeded = fixture.session_of(1);
    assert!(seeded.cookie.starts_with(&format!("{name}=")));
    let logout = post_fields(&fixture.router(), "/logout", Some(&seeded), &[], "").await;
    assert_eq!(logout.status, StatusCode::SEE_OTHER, "{}", logout.body);
    let cleared = set_cookie_of(&logout, name).expect("the logout clears the cookie");
    assert!(cleared.starts_with(&format!("{name}=;")), "{cleared}");
    assert!(cleared.contains("Max-Age=0"), "{cleared}");
    assert_eq!(has_attribute(&cleared, "Secure"), secure, "{cleared}");
}

/// `GET /login`, then `POST /login` with the pre-session cookie and its field.
async fn pre_session_round_trip(secure: bool, name: &str) {
    let fixture = fixture(secure);
    assert_eq!(csrf_cookie_name(secure), name);
    let router = fixture.router();
    let form = get_with(&router, "/login", &[]).await;
    let cookie = set_cookie_of(&form, name).expect("the pre-session cookie has the expected name");
    assert_eq!(has_attribute(&cookie, "Secure"), secure, "{cookie}");
    assert!(has_attribute(&cookie, "HttpOnly"), "{cookie}");
    assert!(has_attribute(&cookie, "SameSite=Strict"), "{cookie}");
    let nonce = field_value(&form.body, "csrf_pre");
    let body = format!("login=user1&password={PASSWORD_FIELD}&csrf_pre={nonce}");
    let pair = cookie_pair(&cookie);
    let reply = post_fields(&router, "/login", None, &[("cookie", &pair)], &body).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
}

#[tokio::test]
async fn login_then_get_with_recipe_session_without_secure() {
    session_round_trip(false, "recipe_session").await;
}

#[tokio::test]
async fn login_then_get_with_host_recipe_session_with_secure() {
    session_round_trip(true, "__Host-recipe_session").await;
}

#[tokio::test]
async fn get_login_then_post_with_recipe_csrf_without_secure() {
    pre_session_round_trip(false, "recipe_csrf").await;
}

#[tokio::test]
async fn get_login_then_post_with_host_recipe_csrf_with_secure() {
    pre_session_round_trip(true, "__Host-recipe_csrf").await;
}

#[test]
fn default_web_settings_keep_the_secure_cookies() {
    assert!(WebSettings::default().secure);
}

#[tokio::test]
async fn lang_cookie_drops_secure_only_without_secure() {
    for secure in [false, true] {
        let fixture = fixture(secure);
        let pre = crate::issue_pre_session(fixture.state.csrf_keys(), secure).unwrap();
        let pair = cookie_pair(pre.set_cookie().to_str().unwrap());
        let body = format!("lang=en&next=%2Flogin&csrf_pre={}", pre.nonce());
        let reply = post_fields(
            &fixture.router(),
            "/lang",
            None,
            &[("cookie", &pair)],
            &body,
        )
        .await;
        let cookie = set_cookie_of(&reply, "recipe_lang").expect("the language cookie");
        assert_eq!(has_attribute(&cookie, "Secure"), secure, "{cookie}");
        assert!(has_attribute(&cookie, "SameSite=Strict"), "{cookie}");
    }
}

#[test]
fn clear_session_cookie_follows_the_value_of_secure() {
    let secure = super::clear_session_cookie(true);
    let plain = super::clear_session_cookie(false);
    assert!(
        secure
            .to_str()
            .unwrap()
            .starts_with("__Host-recipe_session=;")
    );
    assert!(secure.to_str().unwrap().contains("Secure"));
    assert!(plain.to_str().unwrap().starts_with("recipe_session=;"));
    assert!(!plain.to_str().unwrap().contains("Secure"));
}
