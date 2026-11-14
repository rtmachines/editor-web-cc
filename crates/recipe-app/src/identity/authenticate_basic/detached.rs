//! Tests of the detached tasks of `authenticate_basic` after a dropped request (ADR-0173).

use std::sync::Arc;

use async_trait::async_trait;
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::identity::{PasswordHash, Role, ThrottlePolicy, UserId};
use serde_json::json;
use tokio::sync::Notify;
use tracing::Level;

use super::authenticate_basic;
use super::tests::{
    Fixture, PASSWORD, assert_no_trace_of_success, credentials, meta, reset_password, user_id,
};
use crate::error::StorageError;
use crate::identity::fixtures::{DRAIN_WAIT, HeldSink, LogCapture, poll_once_then_drop};
use crate::memory::{Fault, MemoryStore};
use crate::ports::{AttemptId, AttemptKey, AttemptOutcome, BasicConfirmation, LoginAttemptStore};

// ---- the detached event of a T2 refusal (ADR-0173) ----

/// An attempt store that never ends T2 of the Basic check. T1 runs on `inner`.
struct StuckAtT2 {
    inner: MemoryStore,
    entered: Arc<Notify>,
}

#[async_trait]
impl LoginAttemptStore for StuckAtT2 {
    async fn begin_attempt(
        &self,
        key: &AttemptKey,
        policy: &ThrottlePolicy,
    ) -> Result<AttemptOutcome, StorageError> {
        self.inner.begin_attempt(key, policy).await
    }

    async fn confirm_basic(
        &self,
        _attempt: AttemptId,
        _user: UserId,
        _expected_hash: &PasswordHash,
    ) -> Result<BasicConfirmation, StorageError> {
        self.entered.notify_one();
        std::future::pending().await
    }
}

/// Free every detached slot and every permit, then assert that the drain finds no task left.
async fn assert_slots_and_permits_return(fixture: &Fixture) {
    let slots = fixture.context.settings().detached_tasks;
    let mut held = Vec::new();
    for _ in 0..slots {
        held.push(fixture.context.detached().acquire().await.unwrap());
    }
    drop(held);
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

/// Start a Basic check that T2 refuses, stop it inside the write of the event, and drop it.
///
/// The task gets its release after the drop and writes the event. The function returns after the
/// drain of the detached tasks.
async fn drop_the_request_during_the_t2_refusal_write(fixture: &Fixture) {
    let held = HeldSink::over(fixture.sink.clone());
    let mut context = fixture.context.clone();
    context.app.security = Arc::new(held.clone());
    let store = fixture.store.clone();
    fixture
        .hasher
        .change_during_check(move || reset_password(&store));
    let request_meta = meta();
    {
        let mut request = std::pin::pin!(authenticate_basic(
            &context,
            credentials("anna", PASSWORD),
            &request_meta
        ));
        tokio::select! {
            _ended = &mut request => panic!("the check ended before the event write"),
            () = held.entered.notified() => {}
        }
        poll_once_then_drop(request.as_mut()).await;
    }
    held.release.notify_one();
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);
}

#[tokio::test]
async fn basic_check_writes_the_t2_refusal_event_once_after_the_caller_drops_the_future() {
    let fixture = Fixture::with_anna(Role::ExportClient);

    drop_the_request_during_the_t2_refusal_write(&fixture).await;

    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.user(), Some(user_id()));
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "wrong_password", "scheme": "basic"}))
    );
    assert_no_trace_of_success(&fixture);
}

#[tokio::test]
async fn basic_check_writes_one_warn_line_for_a_lost_t2_refusal_event_after_the_caller_drops_the_future()
 {
    let (capture, _guard) = LogCapture::install();
    let fixture = Fixture::with_anna(Role::ExportClient);
    fixture.sink.fail_with(Fault::Unavailable);

    drop_the_request_during_the_t2_refusal_write(&fixture).await;

    let lines = capture.events();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = lines.first().unwrap();
    assert_eq!(line.level, Level::WARN);
    assert_eq!(line.field("event"), Some("\"login_failed\""));
    assert!(line.field("request_id").is_some(), "{line:?}");
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn a_basic_t2_grant_ends_the_task_with_no_write_and_returns_the_slot() {
    let fixture = Fixture::with_anna(Role::ExportClient);

    let principal = fixture.check("anna", PASSWORD).await;

    assert!(principal.is_ok(), "{principal:?}");
    assert_slots_and_permits_return(&fixture).await;
    assert!(fixture.sink.events().is_empty());
    assert!(
        fixture
            .store
            .state()
            .attempts
            .values()
            .all(|row| row.succeeded)
    );
}

#[tokio::test]
async fn a_basic_drop_during_the_t2_await_ends_the_task_with_no_write_and_returns_the_slot() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let entered = Arc::new(Notify::new());
    let mut context = fixture.context.clone();
    context.attempts = Arc::new(StuckAtT2 {
        inner: fixture.store.clone(),
        entered: Arc::clone(&entered),
    });
    let request_meta = meta();
    {
        let mut request = std::pin::pin!(authenticate_basic(
            &context,
            credentials("anna", PASSWORD),
            &request_meta
        ));
        tokio::select! {
            _ended = &mut request => panic!("the check ended before T2"),
            () = entered.notified() => {}
        }
        poll_once_then_drop(request.as_mut()).await;
    }

    assert_slots_and_permits_return(&fixture).await;
    assert!(fixture.sink.events().is_empty());
    assert!(fixture.store.state().sessions.is_empty());
}
