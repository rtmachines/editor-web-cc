//! The batches of the check: the limits by rows and by bytes, and the hand-over between batches.

use super::support::{
    AT_KEY, ROW_BYTES, SEQ_KEY, chain_seq_of, events, rig, rig_with, run, seq, stored,
};
use crate::jobs::audit_chain_check::AuditChainCheckSettings;
use crate::memory::{AuditChainMethod, ChainBreak};

fn limits(batch_rows: u32, batch_bytes: u64) -> AuditChainCheckSettings {
    AuditChainCheckSettings {
        batch_rows,
        batch_bytes,
    }
}

#[tokio::test]
async fn a_chain_of_25_rows_with_a_batch_of_10_rows_needs_3_batches() {
    let rig = rig_with(25, limits(10, u64::MAX), None);
    run(&rig).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 3);
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("25"));
}

#[tokio::test]
async fn a_chain_of_exactly_two_full_batches_needs_2_batches() {
    let rig = rig_with(20, limits(10, u64::MAX), None);
    run(&rig).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 2);
}

#[tokio::test]
async fn a_small_chain_needs_one_batch() {
    let rig = rig(5);
    run(&rig).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 1);
}

#[tokio::test]
async fn an_empty_chain_reads_no_batch() {
    let rig = rig(0);
    run(&rig).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 0);
}

#[tokio::test]
async fn the_limit_in_bytes_ends_a_batch_before_the_limit_in_rows() {
    let rig = rig_with(5, limits(10_000, ROW_BYTES * 2), None);
    run(&rig).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 3);
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("5"));
}

#[tokio::test]
async fn a_limit_in_bytes_below_one_row_still_checks_one_row_per_batch() {
    let rig = rig_with(3, limits(10_000, 1), None);
    run(&rig).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 3);
}

#[tokio::test]
async fn the_second_batch_continues_from_the_hash_of_the_first() {
    let rig = rig_with(25, limits(10, u64::MAX), None);
    rig.chain.break_row(seq(11), ChainBreak::WrongPreviousHash);
    assert_eq!(chain_seq_of(run(&rig).await), Some(11));
}

#[tokio::test]
async fn a_break_in_the_last_batch_stops_the_check_there() {
    let rig = rig_with(25, limits(10, u64::MAX), None);
    rig.chain.break_row(seq(23), ChainBreak::WrongEntryHash);
    assert_eq!(chain_seq_of(run(&rig).await), Some(23));
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 3);
}

#[tokio::test]
async fn a_break_in_the_second_batch_reads_no_third_batch() {
    let rig = rig_with(25, limits(10, u64::MAX), None);
    rig.chain.break_row(seq(15), ChainBreak::WrongEntryHash);
    assert_eq!(chain_seq_of(run(&rig).await), Some(15));
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 2);
}

#[tokio::test]
async fn a_break_in_the_second_batch_records_the_row_before_it() {
    let rig = rig_with(25, limits(10, u64::MAX), None);
    rig.chain.break_row(seq(15), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("14"));
}

#[tokio::test]
async fn a_break_on_the_first_row_of_the_second_batch_records_the_end_of_the_first() {
    let rig = rig_with(25, limits(10, u64::MAX), None);
    rig.chain.break_row(seq(11), ChainBreak::WrongEntryHash);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("10"));
}

#[tokio::test]
async fn a_sound_check_of_several_batches_records_the_time_and_writes_no_event() {
    let rig = rig_with(25, limits(10, u64::MAX), None);
    run(&rig).await.unwrap();
    assert!(stored(&rig, AT_KEY).is_some());
    assert!(events(&rig).is_empty());
}
