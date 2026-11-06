//! Tests of the unlock route of the user pages (ADR-0130).

use axum::http::StatusCode;
use recipe_app::memory::Fault;
use recipe_domain::audit::{AuditAction, SecurityEventKind};

use super::tests::{WRONG_FIELD, login, post_as, world};
use crate::test_support::capture_logs;
#[tokio::test]
async fn pages_users_unlock_clears_the_login_delay_with_the_client_address() {
    let (fixture, admin) = world();
    for _attempt in 0..6 {
        let _reply = login(&fixture, "user2", WRONG_FIELD).await;
    }
    let blocked = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(
        blocked.status,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        blocked.body
    );
    let reply = post_as(&fixture, &admin, "/users/user2/unlock", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let kinds: Vec<_> = fixture.sink.events().iter().map(|e| e.kind()).collect();
    assert!(kinds.contains(&SecurityEventKind::Unlock), "{kinds:?}");
    let free = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(free.status, StatusCode::SEE_OTHER, "{}", free.body);
}

#[tokio::test]
async fn pages_users_unlock_with_a_lost_event_is_a_success_with_one_warn_line() {
    let (fixture, admin) = world();
    for _attempt in 0..3 {
        let _reply = login(&fixture, "user2", WRONG_FIELD).await;
    }
    assert_eq!(fixture.store.state().login_attempt_count(), 3);
    fixture.sink.fail_with(Fault::Unavailable);
    let (logs, _guard) = capture_logs();
    let reply = post_as(&fixture, &admin, "/users/user2/unlock", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("E-DB-005"), "{}", reply.body);
    assert_eq!(fixture.store.state().login_attempt_count(), 0);
    let unlocked = fixture
        .store
        .audit_entries()
        .iter()
        .filter(|entry| entry.action() == AuditAction::LoginUnlocked)
        .count();
    assert_eq!(unlocked, 1);
    let text = logs.text();
    let warnings: Vec<&str> = text.lines().filter(|line| line.contains("WARN")).collect();
    assert_eq!(warnings.len(), 1, "{text}");
    let line = warnings.first().unwrap();
    assert!(line.contains("error_code=E-DB-005"), "{line}");
    assert!(
        line.contains(&format!("request_id={}", reply.header("x-request-id"))),
        "{line}"
    );
    assert!(
        line.contains("chain=The database does not answer. Try again."),
        "{line}"
    );
}

#[tokio::test]
async fn pages_users_unlock_with_a_recorded_event_writes_no_warn_line() {
    let (fixture, admin) = world();
    let (logs, _guard) = capture_logs();
    let reply = post_as(&fixture, &admin, "/users/user2/unlock", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let text = logs.text();
    assert!(!text.lines().any(|line| line.contains("WARN")), "{text}");
}
