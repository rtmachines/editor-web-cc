//! Tests of the user pages in read-only mode (ADR-0113).

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use super::tests::{WRONG_FIELD, get_as, login, world};

/// Return the text of the element `main`, without the layout around it.
fn main_text(body: &str) -> &str {
    let (_before, rest) = body.split_once("<main").unwrap();
    rest.split("</main>").next().unwrap()
}

#[tokio::test]
async fn pages_users_list_in_read_only_mode_holds_no_link_to_a_new_user() {
    let (fixture, admin) = world();
    fixture.state.read_only().set(true);
    let reply = get_as(&fixture, &admin, "/users").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_text(&reply.body);
    assert!(!main.contains("href=\"/users/new\""), "{main}");
    assert!(main.contains("href=\"/users/user2\""), "{main}");
    assert!(main.contains("name=\"q\""), "{main}");
}

#[tokio::test]
async fn pages_users_detail_in_read_only_mode_holds_no_action_form_and_no_reset_disclosure() {
    let (fixture, admin) = world();
    for _attempt in 0..2 {
        let failed = login(&fixture, "user2", WRONG_FIELD).await;
        assert_eq!(failed.status, StatusCode::UNAUTHORIZED);
    }
    fixture.state.read_only().set(true);
    let reply = get_as(&fixture, &admin, "/users/user2").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_text(&reply.body);
    assert!(main.contains("id=\"user-facts\""), "{main}");
    assert!(main.contains("id=\"failures\">2<"), "{main}");
    for gone in [
        "<form",
        "<button",
        "<details",
        "csrf_token",
        "reset-password",
        "user-role-form",
        "user-status-submit",
    ] {
        assert!(!main.contains(gone), "{gone}: {main}");
    }
    assert!(main.contains("href=\"/users\""), "{main}");
}

#[tokio::test]
async fn pages_users_new_form_in_read_only_mode_shows_the_notice_in_place_of_the_form() {
    let (fixture, admin) = world();
    fixture.state.read_only().set(true);
    let reply = get_as(&fixture, &admin, "/users/new").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_text(&reply.body);
    assert!(main.contains("<h1>New user</h1>"), "{main}");
    assert!(
        main.contains("<div class=\"alert alert-warning\">"),
        "{main}"
    );
    assert!(
        main.contains("<span>Read-only mode blocks this form.</span>"),
        "{main}"
    );
    for gone in ["<form", "user-new-form", "csrf_token", "name=\"login\""] {
        assert!(!main.contains(gone), "{gone}: {main}");
    }
    assert!(main.contains("href=\"/users\""), "{main}");
}

#[tokio::test]
async fn pages_users_new_form_in_read_only_mode_still_refuses_an_operator() {
    let (fixture, _admin) = world();
    let operator = fixture.seed_session(3, Role::Operator, false);
    fixture.state.read_only().set(true);
    let reply = get_as(&fixture, &operator, "/users/new").await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
}
