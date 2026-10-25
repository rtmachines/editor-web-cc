//! A break inside a batch: the security event, the error, and the position of the break.

use recipe_domain::audit::SecurityEventKind;
use recipe_domain::error_codes::ErrorCode;

use super::support::{AT_KEY, SEQ_KEY, chain_seq_of, events, request_id, rig, run, seq, stored};
use crate::error::{AppError, AuditError};
use crate::memory::ChainBreak;

#[tokio::test]
async fn a_wrong_entry_hash_returns_the_break_with_its_chain_seq() {
    let rig = rig(5);
    rig.chain.break_row(seq(3), ChainBreak::WrongEntryHash);
    let error = run(&rig).await.unwrap_err();
    assert!(matches!(
        error,
        AppError::Audit(AuditError::ChainBroken { chain_seq: 3 })
    ));
    assert_eq!(error.code(), ErrorCode::AUD_002);
}

#[tokio::test]
async fn a_wrong_previous_hash_returns_the_break_with_its_chain_seq() {
    let rig = rig(5);
    rig.chain.break_row(seq(4), ChainBreak::WrongPreviousHash);
    assert_eq!(chain_seq_of(run(&rig).await), Some(4));
}

#[tokio::test]
async fn the_first_row_can_break() {
    let rig = rig(3);
    rig.chain.break_row(seq(1), ChainBreak::WrongEntryHash);
    assert_eq!(chain_seq_of(run(&rig).await), Some(1));
}

#[tokio::test]
async fn the_job_names_the_first_break_only() {
    let rig = rig(6);
    rig.chain.break_row(seq(5), ChainBreak::WrongEntryHash);
    rig.chain.break_row(seq(2), ChainBreak::WrongEntryHash);
    assert_eq!(chain_seq_of(run(&rig).await), Some(2));
}

#[tokio::test]
async fn a_break_writes_one_security_event_with_the_request_id() {
    let rig = rig(5);
    rig.chain.break_row(seq(3), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    let written = events(&rig);
    assert_eq!(written.len(), 1);
    let event = written.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::AuditChainBroken);
    assert_eq!(event.request_id(), request_id());
}

#[tokio::test]
async fn a_chain_that_stays_broken_writes_a_new_event_at_each_run() {
    let rig = rig(5);
    rig.chain.break_row(seq(3), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    run(&rig).await.unwrap_err();
    assert_eq!(events(&rig).len(), 2);
}

#[tokio::test]
async fn a_break_still_records_the_anchor_and_the_time_of_the_completed_check() {
    let rig = rig(5);
    rig.chain.break_row(seq(3), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("2"));
    assert!(stored(&rig, AT_KEY).is_some());
}

#[tokio::test]
async fn the_error_text_names_the_chain_seq_and_holds_no_hash() {
    let rig = rig(5);
    rig.chain.break_row(seq(3), ChainBreak::WrongEntryHash);
    let error = run(&rig).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Audit entry 3 does not match the hash chain. Contact the administrator."
    );
}
