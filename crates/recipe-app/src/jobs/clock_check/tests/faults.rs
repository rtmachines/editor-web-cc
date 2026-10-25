//! A fault of the read of the database time is a storage error (`reliability.md` 3.3).
//!
//! The job returns it. The supervisor logs it once at `ERROR` (ADR-0060 (q), ADR-0089 (d)).

use recipe_domain::error_codes::ErrorCode;

use super::support::{LogCapture, rig, run, seconds, set_host_offset};
use crate::error::AppError;
use crate::memory::{Fault, OperationsMethod};

#[tokio::test]
async fn a_fault_of_the_database_read_returns_the_storage_error() {
    let rig = rig();
    rig.operations
        .fail_on_call(OperationsMethod::DatabaseNow, 1, Fault::Unavailable);
    let error = run(&rig).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)));
    assert_eq!(error.code(), ErrorCode::DB_005);
}

#[tokio::test]
async fn a_fault_of_the_database_read_writes_no_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(7));
    rig.operations
        .fail_on_call(OperationsMethod::DatabaseNow, 1, Fault::StatementTimeout);
    run(&rig).await.unwrap_err();
    assert_eq!(logs.count(), 0);
}

#[tokio::test]
async fn the_run_after_a_fault_compares_the_clocks() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(7));
    rig.operations
        .fail_on_call(OperationsMethod::DatabaseNow, 1, Fault::Unavailable);
    run(&rig).await.unwrap_err();
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 1);
}
