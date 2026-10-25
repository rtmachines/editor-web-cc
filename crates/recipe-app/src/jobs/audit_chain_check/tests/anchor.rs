//! The anchor `audit.last_check_seq` and the time `audit.last_check_at` (ADR-0086 (e)).

use recipe_domain::export::Sha256;

use super::support::{
    AT_KEY, ROW_BYTES, SEQ_KEY, at, chain_seq_of, rig, rig_with_moving_clock, run, seq, set_anchor,
    stored, time_text,
};
use crate::error::AppError;
use crate::memory::{ChainBreak, Fault, MaintenanceMethod};
use crate::ports::ChainPosition;

#[tokio::test]
async fn rows_1_2_and_100_with_the_head_100_record_2_and_not_99() {
    let rig = rig(2);
    rig.chain.plant_linked_row(seq(100), ROW_BYTES);
    rig.chain.set_head(Some(ChainPosition {
        seq: seq(100),
        hash: Some(Sha256::from([9_u8; 32])),
    }));
    assert_eq!(chain_seq_of(run(&rig).await), Some(100));
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("2"));
}

#[tokio::test]
async fn a_break_in_a_batch_records_the_row_before_the_break() {
    let rig = rig(6);
    rig.chain.break_row(seq(4), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("3"));
}

#[tokio::test]
async fn a_break_on_the_first_row_records_zero() {
    let rig = rig(3);
    rig.chain.break_row(seq(1), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("0"));
}

#[tokio::test]
async fn a_break_below_the_anchor_keeps_the_anchor() {
    let rig = rig(6);
    set_anchor(&rig, 5);
    rig.chain.break_row(seq(2), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("5"));
}

#[tokio::test]
async fn a_break_above_the_anchor_raises_the_anchor_to_the_row_before_the_break() {
    let rig = rig(6);
    set_anchor(&rig, 1);
    rig.chain.break_row(seq(5), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("4"));
}

#[tokio::test]
async fn a_sound_check_raises_the_anchor_to_the_head() {
    let rig = rig(6);
    set_anchor(&rig, 2);
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("6"));
}

#[tokio::test]
async fn the_time_is_the_clock_at_the_end_of_the_check() {
    let rig = rig(3);
    rig.store.clock().set(at(500));
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig, AT_KEY), Some(time_text(at(500))));
}

#[tokio::test]
async fn a_sound_check_records_the_later_instant_when_the_clock_moves_during_the_run() {
    let rig = rig_with_moving_clock(3, 100);
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig, AT_KEY), Some(time_text(at(100))));
}

#[tokio::test]
async fn a_break_records_the_later_instant_when_the_clock_moves_during_the_run() {
    let rig = rig_with_moving_clock(3, 100);
    rig.chain.break_row(seq(2), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, AT_KEY), Some(time_text(at(100))));
}

#[tokio::test]
async fn a_sound_check_writes_exactly_two_state_keys() {
    let rig = rig(3);
    run(&rig).await.unwrap();
    assert_eq!(rig.maintenance.calls(MaintenanceMethod::RecordState), 2);
}

#[tokio::test]
async fn a_failed_write_of_the_time_leaves_the_anchor_written_and_the_time_missing() {
    let rig = rig(3);
    rig.maintenance
        .fail_on_call(MaintenanceMethod::RecordState, 2, Fault::Unavailable);
    let error = run(&rig).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)));
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("3"));
    assert_eq!(stored(&rig, AT_KEY), None);
}

#[tokio::test]
async fn a_failed_write_of_the_anchor_writes_no_time() {
    let rig = rig(3);
    rig.maintenance
        .fail_on_call(MaintenanceMethod::RecordState, 1, Fault::Unavailable);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY), None);
    assert_eq!(stored(&rig, AT_KEY), None);
    assert_eq!(rig.maintenance.calls(MaintenanceMethod::RecordState), 1);
}

#[tokio::test]
async fn a_failed_write_of_the_anchor_after_a_break_returns_the_storage_error() {
    let rig = rig(3);
    rig.chain.break_row(seq(2), ChainBreak::WrongEntryHash);
    rig.maintenance
        .fail_on_call(MaintenanceMethod::RecordState, 1, Fault::Unavailable);
    let error = run(&rig).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)));
    assert_eq!(super::support::events(&rig).len(), 1);
}
