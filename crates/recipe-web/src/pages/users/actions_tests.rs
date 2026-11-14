//! Tests of the commands of the user page: HTMX answers, out-of-band refresh, and labels (task 1.4, task 1.8).

use axum::http::StatusCode;
use recipe_app::memory::{Fault, FaultPoint, JournalEntry};
use recipe_app::ports::Isolation;
use recipe_domain::identity::Role;

use super::tests::{
    PASSWORD, WRONG_FIELD, assert_one_time_password_shape, assert_unique_ids, get_as, login,
    me_status, one_time_password, post_as, world,
};
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::{Fixture, LogBuffer, Reply, capture_logs, deactivate_user, get_with};

// --- The out-of-band refresh of the user page (task 1.4, design D1) ------------------------------

/// Return the opening tag of the element with the `id`.
fn tag_of<'a>(body: &'a str, id: &str) -> &'a str {
    let needle = format!("id=\"{id}\"");
    let (before, _after) = body.split_once(&needle).unwrap();
    let start = before.rfind('<').unwrap();
    body[start..].split_inclusive('>').next().unwrap()
}

/// Return the markup of the facts `dl` of an answer.
fn facts_of(body: &str) -> &str {
    let (_before, after) = body.split_once("id=\"user-facts\"").unwrap();
    after.split("</dl>").next().unwrap()
}

/// Assert the two out-of-band parts of an answer of a command of the user page.
fn assert_refresh_parts(body: &str) {
    assert!(tag_of(body, "user-facts").contains("hx-swap-oob=\"true\""));
    assert!(tag_of(body, "user-actions").contains("hx-swap-oob=\"true\""));
    assert_unique_ids(body);
    assert_clean_html(body);
}

#[tokio::test]
async fn pages_users_deactivate_answer_refreshes_facts_and_actions_out_of_band() {
    let (fixture, admin) = world();
    let reply = post_as(&fixture, &admin, "/users/user2/deactivate", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_refresh_parts(&reply.body);
    assert!(facts_of(&reply.body).contains("Inactive"), "{}", reply.body);
    assert!(reply.body.contains("action=\"/users/user2/activate\""));
    assert!(!reply.body.contains("action=\"/users/user2/deactivate\""));
    assert!(
        !reply
            .body
            .contains("action=\"/users/user2/reset-password\"")
    );
    assert_eq!(reply.body.matches("id=\"user-status-submit\"").count(), 1);
}

#[tokio::test]
async fn pages_users_activate_answer_refreshes_facts_and_actions_out_of_band() {
    let (fixture, admin) = world();
    deactivate_user(&fixture, "user2").await;
    let reply = post_as(&fixture, &admin, "/users/user2/activate", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_refresh_parts(&reply.body);
    assert!(facts_of(&reply.body).contains("Active"), "{}", reply.body);
    assert!(!facts_of(&reply.body).contains("Inactive"));
    assert!(reply.body.contains("action=\"/users/user2/deactivate\""));
    assert!(!reply.body.contains("action=\"/users/user2/activate\""));
}

#[tokio::test]
async fn pages_users_role_answer_refreshes_facts_and_actions_out_of_band() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users/user2/role",
        "role=process_engineer",
        true,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_refresh_parts(&reply.body);
    assert!(
        facts_of(&reply.body).contains("Process engineer"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("action=\"/users/user2/role\""));
    assert_eq!(reply.body.matches("id=\"role\"").count(), 1);
}

#[tokio::test]
async fn pages_users_unlock_answer_shows_no_failures_and_no_unlock_form() {
    let (fixture, admin) = world();
    for _attempt in 0..2 {
        let failed = login(&fixture, "user2", WRONG_FIELD).await;
        assert_eq!(failed.status, StatusCode::UNAUTHORIZED);
    }
    let reply = post_as(&fixture, &admin, "/users/user2/unlock", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_refresh_parts(&reply.body);
    assert!(
        facts_of(&reply.body).contains("<dd id=\"failures\">0</dd>"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("action=\"/users/user2/unlock\""));
    assert!(reply.body.contains("action=\"/users/user2/deactivate\""));
}

#[tokio::test]
async fn pages_users_reset_answer_keeps_the_password_and_refreshes_only_the_facts() {
    let (fixture, admin) = world();
    let reply = post_as(&fixture, &admin, "/users/user2/reset-password", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_one_time_password_shape(&one_time_password(&reply.body));
    assert!(tag_of(&reply.body, "user-facts").contains("hx-swap-oob=\"true\""));
    assert!(
        facts_of(&reply.body).contains("Password change"),
        "{}",
        reply.body
    );
    assert!(facts_of(&reply.body).contains("Required"), "{}", reply.body);
    assert!(
        !reply.body.contains("id=\"user-actions\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("action=\"/users/"), "{}", reply.body);
    assert_eq!(
        reply.body.matches("hx-swap-oob").count(),
        3,
        "toast, panel, facts"
    );
    assert!(reply.header("hx-refresh").is_empty());
    assert_unique_ids(&reply.body);
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_users_reset_without_htmx_gives_the_full_page_without_oob_and_without_facts() {
    let (fixture, admin) = world();
    let reply = post_as(&fixture, &admin, "/users/user2/reset-password", "", false).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("<html"));
    assert_one_time_password_shape(&one_time_password(&reply.body));
    assert!(!reply.body.contains("hx-swap-oob"), "{}", reply.body);
    assert!(!reply.body.contains("id=\"user-facts\""));
}

#[tokio::test]
async fn pages_users_reset_does_not_run_when_the_read_of_the_user_fails() {
    let (fixture, admin) = world();
    fixture
        .store
        .fail_next(FaultPoint::Begin, Fault::Unavailable, 1);
    let reply = post_as(&fixture, &admin, "/users/user2/reset-password", "", true).await;
    assert!(reply.status.is_server_error(), "{}", reply.body);
    assert!(!reply.body.contains("one-time-password"), "{}", reply.body);
    let old = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(old.status, StatusCode::SEE_OTHER, "the old password lives");
    assert_eq!(
        me_status(&fixture, &fixture.session_of(2).cookie).await,
        StatusCode::OK
    );
}

/// Design D1 of `web-ui-review-quick-wins`: the read of `GetUser` comes before the command, so an
/// error after the commit never hides the one-time password. A fault test cannot see the order,
/// because the read-only check of `ResetPassword` also opens a transaction. The journal can.
#[tokio::test]
async fn pages_users_htmx_reset_reads_the_user_before_the_command_transaction() {
    let (fixture, admin) = world();
    let before = fixture.store.journal().len();
    let reply = post_as(&fixture, &admin, "/users/user2/reset-password", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let begins: Vec<Isolation> = fixture
        .store
        .journal()
        .into_iter()
        .skip(before)
        .filter_map(|entry| match entry {
            JournalEntry::Begin(isolation) => Some(isolation),
            JournalEntry::Lock(_) => None,
        })
        .collect();
    // The read of `GetUser` and the check of `ResetPassword` are read-only. The command comes last.
    assert!(
        begins.ends_with(&[
            Isolation::ReadOnly,
            Isolation::ReadOnly,
            Isolation::ReadCommitted
        ]),
        "{begins:?}"
    );
    assert!(!one_time_password(&reply.body).is_empty(), "{}", reply.body);
}

#[tokio::test]
async fn pages_users_commands_without_htmx_give_the_full_page_without_oob() {
    let (fixture, admin) = world();
    for (path, body) in [
        ("/users/user2/role", "role=process_engineer"),
        ("/users/user2/deactivate", ""),
        ("/users/user2/activate", ""),
        ("/users/user2/unlock", ""),
    ] {
        let reply = post_as(&fixture, &admin, path, body, false).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
        assert!(reply.body.contains("<html"), "{path}");
        assert!(
            !reply.body.contains("hx-swap-oob"),
            "{path}: {}",
            reply.body
        );
        assert!(!reply.body.contains("id=\"user-facts\""), "{path}");
        assert!(!reply.body.contains("id=\"user-actions\""), "{path}");
    }
}

#[tokio::test]
async fn pages_users_detail_page_has_the_refresh_containers_without_oob() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users/user2").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!tag_of(&reply.body, "user-facts").contains("hx-swap-oob"));
    assert!(!tag_of(&reply.body, "user-actions").contains("hx-swap-oob"));
    let actions_at = reply.body.find("id=\"user-actions\"").unwrap();
    let after_actions = &reply.body[actions_at..];
    let reset_at = after_actions
        .find("action=\"/users/user2/reset-password\"")
        .unwrap();
    let details_at = after_actions.find("<details").unwrap();
    assert!(details_at < reset_at);
    assert!(
        !after_actions[..details_at].contains("reset-password"),
        "the reset disclosure follows the actions"
    );
}

#[tokio::test]
async fn pages_users_commands_are_forbidden_for_an_operator() {
    let (fixture, _admin) = world();
    fixture.seed_account(3, Role::Operator, PASSWORD, false);
    let operator = fixture.session_of(3);
    for (action, body) in [
        ("role", "role=operator"),
        ("deactivate", ""),
        ("activate", ""),
        ("reset-password", ""),
        ("unlock", ""),
    ] {
        let path = format!("/users/user2/{action}");
        let reply = post_as(&fixture, &operator, &path, body, true).await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-002"), "{path}");
    }
    assert_eq!(
        me_status(&fixture, &fixture.session_of(2).cookie).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn pages_users_a_command_without_the_csrf_token_changes_nothing() {
    let (fixture, admin) = world();
    let request = crate::test_support::request(axum::http::Method::POST, "/users/user2/deactivate")
        .header("cookie", admin.cookie.as_str())
        .header("sec-fetch-site", "same-origin")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::empty())
        .unwrap();
    let reply = crate::test_support::call(&fixture.router(), request).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"));
    let login = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(login.status, StatusCode::SEE_OTHER, "user2 is still active");
}

#[tokio::test]
async fn pages_users_pages_follow_the_language_cookie() {
    let (fixture, admin) = world();
    let polish = get_with(&fixture.router(), "/users", &[("cookie", &admin.cookie)]).await;
    assert!(
        polish
            .body
            .contains("<title>Użytkownicy · Edytor receptur</title>"),
        "{}",
        polish.body
    );
    assert!(polish.body.contains("<html lang=\"pl\">"));
}

// --- Heading and labels of the user page (task 1.8, design D8) ----------------------------------

#[tokio::test]
async fn pages_users_detail_heading_reads_user_and_the_login() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users/user2").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<h1>User: <span class=\"data\">user2</span></h1>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn pages_users_detail_heading_reads_the_polish_label_in_pl() {
    let (fixture, admin) = world();
    let reply = get_with(
        &fixture.router(),
        "/users/user2",
        &[("cookie", &admin.cookie)],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<h1>Użytkownik: <span class=\"data\">user2</span></h1>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn pages_users_result_page_without_htmx_reads_user_in_title_and_heading() {
    let (fixture, admin) = world();
    let reply = post_as(&fixture, &admin, "/users/user2/reset-password", "", false).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("<title>User · Recipe editor</title>"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("<h1>User</h1>"), "{}", reply.body);
}

#[tokio::test]
async fn pages_users_one_time_password_label_ends_with_a_colon_for_a_person() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users",
        "login=newuser&role=operator",
        false,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<p>One-time password: <code class=\"data\" id=\"one-time-password\">"),
        "{}",
        reply.body
    );
}

#[test]
fn pages_users_user_templates_hold_no_colon_literal() {
    let templates = [
        ("user.html", include_str!("../../../templates/user.html")),
        (
            "user-result-body.html",
            include_str!("../../../templates/user-result-body.html"),
        ),
    ];
    for (name, text) in templates {
        assert!(!text.contains(": "), "{name} holds a colon literal");
    }
}

// --- The notice after a committed command whose read fails (task 9.8, ADR-0163) -----------------

/// Assert the notice answer of a committed command whose read of the user failed.
///
/// The user commands give no new `row_version`, so the answer holds no `HX-Trigger` (design D1).
fn assert_saved_notice(fixture: &Fixture, audit_before: usize, reply: &Reply, logs: &LogBuffer) {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#toast");
    assert_eq!(reply.header("hx-push-url"), "false");
    assert!(reply.headers.get("hx-trigger").is_none());
    assert!(
        reply.body.contains("The change is saved. Reload the page."),
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>"),
        "{}",
        reply.body
    );
    assert!(fixture.store.audit_entries().len() > audit_before);
    let text = logs.text();
    let warnings: Vec<&str> = text.lines().filter(|line| line.contains("WARN")).collect();
    assert_eq!(warnings.len(), 1, "{text}");
    assert!(
        warnings
            .first()
            .unwrap()
            .contains(&format!("request_id={}", reply.header("x-request-id"))),
        "{text}"
    );
}

#[tokio::test]
async fn set_role_with_failed_read_after_commit_shows_saved_notice() {
    let (fixture, admin) = world();
    let audit_before = fixture.store.audit_entries().len();
    fixture.store.fail_begin_after_commit(Fault::Unavailable);
    let (logs, _guard) = capture_logs();
    let reply = post_as(
        &fixture,
        &admin,
        "/users/user2/role",
        "role=process_engineer",
        true,
    )
    .await;
    assert_saved_notice(&fixture, audit_before, &reply, &logs);
}

#[tokio::test]
async fn deactivate_with_failed_read_after_commit_shows_saved_notice() {
    let (fixture, admin) = world();
    let audit_before = fixture.store.audit_entries().len();
    fixture.store.fail_begin_after_commit(Fault::Unavailable);
    let (logs, _guard) = capture_logs();
    let reply = post_as(&fixture, &admin, "/users/user2/deactivate", "", true).await;
    assert_saved_notice(&fixture, audit_before, &reply, &logs);
}

#[tokio::test]
async fn activate_with_failed_read_after_commit_shows_saved_notice() {
    let (fixture, admin) = world();
    deactivate_user(&fixture, "user2").await;
    let audit_before = fixture.store.audit_entries().len();
    fixture.store.fail_begin_after_commit(Fault::Unavailable);
    let (logs, _guard) = capture_logs();
    let reply = post_as(&fixture, &admin, "/users/user2/activate", "", true).await;
    assert_saved_notice(&fixture, audit_before, &reply, &logs);
}

#[tokio::test]
async fn unlock_with_failed_read_after_commit_shows_saved_notice() {
    let (fixture, admin) = world();
    for _attempt in 0..2 {
        let failed = login(&fixture, "user2", WRONG_FIELD).await;
        assert_eq!(failed.status, StatusCode::UNAUTHORIZED);
    }
    let audit_before = fixture.store.audit_entries().len();
    fixture.store.fail_begin_after_commit(Fault::Unavailable);
    let (logs, _guard) = capture_logs();
    let reply = post_as(&fixture, &admin, "/users/user2/unlock", "", true).await;
    assert_saved_notice(&fixture, audit_before, &reply, &logs);
}
