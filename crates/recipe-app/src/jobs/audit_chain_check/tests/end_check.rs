//! The end check after the last batch (ADR-0086 (d)).
//!
//! A scripted reader gives a clean batch that ends below the head. The memory chain cannot give one.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::Sha256;

use super::support::{
    AT_KEY, Rig, SEQ_KEY, ScriptedChain, chain_seq_of, events, rig, rig_over, run, seq, stored,
};
use crate::ports::{BatchOutcome, ChainBounds, ChainHead, ChainPosition};

const HEAD: i64 = 5;

fn hash(n: u8) -> Sha256 {
    Sha256::from([n; 32])
}

fn position(n: i64, hash_byte: u8) -> ChainPosition {
    ChainPosition {
        seq: seq(n),
        hash: Some(hash(hash_byte)),
    }
}

fn head(anchor: i64) -> ChainHead {
    ChainHead {
        position: position(HEAD, 5),
        bounds: Some(ChainBounds {
            min: seq(1),
            max: seq(HEAD),
        }),
        anchor: seq(anchor),
    }
}

fn clean(rows: u64, last: Option<ChainPosition>) -> BatchOutcome {
    BatchOutcome::Clean { rows, last }
}

fn scripted(anchor: i64, batches: Vec<BatchOutcome>) -> Rig {
    rig_over(ScriptedChain::new(head(anchor), batches))
}

#[tokio::test]
async fn an_end_below_the_head_names_the_end_plus_one() {
    let rig = scripted(0, vec![clean(3, Some(position(3, 3))), clean(0, None)]);
    assert_eq!(chain_seq_of(run(&rig).await), Some(4));
}

#[tokio::test]
async fn an_end_below_the_head_records_the_end() {
    let rig = scripted(0, vec![clean(3, Some(position(3, 3))), clean(0, None)]);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("3"));
    assert!(stored(&rig, AT_KEY).is_some());
}

#[tokio::test]
async fn an_end_below_the_anchor_records_the_anchor() {
    let rig = scripted(4, vec![clean(3, Some(position(3, 3))), clean(0, None)]);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("4"));
}

#[tokio::test]
async fn no_checked_row_names_one_and_records_zero() {
    let rig = scripted(0, vec![clean(0, None)]);
    assert_eq!(chain_seq_of(run(&rig).await), Some(1));
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("0"));
}

#[tokio::test]
async fn an_end_at_the_head_with_another_hash_names_the_head() {
    let rig = scripted(0, vec![clean(5, Some(position(HEAD, 6)))]);
    assert_eq!(chain_seq_of(run(&rig).await), Some(HEAD));
}

#[tokio::test]
async fn an_end_at_the_head_with_another_hash_records_the_head_minus_one() {
    let rig = scripted(0, vec![clean(5, Some(position(HEAD, 6)))]);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("4"));
}

#[tokio::test]
async fn an_anchor_at_the_head_is_not_lowered_by_a_hash_break_at_the_end() {
    let rig = scripted(HEAD, vec![clean(5, Some(position(HEAD, 6)))]);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("5"));
}

#[tokio::test]
async fn an_end_at_the_head_with_the_same_hash_is_no_break() {
    let rig = scripted(0, vec![clean(5, Some(position(HEAD, 5)))]);
    run(&rig).await.unwrap();
    assert!(events(&rig).is_empty());
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("5"));
}

#[tokio::test]
async fn a_head_with_a_sequence_above_zero_and_no_hash_is_a_break() {
    let rig = rig(3);
    rig.chain.set_head(Some(ChainPosition {
        seq: seq(3),
        hash: None,
    }));
    assert_eq!(chain_seq_of(run(&rig).await), Some(3));
    assert_eq!(events(&rig).len(), 1);
}

#[tokio::test]
async fn a_head_with_no_hash_records_the_head_minus_one() {
    let rig = rig(3);
    rig.chain.set_head(Some(ChainPosition {
        seq: seq(3),
        hash: None,
    }));
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("2"));
}

#[tokio::test]
async fn a_batch_that_ends_above_the_head_is_a_storage_fault() {
    let rig = scripted(0, vec![clean(6, Some(position(HEAD + 1, 5)))]);
    let error = run(&rig).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(events(&rig).is_empty());
}

#[tokio::test]
async fn a_batch_that_does_not_advance_is_a_storage_fault_and_not_a_loop() {
    let rig = scripted(0, vec![clean(1, Some(position(0, 1)))]);
    let error = run(&rig).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
}
