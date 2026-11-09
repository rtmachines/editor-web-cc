//! Tests of the field answer of the own password form (task 5.3, ADR-0132).

use axum::http::StatusCode;
use recipe_app::ports::{AttemptKey, LoginAttemptStore};
use recipe_domain::identity::{Role, ThrottlePolicy, TypedLogin};

use super::tests::{
    ENGLISH, HTMX, NEW_PASSWORD_FIELD, PASSWORD, PASSWORD_FIELD, WRONG_FIELD, change, change_body,
    input_tag,
};
use crate::test_support::{Fixture, PEER, Reply, TestSession, capture_logs, get_with, post_fields};

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

/// Return the start tag of `#password-form` up to its `>`, or an empty text.
fn form_start_tag(body: &str) -> &str {
    body.find("<form id=\"password-form\"")
        .and_then(|from| {
            let rest = &body[from..];
            rest.find('>').map(|end| &rest[..=end])
        })
        .unwrap_or("")
}

/// Return the opening tag of the submit button inside `#password-form`, or an empty text.
fn submit_button_tag(body: &str) -> &str {
    let Some(start) = body.find("<form id=\"password-form\"") else {
        return "";
    };
    let form = &body[start..];
    let form = form.find("</form>").map_or(form, |end| &form[..end]);
    form.find("<button type=\"submit\"")
        .and_then(|from| {
            let rest = &form[from..];
            rest.find('>').map(|end| &rest[..=end])
        })
        .unwrap_or("")
}

/// Assert that the submit button of `#password-form` has the fixed `id`.
fn assert_submit_button_id(body: &str) {
    let tag = submit_button_tag(body);
    assert!(tag.contains(" id=\"password-submit\""), "{tag:?} in {body}");
}

#[tokio::test]
async fn pages_own_password_form_submit_button_has_the_fixed_id() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let reply = get_with(
        &fixture.router(),
        "/me/password",
        &[("cookie", &session.cookie), ENGLISH],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_submit_button_id(&reply.body);
}

#[tokio::test]
async fn pages_own_a_short_new_password_swaps_the_form_with_the_same_button_id() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = fixture.session_of(1);
    let reply = change_in_english(&fixture, &session, &change_body(PASSWORD, "abc")).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let form_tag = form_start_tag(&reply.body);
    assert!(
        form_tag.contains(" hx-swap-oob=\"true\""),
        "{form_tag:?} in {}",
        reply.body
    );
    assert_submit_button_id(&reply.body);
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
