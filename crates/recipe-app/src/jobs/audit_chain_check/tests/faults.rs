//! A storage fault stops the check. The check is then not completed, so it records nothing.

use recipe_domain::error_codes::ErrorCode;

use super::support::{AT_KEY, SEQ_KEY, events, rig_with, run, seq, stored};
use crate::error::AppError;
use crate::jobs::audit_chain_check::AuditChainCheckSettings;
use crate::memory::{AuditChainMethod, ChainBreak, Fault, MaintenanceMethod};

fn small_batches() -> AuditChainCheckSettings {
    AuditChainCheckSettings {
        batch_rows: 10,
        batch_bytes: u64::MAX,
    }
}

#[tokio::test]
async fn a_fault_of_the_head_read_returns_the_storage_error() {
    let rig = rig_with(3, AuditChainCheckSettings::default(), None);
    rig.chain
        .fail_on_call(AuditChainMethod::Head, 1, Fault::Unavailable);
    let error = run(&rig).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)));
    assert_eq!(error.code(), ErrorCode::DB_005);
}

#[tokio::test]
async fn a_fault_of_the_head_read_writes_no_event_and_no_state_key() {
    let rig = rig_with(3, AuditChainCheckSettings::default(), None);
    rig.chain
        .fail_on_call(AuditChainMethod::Head, 1, Fault::StatementTimeout);
    run(&rig).await.unwrap_err();
    assert!(events(&rig).is_empty());
    assert_eq!(rig.maintenance.calls(MaintenanceMethod::RecordState), 0);
}

#[tokio::test]
async fn a_fault_of_a_later_batch_records_nothing() {
    let rig = rig_with(25, small_batches(), None);
    rig.chain
        .fail_on_call(AuditChainMethod::CheckBatch, 2, Fault::StatementTimeout);
    let error = run(&rig).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_002);
    assert_eq!(stored(&rig, SEQ_KEY), None);
    assert_eq!(stored(&rig, AT_KEY), None);
    assert!(events(&rig).is_empty());
}

#[tokio::test]
async fn a_fault_of_the_event_write_returns_the_storage_error_and_records_nothing() {
    let rig = rig_with(3, AuditChainCheckSettings::default(), None);
    rig.chain.break_row(seq(2), ChainBreak::WrongEntryHash);
    rig.sink.fail_with(Fault::Unavailable);
    let error = run(&rig).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)));
    assert_eq!(rig.maintenance.calls(MaintenanceMethod::RecordState), 0);
}

#[tokio::test]
async fn the_event_is_written_before_the_state_keys() {
    let rig = rig_with(3, AuditChainCheckSettings::default(), None);
    rig.chain.break_row(seq(2), ChainBreak::WrongEntryHash);
    rig.maintenance
        .fail_on_call(MaintenanceMethod::RecordState, 1, Fault::Unavailable);
    run(&rig).await.unwrap_err();
    assert_eq!(events(&rig).len(), 1);
}
