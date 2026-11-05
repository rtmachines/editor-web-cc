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

/// Return the text of the `input` tag whose id is `id`, up to its closing `>`.
fn input_tag<'a>(body: &'a str, id: &str) -> &'a str {
    let marker = format!("<input id=\"{id}\"");
    let start = body.find(&marker).expect("the input of the field");
    let rest = &body[start..];
    let end = rest.find('>').expect("the end of the input tag");
    &rest[..end]
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
async fn pages_own_login_form_autofocuses_the_login_input_only() {
    let fixture = Fixture::new();
    let reply = get_with(&fixture.router(), "/login", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        input_tag(&reply.body, "login").contains(" autofocus"),
        "{}",
        reply.body
    );
    assert!(!input_tag(&reply.body, "password").contains("autofocus"));
    assert_eq!(reply.body.matches("autofocus").count(), 1);
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
async fn pages_own_failed_login_form_has_no_autofocus() {
    let fixture = Fixture::new();
    let reply = login(&fixture, "nobody", WRONG_FIELD).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
    assert!(reply.body.contains("name=\"login\""), "the form again");
    assert!(!reply.body.contains("autofocus"), "{}", reply.body);
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

/// Send `POST /logout` with the content type and body. The flag adds the token header.
async fn post_logout_body(
    fixture: &Fixture,
    session: &TestSession,
    content_type: &str,
    token_header: bool,
    body: String,
) -> Reply {
    let mut builder = crate::test_support::request(axum::http::Method::POST, "/logout")
        .header("content-type", content_type)
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str());
    if token_header {
        builder = builder.header("x-csrf-token", session.csrf.as_str());
    }
    let request = builder.body(axum::body::Body::from(body)).unwrap();
    crate::test_support::call(&fixture.router(), request).await
}

#[tokio::test]
async fn pages_own_logout_with_a_json_body_gives_422_and_keeps_the_session() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let reply = post_logout_body(
        &fixture,
        &session,
        "application/json",
        true,
        "{}".to_owned(),
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-008"), "{}", reply.body);
    assert!(set_cookie_of(&reply, SESSION_COOKIE).is_none());
    let after = me_with(&fixture, &session.cookie).await;
    assert_eq!(after.status, StatusCode::OK, "{}", after.body);
}

#[tokio::test]
async fn pages_own_logout_form_with_the_token_field_ends_the_session() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let body = format!("csrf_token={}", session.csrf);
    let reply = post_logout_body(
        &fixture,
        &session,
        "application/x-www-form-urlencoded",
        false,
        body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/login");
    let after = me_with(&fixture, &session.cookie).await;
    assert_eq!(after.status, StatusCode::SEE_OTHER);
    assert_eq!(after.header("location"), "/login");
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

// --- field answer of the own password form (ADR-0132) -------------------------------------------

/// Send an HTMX `POST /me/password` in English with the given fields.
async fn change_in_english(fixture: &Fixture, session: &TestSession, body: &str) -> Reply {
    post_fields(
        &fixture.router(),
        "/me/password",
        Some(session),
        &[HTMX, ENGLISH],
        body,
    )
    .await
}

/// Return the block of the error lines under the input `id`, or an empty text when none exists.
fn error_block<'a>(body: &'a str, id: &str) -> &'a str {
    let marker = format!("<div class=\"field-error\" id=\"{id}-error\">");
    body.find(&marker)
        .and_then(|start| {
            let rest = &body[start..];
            rest.find("</div>").map(|end| &rest[..end])
        })
        .unwrap_or("")
}

/// Assert that the answer holds none of the markers and that both password inputs are empty.
fn assert_no_password_echo(body: &str, markers: &[&str]) {
    for marker in markers {
        assert!(!body.contains(marker), "{marker} in the answer: {body}");
    }
    for id in ["current_password", "password"] {
        let tag = input_tag(body, id);
        assert!(!tag.contains("value="), "{id} is not empty: {tag}");
    }
}

/// Read the attempts of the pair (`user1`, peer) with a probe and return the failures before it.
async fn pair_failures(fixture: &Fixture) -> u32 {
    let key = AttemptKey {
        login: TypedLogin::parse("user1").unwrap(),
        client_ip: PEER.ip(),
    };
    let outcome = fixture
        .store
        .begin_attempt(&key, &ThrottlePolicy::default())
        .await
        .unwrap();
    outcome.count.pair_failures
}

#[tokio::test]
async fn pages_own_two_bad_password_fields_give_one_line_each() {
    const CURRENT_MARKER: &str = "mk1-";
    const NEW_MARKER: &str = "Zq7";
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let current = CURRENT_MARKER.repeat(33);
    let reply = change_in_english(&fixture, &session, &change_body(&current, NEW_MARKER)).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let body = &reply.body;
    assert!(body.contains("hx-swap-oob=\"true\""), "{body}");
    let on_current = error_block(body, "current_password");
    assert!(on_current.contains("E-WEB-008"), "{body}");
    assert_eq!(on_current.matches("<p>").count(), 1, "{on_current}");
    let on_new = error_block(body, "password");
    assert!(on_new.contains("E-IDN-006"), "{body}");
    assert_eq!(on_new.matches("<p>").count(), 1, "{on_new}");
    assert!(input_tag(body, "current_password").contains("aria-invalid=\"true\""));
    assert!(input_tag(body, "password").contains("aria-invalid=\"true\""));
    assert_no_password_echo(body, &[CURRENT_MARKER, NEW_MARKER]);
    assert_eq!(pair_failures(&fixture).await, 0, "no attempt row");
}

#[tokio::test]
async fn pages_own_a_listed_new_password_gives_one_line_and_no_attempt_row() {
    const LISTED: &str = "aaaaaaaaaaaa";
    const CURRENT_MARKER: &str = "wrong-marker-two";
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let reply = change_in_english(&fixture, &session, &change_body(CURRENT_MARKER, LISTED)).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let body = &reply.body;
    let on_new = error_block(body, "password");
    assert!(on_new.contains("E-IDN-006"), "{body}");
    assert_eq!(on_new.matches("<p>").count(), 1, "{on_new}");
    assert_eq!(error_block(body, "current_password"), "", "{body}");
    assert!(!input_tag(body, "current_password").contains("aria-invalid"));
    assert_no_password_echo(body, &[CURRENT_MARKER, LISTED]);
    assert_eq!(pair_failures(&fixture).await, 0, "no attempt row");
}

#[tokio::test]
async fn pages_own_a_wrong_current_password_marks_its_input() {
    const CURRENT_MARKER: &str = "wrong-marker-three";
    const NEW_MARKER: &str = "fresh-marker-three-long";
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let body = change_body(CURRENT_MARKER, NEW_MARKER);
    let reply = change_in_english(&fixture, &session, &body).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let text = &reply.body;
    assert!(text.contains("E-IDN-008"), "{text}");
    assert!(text.contains("href=\"#current_password\""), "{text}");
    let on_current = error_block(text, "current_password");
    assert!(on_current.contains("E-IDN-008"), "{text}");
    assert!(input_tag(text, "current_password").contains("aria-invalid=\"true\""));
    assert!(!input_tag(text, "password").contains("aria-invalid"));
    assert_no_password_echo(text, &[CURRENT_MARKER, NEW_MARKER]);
    assert_eq!(
        pair_failures(&fixture).await,
        1,
        "the wrong password counts"
    );
    let plain = change(&fixture, &session, &body, false).await;
    assert_eq!(plain.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(plain.body.contains("<html"), "{}", plain.body);
    assert!(
        input_tag(&plain.body, "current_password").contains("aria-invalid=\"true\""),
        "{}",
        plain.body
    );
    assert_no_password_echo(&plain.body, &[CURRENT_MARKER, NEW_MARKER]);
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

// --- read-only mode (ADR-0113 (g)) --------------------------------------------------------------

const BLOCKED_NOTICE: &str = "Read-only mode blocks this form.";

#[tokio::test]
async fn pages_own_password_page_in_read_only_mode_shows_the_notice_and_the_logout_button() {
    for must_change in [false, true] {
        let fixture = Fixture::new();
        let session = fixture.seed_session(1, Role::Operator, must_change);
        fixture.state.read_only().set(true);
        let reply = get_with(
            &fixture.router(),
            "/me/password",
            &[("cookie", &session.cookie), ENGLISH],
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(reply.body.contains(BLOCKED_NOTICE), "{}", reply.body);
        assert!(!reply.body.contains("name=\"current_password\""));
        assert!(!reply.body.contains("action=\"/me/password\""));
        assert!(reply.body.contains("action=\"/logout\""), "{}", reply.body);
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn pages_own_profile_page_in_read_only_mode_has_no_link_to_the_password_form() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let normal = me_with(&fixture, &session.cookie).await;
    assert!(
        normal.body.contains("href=\"/me/password\""),
        "{}",
        normal.body
    );
    fixture.state.read_only().set(true);
    let reply = me_with(&fixture, &session.cookie).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        !reply.body.contains("href=\"/me/password\""),
        "{}",
        reply.body
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_own_login_form_stays_in_read_only_mode() {
    let fixture = Fixture::new();
    fixture.state.read_only().set(true);
    let reply = get_with(&fixture.router(), "/login", &[ENGLISH]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("name=\"login\""));
    assert!(reply.body.contains("name=\"password\""));
}
