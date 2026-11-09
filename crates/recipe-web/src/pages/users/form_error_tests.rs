//! Tests of the form errors of the role form and the new-user form (task 1.7, ADR-0041).

use axum::http::StatusCode;

use super::tests::{assert_unique_ids, get_as, post_as, role_select_selected_values, world};
use crate::pages::test_helpers::{assert_clean_html, tags};

// --- A role outside the list (task 1.7, ADR-0041) -----------------------------------------------

/// Assert the wiring of the select `role` after a refused role and the line under it.
///
/// The select has `aria-invalid="true"` and `aria-describedby="role-error"`. The element
/// `role-error` occurs once and holds the code `E-WEB-008` and one sentence.
fn assert_role_error_line(body: &str) {
    let found = tags(body);
    let select = found
        .iter()
        .find(|tag| tag.name == "select" && tag.attribute("name") == Some("role") && !tag.closing)
        .expect("the answer holds the select role");
    assert_eq!(select.attribute("aria-invalid"), Some("true"), "{body}");
    assert_eq!(
        select.attribute("aria-describedby"),
        Some("role-error"),
        "{body}"
    );
    assert_eq!(body.matches("id=\"role-error\"").count(), 1, "{body}");
    let (_before, rest) = body.split_once("id=\"role-error\"").unwrap();
    let line = rest.split("</div>").next().unwrap();
    assert!(line.contains("E-WEB-008"), "{line}");
    assert_eq!(line.matches("<p>").count(), 1, "{line}");
}

/// Assert the HTMX answer of a refused form: the form is the one out-of-band element.
fn assert_oob_form(body: &str, form_id: &str) {
    assert!(!body.contains("<html"), "a fragment: {body}");
    let start = format!("<form id=\"{form_id}\"");
    let (_before, form) = body.split_once(&start).expect(body);
    let tag = form.split('>').next().unwrap();
    assert!(tag.contains(" hx-swap-oob=\"true\""), "{tag}");
    assert_eq!(body.matches(&start).count(), 1, "{body}");
}

#[tokio::test]
async fn pages_users_set_role_outside_the_list_shows_the_line_under_the_select() {
    let (fixture, admin) = world();
    let reply = post_as(&fixture, &admin, "/users/user2/role", "role=root", false).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("<html"), "a full page");
    assert_role_error_line(&reply.body);
    assert_eq!(role_select_selected_values(&reply.body), ["operator"]);
    assert!(!reply.body.contains("hx-swap-oob"), "{}", reply.body);
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_users_set_role_outside_the_list_swaps_the_form_out_of_band_for_htmx() {
    let (fixture, admin) = world();
    for body in ["role=root", "role="] {
        let reply = post_as(&fixture, &admin, "/users/user2/role", body, true).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.body
        );
        assert_eq!(reply.header("hx-retarget"), "#error-panel");
        assert!(reply.body.contains("E-WEB-008"), "{}", reply.body);
        assert_oob_form(&reply.body, "user-role-form");
        assert_role_error_line(&reply.body);
        assert_unique_ids(&reply.body);
        assert!(
            reply.body.contains("action=\"/users/user2/role\""),
            "{}",
            reply.body
        );
        assert_eq!(role_select_selected_values(&reply.body), ["operator"]);
    }
}

#[tokio::test]
async fn pages_users_set_role_form_on_the_page_has_the_id_of_the_out_of_band_form() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users/user2").await;
    assert!(
        reply.body.contains("<form id=\"user-role-form\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("id=\"role-error\""), "{}", reply.body);
    assert_unique_ids(&reply.body);
}

#[tokio::test]
async fn pages_users_role_form_has_a_submit_button_with_a_stable_id_in_every_answer() {
    let (fixture, admin) = world();
    let page = get_as(&fixture, &admin, "/users/user2").await;
    let success = post_as(
        &fixture,
        &admin,
        "/users/user2/role",
        "role=process_engineer",
        true,
    )
    .await;
    let refusal = post_as(&fixture, &admin, "/users/user2/role", "role=root", true).await;
    for (name, body) in [
        ("page", &page.body),
        ("success", &success.body),
        ("refusal", &refusal.body),
    ] {
        assert_eq!(body.matches("id=\"user-role-submit\"").count(), 1, "{name}");
        let start = body.find("<form id=\"user-role-form\"").unwrap();
        let form = &body[start..];
        let form = &form[..form.find("</form>").unwrap()];
        assert!(
            form.contains("<button type=\"submit\" id=\"user-role-submit\">"),
            "{name}"
        );
    }
}

#[tokio::test]
async fn pages_users_set_role_with_a_bad_role_checks_the_permission_first() {
    let (fixture, _admin) = world();
    let operator = fixture.session_of(2);
    let reply = post_as(&fixture, &operator, "/users/user2/role", "role=root", true).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"), "{}", reply.body);
}

#[tokio::test]
async fn pages_users_create_with_a_role_outside_the_list_shows_the_line_under_the_select() {
    let (fixture, admin) = world();
    let reply = post_as(&fixture, &admin, "/users", "login=newuser&role=root", false).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("<html"), "a full page");
    assert_role_error_line(&reply.body);
    assert!(reply.body.contains("value=\"newuser\""), "{}", reply.body);
    assert_eq!(role_select_selected_values(&reply.body), ["operator"]);
    assert!(!reply.body.contains("hx-swap-oob"), "{}", reply.body);
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_users_create_with_a_role_outside_the_list_swaps_the_form_out_of_band_for_htmx() {
    let (fixture, admin) = world();
    let reply = post_as(&fixture, &admin, "/users", "login=newuser&role=root", true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert!(reply.body.contains("E-WEB-008"), "{}", reply.body);
    assert_oob_form(&reply.body, "user-new-form");
    assert_role_error_line(&reply.body);
    assert_unique_ids(&reply.body);
    assert!(reply.body.contains("value=\"newuser\""), "{}", reply.body);
    assert!(!reply.body.contains("one-time-password"), "{}", reply.body);
}

#[tokio::test]
async fn pages_users_create_lists_a_bad_login_and_a_bad_role_at_once() {
    let (fixture, admin) = world();
    let reply = post_as(&fixture, &admin, "/users", "login=ab&role=root", true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_role_error_line(&reply.body);
    assert_eq!(
        reply.body.matches("id=\"login-error\"").count(),
        1,
        "{}",
        reply.body
    );
    assert_unique_ids(&reply.body);
}

#[tokio::test]
async fn pages_users_create_with_a_bad_role_is_forbidden_for_an_operator() {
    let (fixture, _admin) = world();
    let operator = fixture.session_of(2);
    let reply = post_as(
        &fixture,
        &operator,
        "/users",
        "login=newuser&role=root",
        true,
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"), "{}", reply.body);
}

#[tokio::test]
async fn pages_users_new_form_has_the_id_of_the_out_of_band_form() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users/new").await;
    assert!(
        reply.body.contains("<form id=\"user-new-form\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("hx-swap-oob"), "{}", reply.body);
}

/// Return the opening tag of the submit button inside `#user-new-form`, or an empty text.
fn submit_button_tag(body: &str) -> &str {
    let Some(start) = body.find("<form id=\"user-new-form\"") else {
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

#[tokio::test]
async fn pages_users_new_form_submit_button_has_the_fixed_id_in_the_page_and_the_refusal() {
    let (fixture, admin) = world();
    let page = get_as(&fixture, &admin, "/users/new").await;
    let refusal = post_as(&fixture, &admin, "/users", "login=newuser&role=root", false).await;
    assert_eq!(
        refusal.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        refusal.body
    );
    for (name, body) in [("page", &page.body), ("refusal", &refusal.body)] {
        let tag = submit_button_tag(body);
        assert!(
            tag.contains(" id=\"user-new-submit\""),
            "{name}: {tag:?} in {body}"
        );
    }
}
