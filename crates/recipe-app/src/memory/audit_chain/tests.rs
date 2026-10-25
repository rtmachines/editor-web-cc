//! Tests of the in-memory `AuditChainReads` (task 3.10, design D6, test rows T6 and T14).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::setting::{SettingKey, SettingValue};

use super::{AuditChainMethod, ChainBreak, MemoryAuditChain};
use crate::memory::testkit::store;
use crate::memory::{Fault, JournalEntry, MemoryStore};
use crate::ports::{
    AuditChainReads as _, BatchOutcome, ChainBounds, ChainPosition, ChainSeq, Isolation,
};

/// The key of the anchor row (`data-model.md` 4.9 step 1).
const ANCHOR_KEY: &str = "audit.last_check_seq";

const MANY_BYTES: u64 = 1_000_000;

fn chain_of(rows: u32, bytes: u64) -> (MemoryStore, MemoryAuditChain) {
    let store = store();
    let chain = MemoryAuditChain::new(store.clone());
    for _ in 0..rows {
        chain.append_row(bytes);
    }
    (store, chain)
}

fn seq(n: i64) -> ChainSeq {
    ChainSeq::from(n)
}

/// A batch with a break at `first_break` and the last good row `last_clean` before it.
fn broken(first_break: i64, last_clean: i64) -> BatchOutcome {
    BatchOutcome::Broken {
        first_break: seq(first_break),
        last_clean: seq(last_clean),
    }
}

/// The row count and the last position of a clean batch.
fn parts(outcome: BatchOutcome) -> (u64, Option<ChainPosition>) {
    match outcome {
        BatchOutcome::Clean { rows, last } => (rows, last),
        BatchOutcome::Broken { .. } => (u64::MAX, None),
    }
}

async fn clean(
    chain: &MemoryAuditChain,
    after: &ChainPosition,
    head: i64,
    batch: u32,
) -> BatchOutcome {
    chain
        .check_batch(after, seq(head), batch, MANY_BYTES)
        .await
        .unwrap()
}

#[tokio::test]
async fn an_empty_chain_has_the_start_position() {
    let chain = MemoryAuditChain::new(store());
    assert_eq!(chain.head().await.unwrap().position, ChainPosition::START);
}

fn bounds(min: i64, max: i64) -> Option<ChainBounds> {
    Some(ChainBounds {
        min: seq(min),
        max: seq(max),
    })
}

/// Write the text of the anchor row as a stored state value.
fn plant_anchor(store: &MemoryStore, text: &str) {
    let key = SettingKey::parse(ANCHOR_KEY).unwrap();
    let value = SettingValue::parse(text).unwrap();
    store.update_state(|state, now| state.upsert_setting(key, value, now).unwrap());
}

#[tokio::test]
async fn an_empty_chain_has_no_bounds_and_the_anchor_zero() {
    let chain = MemoryAuditChain::new(store());
    let head = chain.head().await.unwrap();
    assert_eq!(head.bounds, None);
    assert_eq!(head.anchor, ChainSeq::ZERO);
}

#[tokio::test]
async fn a_clean_chain_has_the_bounds_one_and_the_head() {
    let (_store, chain) = chain_of(3, 10);
    let head = chain.head().await.unwrap();
    assert_eq!(head.bounds, bounds(1, 3));
    assert_eq!(head.position.seq, seq(3));
}

#[tokio::test]
async fn a_planted_row_at_zero_gives_the_minimum_zero_and_keeps_the_head() {
    let (_store, chain) = chain_of(3, 10);
    let before = chain.head().await.unwrap().position;
    chain.plant_row(seq(0), 10);
    let head = chain.head().await.unwrap();
    assert_eq!(head.bounds, bounds(0, 3));
    assert_eq!(head.position, before);
}

#[tokio::test]
async fn a_planted_row_below_zero_gives_a_negative_minimum_and_keeps_the_head() {
    let (_store, chain) = chain_of(3, 10);
    let before = chain.head().await.unwrap().position;
    chain.plant_row(seq(-3), 10);
    let head = chain.head().await.unwrap();
    assert_eq!(head.bounds, bounds(-3, 3));
    assert_eq!(head.position, before);
}

#[tokio::test]
async fn a_planted_row_above_the_head_gives_a_larger_maximum_and_keeps_the_head() {
    let (_store, chain) = chain_of(3, 10);
    let before = chain.head().await.unwrap().position;
    chain.plant_row(seq(9), 10);
    let head = chain.head().await.unwrap();
    assert_eq!(head.bounds, bounds(1, 9));
    assert_eq!(head.position, before);
}

#[tokio::test]
async fn a_row_appended_after_a_planted_row_continues_the_real_chain() {
    let (_store, chain) = chain_of(3, 10);
    chain.plant_row(seq(9), 10);
    let appended = chain.append_row(10);
    assert_eq!(appended.seq, seq(4));
    assert_eq!(chain.head().await.unwrap().position, appended);
}

#[tokio::test]
async fn the_anchor_is_read_from_the_state_key() {
    let (store, chain) = chain_of(1, 10);
    plant_anchor(&store, "41");
    assert_eq!(chain.head().await.unwrap().anchor, seq(41));
}

#[tokio::test]
async fn a_bad_anchor_text_is_a_storage_error() {
    let (store, chain) = chain_of(1, 10);
    for text in ["x", "-1", "01"] {
        plant_anchor(&store, text);
        let error = chain.head().await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::DB_008, "text: {text:?}");
    }
}

#[tokio::test]
async fn each_appended_row_moves_the_head() {
    let chain = MemoryAuditChain::new(store());
    let first = chain.append_row(10);
    let second = chain.append_row(10);
    assert_eq!(first.seq, seq(1));
    assert_eq!(second.seq, seq(2));
    assert!(first.hash.is_some());
    assert_ne!(first.hash, second.hash);
    assert_eq!(chain.head().await.unwrap().position, second);
}

#[tokio::test]
async fn an_empty_range_gives_a_clean_batch_without_rows() {
    let chain = MemoryAuditChain::new(store());
    let outcome = clean(&chain, &ChainPosition::START, 0, 10).await;
    assert_eq!(
        outcome,
        BatchOutcome::Clean {
            rows: 0,
            last: None
        }
    );
}

#[tokio::test]
async fn a_clean_chain_checks_in_one_batch() {
    let (_store, chain) = chain_of(3, 10);
    let head = chain.head().await.unwrap().position;
    let outcome = clean(&chain, &ChainPosition::START, 3, 10).await;
    assert_eq!(
        outcome,
        BatchOutcome::Clean {
            rows: 3,
            last: Some(head)
        }
    );
}

#[tokio::test]
async fn the_batch_stops_at_the_row_limit_and_the_next_batch_continues() {
    let (_store, chain) = chain_of(5, 10);
    let mut after = ChainPosition::START;
    let mut rows = Vec::new();
    for _ in 0..4 {
        let (count, last) = parts(clean(&chain, &after, 5, 2).await);
        rows.push(count);
        if let Some(position) = last {
            after = position;
        }
    }
    assert_eq!(rows, [2, 2, 1, 0]);
    assert_eq!(after.seq, seq(5));
}

#[tokio::test]
async fn the_batch_stops_at_the_byte_limit() {
    let (_store, chain) = chain_of(4, 100);
    let outcome = chain
        .check_batch(&ChainPosition::START, seq(4), 100, 250)
        .await
        .unwrap();
    let (rows, last) = parts(outcome);
    assert_eq!(rows, 2);
    assert_eq!(last.unwrap().seq, seq(2));
}

#[tokio::test]
async fn a_batch_holds_one_row_although_the_row_is_above_the_byte_limit() {
    let (_store, chain) = chain_of(2, 100);
    let outcome = chain
        .check_batch(&ChainPosition::START, seq(2), 100, 1)
        .await
        .unwrap();
    assert!(matches!(outcome, BatchOutcome::Clean { rows: 1, .. }));
}

#[tokio::test]
async fn a_batch_of_zero_rows_still_holds_one_row() {
    let (_store, chain) = chain_of(2, 10);
    let outcome = clean(&chain, &ChainPosition::START, 2, 0).await;
    assert!(matches!(outcome, BatchOutcome::Clean { rows: 1, .. }));
}

#[tokio::test]
async fn the_range_ends_at_the_head_argument() {
    let (_store, chain) = chain_of(5, 10);
    let outcome = clean(&chain, &ChainPosition::START, 3, 10).await;
    assert!(matches!(outcome, BatchOutcome::Clean { rows: 3, .. }));
}

#[tokio::test]
async fn a_wrong_entry_hash_is_the_first_break() {
    let (_store, chain) = chain_of(5, 10);
    assert!(chain.break_row(seq(4), ChainBreak::WrongEntryHash));
    assert!(chain.break_row(seq(2), ChainBreak::WrongEntryHash));
    let outcome = clean(&chain, &ChainPosition::START, 5, 10).await;
    assert_eq!(outcome, broken(2, 1));
}

#[tokio::test]
async fn a_wrong_previous_hash_is_a_break() {
    let (_store, chain) = chain_of(3, 10);
    assert!(chain.break_row(seq(3), ChainBreak::WrongPreviousHash));
    let outcome = clean(&chain, &ChainPosition::START, 3, 10).await;
    assert_eq!(outcome, broken(3, 2));
}

#[tokio::test]
async fn the_last_clean_row_is_the_row_before_the_break() {
    let (_store, chain) = chain_of(5, 10);
    assert!(chain.break_row(seq(3), ChainBreak::WrongEntryHash));
    let outcome = clean(&chain, &ChainPosition::START, 5, 10).await;
    assert_eq!(outcome, broken(3, 2));
}

#[tokio::test]
async fn the_last_clean_row_is_the_after_value_when_the_first_row_breaks() {
    let (_store, chain) = chain_of(5, 10);
    let (_rows, last) = parts(clean(&chain, &ChainPosition::START, 2, 10).await);
    let after = last.unwrap();
    assert!(chain.break_row(seq(3), ChainBreak::WrongEntryHash));
    let outcome = clean(&chain, &after, 5, 10).await;
    assert_eq!(outcome, broken(3, 2));
}

#[tokio::test]
async fn a_break_outside_the_batch_is_not_seen_yet() {
    let (_store, chain) = chain_of(4, 10);
    assert!(chain.break_row(seq(4), ChainBreak::WrongEntryHash));
    let outcome = clean(&chain, &ChainPosition::START, 4, 3).await;
    assert!(matches!(outcome, BatchOutcome::Clean { rows: 3, .. }));
}

#[tokio::test]
async fn breaking_a_missing_row_returns_false() {
    let (_store, chain) = chain_of(1, 10);
    assert!(!chain.break_row(seq(2), ChainBreak::WrongEntryHash));
}

#[tokio::test]
async fn the_hash_of_the_after_position_must_link_to_the_next_row() {
    let (_store, chain) = chain_of(3, 10);
    let (_rows, last) = parts(clean(&chain, &ChainPosition::START, 1, 10).await);
    let after = last.unwrap();
    assert!(matches!(
        clean(&chain, &after, 3, 10).await,
        BatchOutcome::Clean { rows: 2, .. }
    ));
    let wrong = ChainPosition {
        seq: after.seq,
        hash: ChainPosition::START.hash,
    };
    let outcome = clean(&chain, &wrong, 3, 10).await;
    assert_eq!(outcome, broken(2, 1));
}

#[tokio::test]
async fn set_head_replaces_the_head_and_none_restores_it() {
    let (_store, chain) = chain_of(2, 10);
    let last = chain.head().await.unwrap().position;
    let planted = ChainPosition {
        seq: seq(9),
        hash: last.hash,
    };
    chain.set_head(Some(planted));
    assert_eq!(chain.head().await.unwrap().position, planted);
    chain.set_head(None);
    assert_eq!(chain.head().await.unwrap().position, last);
}

#[tokio::test]
async fn the_batch_reads_the_rows_in_chain_seq_order() {
    let (_store, chain) = chain_of(3, 10);
    chain.plant_row(seq(5), 10);
    chain.plant_row(seq(4), 10);
    let outcome = clean(&chain, &ChainPosition::START, 5, 10).await;
    assert_eq!(outcome, broken(4, 3));
}

/// The number of rows in the table, planted rows included.
fn row_count(store: &MemoryStore) -> usize {
    store.update_state(|state, _now| state.chain.len())
}

#[tokio::test]
async fn a_planted_row_at_a_taken_sequence_is_refused_and_adds_no_row() {
    let (store, chain) = chain_of(3, 10);
    assert!(chain.plant_row(seq(9), 10));
    assert!(!chain.plant_row(seq(2), 10));
    assert!(!chain.plant_row(seq(9), 10));
    assert_eq!(row_count(&store), 4);
}

#[tokio::test]
async fn an_append_at_a_planted_sequence_adds_no_second_row() {
    let (store, chain) = chain_of(3, 10);
    assert!(chain.plant_row(seq(4), 10));
    chain.append_row(10);
    assert_eq!(row_count(&store), 4);
}

#[tokio::test]
async fn a_linked_row_with_a_gap_in_chain_seq_is_a_break() {
    let (_store, chain) = chain_of(2, 10);
    assert!(chain.plant_linked_row(seq(100), 10));
    let outcome = clean(&chain, &ChainPosition::START, 100, 10).await;
    assert_eq!(outcome, broken(100, 2));
}

#[tokio::test]
async fn a_lone_row_after_the_start_is_a_gap_break() {
    let chain = MemoryAuditChain::new(store());
    chain.plant_row(seq(3), 10);
    let outcome = clean(&chain, &ChainPosition::START, 3, 10).await;
    assert_eq!(outcome, broken(3, 0));
}

#[tokio::test]
async fn a_head_with_a_sequence_and_no_hash_is_permitted() {
    let (_store, chain) = chain_of(2, 10);
    let planted = ChainPosition {
        seq: seq(5),
        hash: None,
    };
    chain.set_head(Some(planted));
    assert_eq!(chain.head().await.unwrap().position, planted);
}

#[tokio::test]
async fn a_fault_on_the_second_batch_leaves_the_first_batch_checked() {
    let (_store, chain) = chain_of(4, 10);
    chain.fail_on_call(AuditChainMethod::CheckBatch, 2, Fault::StatementTimeout);
    let (_rows, last) = parts(clean(&chain, &ChainPosition::START, 4, 2).await);
    let after = last.unwrap();
    let error = chain
        .check_batch(&after, seq(4), 2, MANY_BYTES)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_002);
    assert!(matches!(
        clean(&chain, &after, 4, 2).await,
        BatchOutcome::Clean { rows: 2, .. }
    ));
}

#[tokio::test]
async fn the_head_has_its_own_fault_and_its_own_counter() {
    let (_store, chain) = chain_of(1, 10);
    chain.fail_from_call(AuditChainMethod::Head, 1, Fault::Unavailable);
    assert_eq!(chain.head().await.unwrap_err().code(), ErrorCode::DB_005);
    assert_eq!(chain.calls(AuditChainMethod::Head), 1);
    assert_eq!(chain.calls(AuditChainMethod::CheckBatch), 0);
    clean(&chain, &ChainPosition::START, 1, 1).await;
    chain.clear_faults();
    chain.head().await.unwrap();
}

#[tokio::test]
async fn a_call_reads_in_one_read_only_transaction() {
    let (store, chain) = chain_of(1, 10);
    chain.head().await.unwrap();
    assert_eq!(
        store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
}

#[tokio::test]
async fn a_begin_fault_of_the_store_fails_the_call() {
    let (store, chain) = chain_of(1, 10);
    store.fail_begin(Fault::Unavailable);
    assert_eq!(chain.head().await.unwrap_err().code(), ErrorCode::DB_005);
}
