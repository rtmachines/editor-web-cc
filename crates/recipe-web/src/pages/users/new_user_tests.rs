//! Tests of the form `user-new` and of `POST /users` (task 6.8, `api.md` section 2.8, ADR-0172).

use axum::http::StatusCode;

use super::tests::{
    assert_one_time_password_shape, get_as, login, one_time_password, post_as,
    role_select_selected_values, world,
};
use crate::pages::test_helpers::{assert_clean_html, tags};
use crate::test_support::capture_logs;

// --- GET /users/new and POST /users -------------------------------------------------------------

#[tokio::test]
async fn pages_users_new_form_has_login_role_and_the_token() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users/new").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("name=\"login\""));
    assert!(reply.body.contains("name=\"role\""));
    for role in ["administrator", "process_engineer", "operator"] {
        assert!(reply.body.contains(&format!("value=\"{role}\"")), "{role}");
    }
    let hidden = format!("name=\"csrf_token\" value=\"{}\"", admin.csrf);
    assert!(reply.body.contains(&hidden));
    assert_clean_html(&reply.body);
}

/// Design D11 (b), ADR-0172: the form of a new user drops a second submit while the first runs.
/// The answer stays the full result page, so the boosted form does not push a URL.
#[tokio::test]
async fn user_new_form_drops_a_second_submit() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users/new").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let found = tags(&reply.body);
    let form = found
        .iter()
        .find(|tag| tag.name == "form" && tag.attribute("id") == Some("user-new-form"))
        .expect("the page holds the form user-new-form");
    assert_eq!(form.attribute("action"), Some("/users"));
    assert_eq!(form.attribute("method"), Some("post"));
    assert_eq!(form.attribute("hx-boost"), Some("true"));
    assert_eq!(form.attribute("hx-sync"), Some("closest form:drop"));
    assert_eq!(form.attribute("hx-push-url"), Some("false"));
}

#[tokio::test]
async fn pages_users_new_form_role_select_has_a_label_and_aria_required() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users/new").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(role_select_selected_values(&reply.body), ["operator"]);
}

#[tokio::test]
async fn pages_users_detail_role_select_has_a_label_and_selects_the_current_role() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users/user2").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(role_select_selected_values(&reply.body), ["operator"]);
    let own = get_as(&fixture, &admin, "/users/user1").await;
    assert_eq!(own.status, StatusCode::OK, "{}", own.body);
    assert_eq!(role_select_selected_values(&own.body), ["administrator"]);
}

#[tokio::test]
async fn pages_users_create_shows_the_one_time_password_once() {
    let (fixture, admin) = world();
    let (logs, _guard) = capture_logs();
    let reply = post_as(
        &fixture,
        &admin,
        "/users",
        "login=newuser&role=operator",
        false,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("cache-control"), "no-store");
    let password = one_time_password(&reply.body);
    assert_one_time_password_shape(&password);
    assert!(reply.body.contains("newuser"));
    assert_clean_html(&reply.body);
    // The password is not in the log, the audit entries, or a later page.
    assert!(!logs.text().contains(&password));
    let audit = format!("{:?}", fixture.store.audit_entries());
    assert!(!audit.contains(&password));
    let detail = get_as(&fixture, &admin, "/users/newuser").await;
    assert_eq!(detail.status, StatusCode::OK, "{}", detail.body);
    assert!(!detail.body.contains(&password));
    let list = get_as(&fixture, &admin, "/users").await;
    assert!(!list.body.contains(&password));
    // The user logs in with it and must change it.
    let first = login(&fixture, "newuser", &password).await;
    assert_eq!(first.status, StatusCode::SEE_OTHER, "{}", first.body);
    assert_eq!(first.header("location"), "/me/password");
}

#[tokio::test]
async fn pages_users_create_with_htmx_gives_the_same_page() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users",
        "login=newuser&role=operator",
        true,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_one_time_password_shape(&one_time_password(&reply.body));
}

#[tokio::test]
async fn pages_users_create_with_a_known_login_gives_409_and_no_password() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users",
        "login=user2&role=operator",
        false,
    )
    .await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(reply.body.contains("E-IDN-001"));
    assert!(!reply.body.contains("one-time-password"));
}

#[tokio::test]
async fn pages_users_create_rejects_bad_fields_with_422_and_never_echoes_them() {
    let (fixture, admin) = world();
    let cases = [
        "login=ab&role=operator",
        "login=%3Cscript%3Ealert(1)%3C%2Fscript%3E&role=operator",
        "login=newuser&role=root",
        "login=newuser",
        "role=operator",
        "login=newuser&role=operator&extra=1",
    ];
    for body in cases {
        let reply = post_as(&fixture, &admin, "/users", body, false).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert!(reply.body.contains("E-WEB-008"), "{body}: {}", reply.body);
        assert!(!reply.body.contains("<script>alert(1)"), "{body}");
        assert!(!reply.body.contains("one-time-password"), "{body}");
    }
}

#[tokio::test]
async fn pages_users_create_is_forbidden_for_an_operator() {
    let (fixture, _admin) = world();
    let operator = fixture.session_of(2);
    let reply = post_as(
        &fixture,
        &operator,
        "/users",
        "login=newuser&role=operator",
        false,
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"));
    assert!(!reply.body.contains("one-time-password"));
}
