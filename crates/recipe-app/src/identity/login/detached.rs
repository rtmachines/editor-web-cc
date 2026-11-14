//! Tests of the detached tasks of `Login` after a dropped request (ADR-0172, ADR-0173).

use std::future::Future;
use std::sync::Arc;
use std::task::Poll;

use async_trait::async_trait;
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{CredentialFailure, IdentityError};
use tokio::sync::Notify;
use tracing::Level;

use super::Login;
use super::tests::{
    PASSWORD, assert_one_event, deactivate, fixture, input, meta, racing_fixture, reset,
    seed_failures, wrong_password,
};
use crate::error::{AppError, StorageError, TimeoutStage};
use crate::identity::fixtures::{
    DRAIN_WAIT, HeldSink, IdentityFixture, LogCapture, poll_once_then_drop,
};
use crate::memory::{Fault, MemoryStore};
use crate::ports::{Isolation, Transaction, UnitOfWork};

#[tokio::test]
async fn login_writes_login_failed_after_the_caller_drops_the_future() {
    let fixture = fixture();
    let login = Login::new(fixture.context.clone());

    poll_once_then_drop(login.execute(&meta(), input("anna", "not-the-password"))).await;
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);

    assert_one_event(&fixture, SecurityEventKind::LoginFailed);
    let state = fixture.store.state();
    assert_eq!(state.attempts.len(), 1);
    assert!(state.sessions.is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn login_writes_login_throttled_after_the_caller_drops_the_future() {
    let fixture = fixture();
    seed_failures(&fixture.store, &[10, 9, 8, 1]);
    let login = Login::new(fixture.context.clone());

    poll_once_then_drop(login.execute(&meta(), input("anna", PASSWORD))).await;
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);

    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(
        events.first().unwrap().kind(),
        SecurityEventKind::LoginThrottled
    );
    assert_eq!(fixture.hasher.verify_calls(), 0, "no password is checked");
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn login_creates_no_session_after_the_caller_drops_the_future() {
    let fixture = fixture();
    let login = Login::new(fixture.context.clone());

    poll_once_then_drop(login.execute(&meta(), input("anna", PASSWORD))).await;
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);

    let state = fixture.store.state();
    assert!(
        state.sessions.is_empty(),
        "T2 runs only for a waiting caller"
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(state.attempts.len(), 1);
    assert!(state.attempts.values().all(|row| !row.succeeded));
    assert!(fixture.sink.events().is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn login_refuses_with_one_warn_line_when_the_event_fails() {
    type Check = fn(&AppError) -> bool;
    let wrong: Check = |error| wrong_password()(error);
    let unknown: Check = |error| {
        matches!(
            error,
            AppError::Identity(IdentityError::InvalidCredentials {
                cause: CredentialFailure::UnknownLogin
            })
        )
    };
    let inactive: Check = |error| matches!(error, AppError::Identity(IdentityError::UserInactive));
    let throttled: Check = |error| {
        matches!(
            error,
            AppError::Identity(IdentityError::LoginThrottled { retry_after: 4 })
        )
    };
    let throttled_fixture = fixture();
    seed_failures(&throttled_fixture.store, &[10, 9, 8, 1]);
    let inactive_fixture = fixture();
    deactivate(&inactive_fixture.store);
    let cases = [
        (
            "wrong password",
            fixture(),
            input("anna", "wrong-password"),
            "login_failed",
            wrong,
        ),
        (
            "unknown login",
            fixture(),
            input("nobody", PASSWORD),
            "login_failed",
            unknown,
        ),
        (
            "inactive user",
            inactive_fixture,
            input("anna", PASSWORD),
            "login_rejected_inactive",
            inactive,
        ),
        (
            "throttled attempt",
            throttled_fixture,
            input("anna", PASSWORD),
            "login_throttled",
            throttled,
        ),
        (
            "user changed before T2",
            racing_fixture(reset),
            input("anna", PASSWORD),
            "login_failed",
            wrong,
        ),
    ];
    for (name, fixture, request, kind, answers) in cases {
        let (capture, _guard) = LogCapture::install();
        fixture.sink.fail_with(Fault::Unavailable);

        let error = Login::new(fixture.context.clone())
            .execute(&meta(), request)
            .await
            .unwrap_err();

        assert!(answers(&error), "{name}: {error:?}");
        let lines = capture.events();
        assert_eq!(lines.len(), 1, "{name}: {lines:?}");
        let line = lines.first().unwrap();
        assert_eq!(line.level, Level::WARN, "{name}");
        assert_eq!(
            line.field("event"),
            Some(format!("{kind:?}").as_str()),
            "{name}"
        );
        assert!(line.field("request_id").is_some(), "{name}: {line:?}");
        assert!(fixture.sink.events().is_empty(), "{name}");
        assert_eq!(fixture.context.gate().available_permits(), 4, "{name}");
    }
}

#[tokio::test]
async fn a_recorded_event_writes_no_warn_line() {
    let (capture, _guard) = LogCapture::install();
    let fixture = fixture();

    Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", "not-the-password"))
        .await
        .unwrap_err();

    assert_one_event(&fixture, SecurityEventKind::LoginFailed);
    assert_eq!(capture.events().len(), 0);
}

#[tokio::test]
async fn a_storage_failure_at_t1_in_the_task_returns_the_error_and_frees_the_slot() {
    let fixture = fixture();
    fixture.store.fail_begin(Fault::Unavailable);
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    assert!(fixture.context.detached().acquire().await.is_ok());
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);
}

// ---- the detached event of a T2 refusal (ADR-0173) ----

/// A unit of work that never ends the first `READ COMMITTED` begin, which is the begin of T2.
struct StuckAtT2 {
    inner: MemoryStore,
    entered: Arc<Notify>,
}

#[async_trait]
impl UnitOfWork for StuckAtT2 {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError> {
        if isolation == Isolation::ReadCommitted {
            self.entered.notify_one();
            std::future::pending::<()>().await;
        }
        self.inner.begin(isolation).await
    }
}

/// Start a login that T2 refuses, stop it inside the write of the event, and drop it.
///
/// The task gets its release after the drop and writes the event. The function returns after the
/// drain of the detached tasks.
async fn drop_the_request_during_the_t2_refusal_write(fixture: &IdentityFixture) {
    let held = HeldSink::over(fixture.sink.clone());
    let mut context = fixture.context.clone();
    context.app.security = Arc::new(held.clone());
    let login = Login::new(context);
    let request_meta = meta();
    {
        let mut request = std::pin::pin!(login.execute(&request_meta, input("anna", PASSWORD)));
        tokio::select! {
            _ended = &mut request => panic!("the login ended before the event write"),
            () = held.entered.notified() => {}
        }
        poll_once_then_drop(request.as_mut()).await;
    }
    held.release.notify_one();
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);
}

#[tokio::test]
async fn login_writes_the_t2_refusal_event_once_after_the_caller_drops_the_future() {
    let fixture = racing_fixture(reset);

    drop_the_request_during_the_t2_refusal_write(&fixture).await;

    assert_one_event(&fixture, SecurityEventKind::LoginFailed);
    let state = fixture.store.state();
    assert!(state.sessions.is_empty());
    assert!(state.attempts.values().all(|row| !row.succeeded));
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn login_writes_one_warn_line_for_a_lost_t2_refusal_event_after_the_caller_drops_the_future()
{
    let (capture, _guard) = LogCapture::install();
    let fixture = racing_fixture(reset);
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
async fn a_t2_grant_ends_the_task_with_no_write_and_returns_the_slot() {
    let fixture = fixture();

    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await;

    assert!(outcome.is_ok(), "{outcome:?}");
    let slots = fixture.context.settings.detached_tasks;
    let mut held = Vec::new();
    for _ in 0..slots {
        held.push(fixture.context.detached().acquire().await.unwrap());
    }
    drop(held);
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);
    assert!(fixture.sink.events().is_empty());
    assert_eq!(fixture.store.state().sessions.len(), 1);
}

#[tokio::test]
async fn a_drop_during_the_t2_await_ends_the_task_with_no_write_and_returns_the_slot() {
    let fixture = fixture();
    let entered = Arc::new(Notify::new());
    let mut context = fixture.context.clone();
    context.app.uow = Arc::new(StuckAtT2 {
        inner: fixture.store.clone(),
        entered: Arc::clone(&entered),
    });
    let login = Login::new(context);
    let request_meta = meta();
    {
        let mut request = std::pin::pin!(login.execute(&request_meta, input("anna", PASSWORD)));
        tokio::select! {
            _ended = &mut request => panic!("the login ended before T2"),
            () = entered.notified() => {}
        }
        std::future::poll_fn(|cx| {
            assert!(request.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
    }

    let slots = fixture.context.settings.detached_tasks;
    let mut held = Vec::new();
    for _ in 0..slots {
        held.push(fixture.context.detached().acquire().await.unwrap());
    }
    drop(held);
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);
    assert!(fixture.sink.events().is_empty());
    assert!(fixture.store.state().sessions.is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn login_after_the_drain_fails_at_once_with_e_web_009_and_commits_no_attempt() {
    let fixture = fixture();
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);
    let login = Login::new(fixture.context.clone());

    let outcome = login.execute(&meta(), input("anna", PASSWORD)).await;

    let error = outcome.expect_err("a login after the drain must fail");
    assert_eq!(error.code(), ErrorCode::WEB_009, "{error:?}");
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::DetachedSlot
        }
    ));
    let state = fixture.store.state();
    assert!(state.attempts.is_empty(), "T1 must not run after the drain");
    assert!(state.sessions.is_empty());
    assert!(fixture.sink.events().is_empty());
    assert_eq!(fixture.hasher.verify_calls(), 0, "no password is checked");
    assert_eq!(fixture.context.gate().available_permits(), 4);
}
