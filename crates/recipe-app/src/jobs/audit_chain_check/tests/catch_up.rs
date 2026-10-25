//! The run kinds (ADR-0088 (b)): a `CatchUp` run skips after a newer check, a `Scheduled` run never.

use super::support::{
    AT_KEY, SEQ_KEY, at, events, rig, rig_with, run, run_kind, seq, set_checked_at, stored,
};
use crate::error::AppError;
use crate::jobs::RunKind;
use crate::jobs::audit_chain_check::AuditChainCheckSettings;
use crate::memory::{AuditChainMethod, ChainBreak, Fault, MaintenanceMethod, OperationsMethod};

#[tokio::test]
async fn a_catch_up_without_any_check_runs() {
    let rig = rig(3);
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("3"));
}

#[tokio::test]
async fn a_catch_up_runs_when_the_time_equals_the_value_of_the_start() {
    let rig = rig_with(3, AuditChainCheckSettings::default(), Some(at(0)));
    set_checked_at(&rig, at(0));
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("3"));
}

#[tokio::test]
async fn a_catch_up_runs_when_the_time_is_older_than_the_value_of_the_start() {
    let rig = rig_with(3, AuditChainCheckSettings::default(), Some(at(0)));
    set_checked_at(&rig, at(-5));
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("3"));
}

#[tokio::test]
async fn a_catch_up_skips_when_a_check_completed_after_a_start_without_a_check() {
    let rig = rig(3);
    set_checked_at(&rig, at(100));
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::Head), 0);
    assert_eq!(rig.maintenance.calls(MaintenanceMethod::RecordState), 0);
}

#[tokio::test]
async fn a_catch_up_skips_when_the_time_is_newer_than_the_value_of_the_start() {
    let rig = rig_with(3, AuditChainCheckSettings::default(), Some(at(0)));
    set_checked_at(&rig, at(100));
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::Head), 0);
    assert_eq!(stored(&rig, SEQ_KEY), None);
}

#[tokio::test]
async fn a_skipped_catch_up_reports_no_break_even_on_a_broken_chain() {
    let rig = rig(3);
    rig.chain.break_row(seq(2), ChainBreak::WrongEntryHash);
    set_checked_at(&rig, at(100));
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert!(events(&rig).is_empty());
}

#[tokio::test]
async fn a_scheduled_run_never_skips() {
    let rig = rig_with(3, AuditChainCheckSettings::default(), Some(at(0)));
    set_checked_at(&rig, at(100));
    run(&rig).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::Head), 1);
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("3"));
}

#[tokio::test]
async fn a_scheduled_run_does_not_read_the_time() {
    let rig = rig(3);
    run(&rig).await.unwrap();
    assert_eq!(rig.operations.calls(OperationsMethod::State), 0);
}

#[tokio::test]
async fn a_scheduled_run_on_a_broken_chain_reports_the_break_after_a_newer_check() {
    let rig = rig(3);
    rig.chain.break_row(seq(2), ChainBreak::WrongEntryHash);
    set_checked_at(&rig, at(100));
    run(&rig).await.unwrap_err();
    assert_eq!(events(&rig).len(), 1);
}

#[tokio::test]
async fn a_catch_up_reads_the_time_once() {
    let rig = rig(3);
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert_eq!(rig.operations.calls(OperationsMethod::State), 1);
}

#[tokio::test]
async fn a_catch_up_after_a_nightly_run_of_the_same_process_skips() {
    let rig = rig(3);
    run(&rig).await.unwrap();
    rig.chain.append_row(10);
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::Head), 1);
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("3"));
}

#[tokio::test]
async fn a_catch_up_that_cannot_read_the_time_returns_the_storage_error_and_checks_nothing() {
    let rig = rig(3);
    rig.operations
        .fail_on_call(OperationsMethod::State, 1, Fault::Unavailable);
    let error = run_kind(&rig, RunKind::CatchUp).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)));
    assert_eq!(rig.chain.calls(AuditChainMethod::Head), 0);
    assert_eq!(stored(&rig, AT_KEY), None);
}
