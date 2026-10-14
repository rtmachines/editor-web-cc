//! Tests of the pages of the own account (task 6.7, `api.md` sections 2.1 and 2.8).

use axum::http::StatusCode;
use recipe_app::ports::{AttemptKey, LoginAttemptStore};
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::identity::{Role, ThrottlePolicy, TypedLogin};

use crate::pages::test_helpers::{assert_clean_html, field_value, set_cookie_of};
use crate::test_support::{
    Fixture, PEER, Reply, TestSession, capture_logs, cookie_pair, deactivate_user, get_with,
    post_fields, post_pre_session,
};

const PASSWORD: &str = "correct horse battery";
const PASSWORD_FIELD: &str = "correct+horse+battery";
const WRONG_FIELD: &str = "wrong+password+here";
const NEW_PASSWORD_FIELD: &str = "another+long+passphrase";
const SESSION_COOKIE: &str = "__Host-recipe_session";
const CSRF_COOKIE: &str = "__Host-recipe_csrf";
const HTMX: (&str, &str) = ("hx-request", "true");
const ENGLISH: (&str, &str) = ("cookie", "recipe_lang=en");

// --- Helpers ------------------------------------------------------------------------------------

/// Send `POST /login` with a valid pre-session token and the given fields.
async fn login(fixture: &Fixture, login: &str, password_field: &str) -> Reply {
    let fields = format!("login={login}&password={password_field}");
    post_pre_session(fixture, "/login", &fields, &[]).await
}

/// Return the attributes of a `Set-Cookie` text, sorted, without the pair.
fn attributes_of(set_cookie: &str) -> Vec<String> {
    let mut parts: Vec<String> = set_cookie.split("; ").skip(1).map(str::to_owned).collect();
    parts.sort_unstable();
    parts
}

/// Return the body of a reply with the request id and the pre-session nonce blanked.
fn normalized(reply: &Reply) -> String {
    let id = reply.header("x-request-id");
    let body = reply.body.replace(&id, "<id>");
    match body.split_once("name=\"csrf_pre\" value=\"") {
        Some(_) => body.replace(&field_value(&body, "csrf_pre"), "<nonce>"),
        None => body,
    }
}

/// Return the sorted names of the headers of a reply, without the ones that differ by design.
fn header_names(reply: &Reply) -> Vec<String> {
    let mut names: Vec<String> = reply.headers.keys().map(|name| name.to_string()).collect();
    names.sort_unstable();
    names.dedup();
    names
}

async fn me_with(fixture: &Fixture, cookie: &str) -> Reply {
    get_with(&fixture.router(), "/me", &[("cookie", cookie)]).await
}

fn session_cookie_pair(reply: &Reply) -> String {
    cookie_pair(&set_cookie_of(reply, SESSION_COOKIE).unwrap())
}

// --- GET /login ---------------------------------------------------------------------------------

#[tokio::test]
async fn pages_own_login_form_has_the_three_fields_and_sets_the_pre_session_cookie() {
    let fixture = Fixture::new();
    let reply = get_with(&fixture.router(), "/login", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.header("content-type").starts_with("text/html"));
    assert!(reply.body.contains("name=\"login\""), "{}", reply.body);
    assert!(reply.body.contains("name=\"password\""));
    assert!(reply.body.contains("type=\"password\""));
    let nonce = field_value(&reply.body, "csrf_pre");
    assert_eq!(nonce.len(), 32);
    let cookie = set_cookie_of(&reply, CSRF_COOKIE).unwrap();
    assert!(
        cookie.starts_with(&format!("{CSRF_COOKIE}={nonce}.")),
        "{cookie}"
    );
    assert!(reply.body.contains("hx-history=\"false\""));
    assert!(reply.body.contains("\"allowEval\":false"));
    assert!(!reply.body.contains("hx-headers"), "no session, no token");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_own_cookie_and_field_of_the_login_form_are_accepted_by_post_login() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let router = fixture.router();
    let form = get_with(&router, "/login", &[]).await;
    let cookie = cookie_pair(&set_cookie_of(&form, CSRF_COOKIE).unwrap());
    let nonce = field_value(&form.body, "csrf_pre");
    let body = format!("login=user1&password={PASSWORD_FIELD}&csrf_pre={nonce}");
    let reply = post_fields(&router, "/login", None, &[("cookie", &cookie)], &body).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
}

#[tokio::test]
async fn pages_own_login_form_is_polish_by_default_and_english_with_the_cookie() {
    let fixture = Fixture::new();
    let polish = get_with(&fixture.router(), "/login", &[]).await;
    assert!(
        polish.body.contains("<html lang=\"pl\">"),
        "{}",
        polish.body
    );
    assert!(
        polish
            .body
            .contains("<title>Logowanie · Edytor receptur</title>")
    );
    let english = get_with(&fixture.router(), "/login", &[ENGLISH]).await;
    assert!(english.body.contains("<html lang=\"en\">"));
    assert!(
        english
            .body
            .contains("<title>Log in · Recipe editor</title>")
    );
}

// --- POST /login --------------------------------------------------------------------------------

#[tokio::test]
async fn pages_own_login_redirects_to_recipes_and_sets_the_session_cookie() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let reply = login(&fixture, "user1", PASSWORD_FIELD).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes");
    let cookie = set_cookie_of(&reply, SESSION_COOKIE).unwrap();
    assert_eq!(
        attributes_of(&cookie),
        ["HttpOnly", "Path=/", "SameSite=Strict", "Secure"],
        "{cookie}"
    );
    let value = cookie_pair(&cookie).split_once('=').unwrap().1.to_owned();
    assert_eq!(value.len(), 32, "{cookie}");
    assert!(
        value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    let me = me_with(&fixture, &cookie_pair(&cookie)).await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.body);
}

#[tokio::test]
async fn pages_own_login_with_a_one_time_password_goes_to_the_password_page() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, true);
    let reply = login(&fixture, "user1", PASSWORD_FIELD).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/me/password");
    assert!(set_cookie_of(&reply, SESSION_COOKIE).is_some());
}

#[tokio::test]
async fn pages_own_login_deletes_the_session_of_the_request_cookie() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let old = fixture.session_of(1);
    let fields = format!("login=user1&password={PASSWORD_FIELD}");
    let reply = post_pre_session(&fixture, "/login", &fields, &[("cookie", &old.cookie)]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    let with_old = me_with(&fixture, &old.cookie).await;
    assert_eq!(with_old.status, StatusCode::SEE_OTHER);
    assert_eq!(with_old.header("location"), "/login");
}

#[tokio::test]
async fn pages_own_wrong_password_unknown_login_and_inactive_user_give_one_answer() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    fixture.seed_account(2, Role::Operator, PASSWORD, false);
    deactivate_user(&fixture, "user2").await;
    let wrong = login(&fixture, "user1", WRONG_FIELD).await;
    let unknown = login(&fixture, "nobody", WRONG_FIELD).await;
    let inactive = login(&fixture, "user2", PASSWORD_FIELD).await;
    for reply in [&wrong, &unknown, &inactive] {
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
        assert!(reply.body.contains("E-IDN-003"), "{}", reply.body);
        assert!(!reply.body.contains("E-IDN-004"));
        assert!(reply.body.contains("name=\"login\""), "the form again");
        assert!(set_cookie_of(reply, SESSION_COOKIE).is_none());
        assert!(set_cookie_of(reply, CSRF_COOKIE).is_some(), "a fresh token");
        assert_clean_html(&reply.body);
    }
    assert_eq!(normalized(&wrong), normalized(&unknown));
    assert_eq!(normalized(&wrong), normalized(&inactive));
    assert_eq!(header_names(&wrong), header_names(&unknown));
    assert_eq!(header_names(&wrong), header_names(&inactive));
    let kinds: Vec<_> = fixture.sink.events().iter().map(|e| e.kind()).collect();
    assert_eq!(
        kinds,
        [
            SecurityEventKind::LoginFailed,
            SecurityEventKind::LoginFailed,
            SecurityEventKind::LoginRejectedInactive
        ]
    );
}

#[tokio::test]
async fn pages_own_a_fresh_pre_session_token_comes_with_each_failure() {
    let fixture = Fixture::new();
    let first = login(&fixture, "nobody", WRONG_FIELD).await;
    let second = login(&fixture, "nobody", WRONG_FIELD).await;
    assert_ne!(
        field_value(&first.body, "csrf_pre"),
        field_value(&second.body, "csrf_pre")
    );
}

#[tokio::test]
async fn pages_own_repeated_failures_give_429_with_retry_after() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let mut throttled = None;
    for _attempt in 0..6 {
        let reply = login(&fixture, "user1", WRONG_FIELD).await;
        if reply.status == StatusCode::TOO_MANY_REQUESTS {
            throttled = Some(reply);
            break;
        }
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    }
    let reply = throttled.expect("the login delay starts after three failures");
    assert!(reply.body.contains("E-IDN-005"), "{}", reply.body);
    assert!(reply.body.contains("name=\"login\""), "the form again");
    let seconds: u64 = reply.header("retry-after").parse().unwrap();
    assert!(seconds >= 1);
}

#[tokio::test]
async fn pages_own_bad_login_fields_give_422_and_call_no_use_case() {
    let fixture = Fixture::new();
    let long_login = "a".repeat(65);
    let long_password = "p".repeat(129);
    let cases = [
        ("login=&password=x".to_owned(), "login"),
        (format!("login={long_login}&password=x"), "login"),
        ("login=user1&password=".to_owned(), "password"),
        (format!("login=user1&password={long_password}"), "password"),
        ("login=user1&password=x&extra=1".to_owned(), "form"),
        ("password=x".to_owned(), "form"),
    ];
    for (fields, field) in cases {
        let reply = post_pre_session(&fixture, "/login", &fields, &[]).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{fields}");
        assert!(reply.body.contains("E-WEB-008"), "{}", reply.body);
        assert!(reply.body.contains(field), "{field}: {}", reply.body);
        assert!(reply.body.contains("name=\"login\""), "the form again");
        assert!(
            fixture.sink.events().is_empty(),
            "no use case ran: {fields}"
        );
    }
}

#[tokio::test]
async fn pages_own_a_login_with_a_nul_character_gives_422_and_counts_no_attempt() {
    let fixture = Fixture::new();
    let reply = login(&fixture, "admin%00", WRONG_FIELD).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.body.contains("E-WEB-008"), "{}", reply.body);
    assert!(reply.body.contains("name=\"login\""), "the form again");
    let key = AttemptKey {
        login: TypedLogin::parse("probe").unwrap(),
        client_ip: PEER.ip(),
    };
    let outcome = fixture
        .store
        .begin_attempt(&key, &ThrottlePolicy::default())
        .await
        .unwrap();
    assert_eq!(outcome.count.address_failures, 0, "the store holds no row");
}

#[tokio::test]
async fn pages_own_login_logs_hold_no_password_login_or_token() {
    let fixture = Fixture::new();
    let (logs, _guard) = capture_logs();
    let reply = login(&fixture, "nobody-secret", "hunter2+hunter2+hunter2").await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    let nonce = field_value(&reply.body, "csrf_pre");
    let cookie = set_cookie_of(&reply, CSRF_COOKIE).unwrap();
    let text = logs.text();
    for secret in ["hunter2", "nobody-secret", nonce.as_str(), cookie.as_str()] {
        assert!(!text.contains(secret), "{secret} in the log");
    }
    assert!(
        text.contains("E-IDN-004") || text.contains("E-IDN-003"),
        "{text}"
    );
}

// --- POST /logout -------------------------------------------------------------------------------

#[tokio::test]
async fn pages_own_logout_redirects_to_login_and_clears_the_cookie() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let reply = post_fields(&fixture.router(), "/logout", Some(&session), &[], "").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/login");
    let cookie = set_cookie_of(&reply, SESSION_COOKIE).unwrap();
    assert!(
        cookie.starts_with(&format!("{SESSION_COOKIE}=;")),
        "{cookie}"
    );
    assert!(cookie.contains("Max-Age=0"), "{cookie}");
    let after = me_with(&fixture, &session.cookie).await;
    assert_eq!(after.status, StatusCode::SEE_OTHER);
    assert_eq!(after.header("location"), "/login");
}

#[tokio::test]
async fn pages_own_logout_needs_the_csrf_token() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let headers = [
        ("cookie", session.cookie.as_str()),
        ("sec-fetch-site", "same-origin"),
    ];
    let request = crate::test_support::request(axum::http::Method::POST, "/logout")
        .header(headers[0].0, headers[0].1)
        .header(headers[1].0, headers[1].1)
        .body(axum::body::Body::empty())
        .unwrap();
    let reply = crate::test_support::call(&fixture.router(), request).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"));
}

// --- GET /me ------------------------------------------------------------------------------------

#[tokio::test]
async fn pages_own_profile_shows_the_login_and_the_role_with_the_token_in_hx_headers() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let reply = get_with(
        &fixture.router(),
        "/me",
        &[("cookie", &session.cookie), ENGLISH],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("user1"));
    assert!(reply.body.contains("Operator"), "{}", reply.body);
    let expected = format!("hx-headers='{{\"X-CSRF-Token\":\"{}\"}}'", session.csrf);
    assert!(reply.body.contains(&expected), "{}", reply.body);
    assert!(reply.body.contains("hx-history=\"false\""));
    assert!(reply.body.contains("action=\"/logout\""));
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_own_profile_without_a_session_goes_to_login() {
    let fixture = Fixture::new();
    let reply = get_with(&fixture.router(), "/me", &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), "/login");
}

// --- /me/password -------------------------------------------------------------------------------

#[tokio::test]
async fn pages_own_password_form_has_the_two_fields_and_the_sync_guard() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let reply = get_with(
        &fixture.router(),
        "/me/password",
        &[("cookie", &session.cookie)],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("name=\"current_password\""));
    assert!(reply.body.contains("name=\"password\""));
    assert!(reply.body.contains("hx-sync=\"closest form:drop\""));
    assert!(
        reply
            .body
            .contains(&format!("name=\"csrf_token\" value=\"{}\"", session.csrf))
    );
    assert!(reply.body.contains("autocomplete=\"new-password\""));
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_own_password_form_tells_a_user_with_a_one_time_password_about_it() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, true);
    let reply = get_with(
        &fixture.router(),
        "/me/password",
        &[("cookie", &session.cookie), ENGLISH],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("one-time password"), "{}", reply.body);
    let other = fixture.seed_session(2, Role::Operator, false);
    let plain = get_with(
        &fixture.router(),
        "/me/password",
        &[("cookie", &other.cookie), ENGLISH],
    )
    .await;
    assert!(!plain.body.contains("one-time password"));
}

fn change_body(current: &str, new: &str) -> String {
    format!("current_password={current}&password={new}")
}

async fn change(fixture: &Fixture, session: &TestSession, body: &str, htmx: bool) -> Reply {
    let headers: &[(&str, &str)] = if htmx { &[HTMX] } else { &[] };
    post_fields(
        &fixture.router(),
        "/me/password",
        Some(session),
        headers,
        body,
    )
    .await
}

#[tokio::test]
async fn pages_own_password_change_through_htmx_refreshes_with_a_new_session() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, true);
    let session = fixture.session_of(1);
    let body = change_body(PASSWORD_FIELD, NEW_PASSWORD_FIELD);
    let reply = change(&fixture, &session, &body, true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-refresh"), "true");
    let fresh = session_cookie_pair(&reply);
    assert_ne!(fresh, session.cookie);
    let old = me_with(&fixture, &session.cookie).await;
    assert_eq!(old.status, StatusCode::SEE_OTHER, "the old session ended");
    let now = me_with(&fixture, &fresh).await;
    assert_eq!(now.status, StatusCode::OK, "{}", now.body);
    let relogin = login(&fixture, "user1", NEW_PASSWORD_FIELD).await;
    assert_eq!(relogin.status, StatusCode::SEE_OTHER);
    assert_eq!(
        relogin.header("location"),
        "/recipes",
        "the flag is cleared"
    );
}

#[tokio::test]
async fn pages_own_password_change_without_htmx_redirects_to_recipes() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let body = change_body(PASSWORD_FIELD, NEW_PASSWORD_FIELD);
    let reply = change(&fixture, &session, &body, false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes");
    assert!(reply.headers.get("hx-refresh").is_none());
    let fresh = session_cookie_pair(&reply);
    assert_eq!(me_with(&fixture, &fresh).await.status, StatusCode::OK);
}

#[tokio::test]
async fn pages_own_a_weak_new_password_gives_422_e_idn_006() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let long = "p".repeat(129);
    for new in ["short".to_owned(), String::new(), long] {
        let reply = change(&fixture, &session, &change_body(PASSWORD_FIELD, &new), true).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.body
        );
        assert!(reply.body.contains("E-IDN-006"), "{}", reply.body);
        assert_eq!(reply.header("hx-retarget"), "#error-panel");
    }
}

#[tokio::test]
async fn pages_own_a_wrong_current_password_gives_422_e_idn_008() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let body = change_body(WRONG_FIELD, NEW_PASSWORD_FIELD);
    let reply = change(&fixture, &session, &body, true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-IDN-008"), "{}", reply.body);
    let plain = change(&fixture, &session, &body, false).await;
    assert_eq!(plain.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        plain.body.contains("<html"),
        "a full page without HX-Request"
    );
}

#[tokio::test]
async fn pages_own_repeated_wrong_current_passwords_give_429_e_idn_005() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let body = change_body(WRONG_FIELD, NEW_PASSWORD_FIELD);
    let mut throttled = None;
    for _attempt in 0..6 {
        let reply = change(&fixture, &session, &body, true).await;
        if reply.status == StatusCode::TOO_MANY_REQUESTS {
            throttled = Some(reply);
            break;
        }
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let reply = throttled.expect("the throttle of ChangeOwnPassword starts");
    assert!(reply.body.contains("E-IDN-005"), "{}", reply.body);
    assert!(reply.header("retry-after").parse::<u64>().unwrap() >= 1);
}

#[tokio::test]
async fn pages_own_a_password_request_without_a_session_gives_e_web_004() {
    let fixture = Fixture::new();
    let body = change_body(PASSWORD_FIELD, NEW_PASSWORD_FIELD);
    let router = fixture.router();
    let htmx = post_fields(&router, "/me/password", None, &[HTMX], &body).await;
    assert_eq!(htmx.status, StatusCode::UNAUTHORIZED, "{}", htmx.body);
    assert_eq!(htmx.header("hx-redirect"), "/login");
    let plain = post_fields(&router, "/me/password", None, &[], &body).await;
    assert_eq!(plain.status, StatusCode::SEE_OTHER);
    assert_eq!(plain.header("location"), "/login");
}

#[tokio::test]
async fn pages_own_an_unknown_password_form_field_gives_422_e_web_008() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let body = format!(
        "{}&extra=1",
        change_body(PASSWORD_FIELD, NEW_PASSWORD_FIELD)
    );
    let reply = change(&fixture, &session, &body, true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-008"));
    let empty = change(
        &fixture,
        &session,
        &change_body("", NEW_PASSWORD_FIELD),
        true,
    )
    .await;
    assert_eq!(empty.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(empty.body.contains("E-WEB-008"), "{}", empty.body);
}

#[tokio::test]
async fn pages_own_password_change_logs_hold_no_password() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let (logs, _guard) = capture_logs();
    let body = change_body(WRONG_FIELD, NEW_PASSWORD_FIELD);
    let _reply = change(&fixture, &session, &body, true).await;
    let text = logs.text();
    for secret in [
        "wrong password here",
        "another long passphrase",
        session.csrf.as_str(),
    ] {
        assert!(!text.contains(secret), "{secret} in the log");
    }
}
