//! Tests of the permit and the deadline of `GetApprovalPreview` (ADR-0050 (g), design D3).

use std::time::Duration;

use recipe_domain::error_codes::ErrorCode;

use super::approval_preview::{ApprovalPreview, GetApprovalPreview};
use super::approval_preview_tests::{admin, input, valid_world};
use super::diff_tests::one_permit;
use crate::blocking::Blocking;
use crate::context::AppContext;
use crate::error::{AppError, TaskStage, TimeoutStage};
use crate::memory::JournalEntry;
use crate::memory::fixtures::meta;
use crate::ports::Isolation;

/// A limit of `timeout.validation` that a test can pass on purpose.
const SHORT: Duration = Duration::from_millis(30);

async fn preview_in(context: AppContext) -> Result<ApprovalPreview, AppError> {
    GetApprovalPreview::new(context)
        .execute(&admin(), &meta(), input(2))
        .await
}

#[tokio::test]
async fn a_busy_gate_beyond_the_deadline_gives_web_009_before_the_transaction() {
    let fixture = valid_world();
    let short = Blocking::new(SHORT, Duration::from_secs(10));
    let context = fixture.context_with(one_permit(short));
    let far = Blocking::default().diff_deadline().unwrap();
    let held = context.diff_gate.acquire(far).await.unwrap();

    let error = preview_in(context).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Diff
        }
    ));
    assert!(fixture.store.journal().is_empty());
    drop(held);
}

#[tokio::test]
async fn a_failed_preview_task_gives_web_011_with_the_diff_stage() {
    let fixture = valid_world();
    let context = fixture.context_with(one_permit(Blocking::panicked()));

    let error = preview_in(context.clone()).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Diff
        }
    ));
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::RepeatableRead)]
    );
    assert_eq!(context.diff_gate.available(), 1);
}

#[tokio::test]
async fn the_preview_task_gets_only_the_rest_of_the_deadline() {
    let fixture = valid_world();
    let short = Blocking::new(SHORT, Duration::from_secs(10));
    let context = fixture.context_with(one_permit(short));
    // The read of the draft uses more time than the whole deadline.
    fixture
        .store
        .on_read_draft(|_| std::thread::sleep(SHORT.checked_mul(2).unwrap()));

    let error = preview_in(context.clone()).await.unwrap_err();

    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Diff
        }
    ));
    assert_eq!(context.diff_gate.available(), 1);
}
