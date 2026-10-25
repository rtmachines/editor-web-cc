//! The rules of step 1 (ADR-0086 (c)). The first rule that applies names the `chain_seq`.

use recipe_domain::export::Sha256;

use super::support::{Rig, SEQ_KEY, chain_seq_of, events, rig, run, seq, set_anchor, stored};
use crate::memory::AuditChainMethod;
use crate::ports::{AuditChainReads, ChainPosition};

fn head_at(n: i64) -> ChainPosition {
    ChainPosition {
        seq: seq(n),
        hash: Some(Sha256::from([9_u8; 32])),
    }
}

#[tokio::test]
async fn rule_1_a_minimum_below_one_names_the_minimum() {
    let rig = rig(3);
    rig.chain.plant_row(seq(-2), 10);
    assert_eq!(chain_seq_of(run(&rig).await), Some(-2));
}

#[tokio::test]
async fn rule_1_a_minimum_of_zero_names_zero() {
    let rig = rig(3);
    rig.chain.plant_row(seq(0), 10);
    assert_eq!(chain_seq_of(run(&rig).await), Some(0));
}

#[tokio::test]
async fn rule_2_a_maximum_above_the_head_names_the_maximum() {
    let rig = rig(3);
    rig.chain.plant_row(seq(7), 10);
    assert_eq!(chain_seq_of(run(&rig).await), Some(7));
}

#[tokio::test]
async fn rule_2_a_head_before_the_last_row_names_the_last_row() {
    let rig = rig(5);
    rig.chain.set_head(Some(head_at(3)));
    assert_eq!(chain_seq_of(run(&rig).await), Some(5));
}

#[tokio::test]
async fn rule_1_wins_over_rule_2() {
    let rig = rig(3);
    rig.chain.plant_row(seq(0), 10);
    rig.chain.plant_row(seq(9), 10);
    assert_eq!(chain_seq_of(run(&rig).await), Some(0));
}

#[tokio::test]
async fn rule_3_a_head_below_the_anchor_names_the_head_plus_one() {
    let rig = rig(3);
    set_anchor(&rig, 5);
    assert_eq!(chain_seq_of(run(&rig).await), Some(4));
}

#[tokio::test]
async fn rule_3_a_head_equal_to_the_anchor_is_no_break() {
    let rig = rig(3);
    set_anchor(&rig, 3);
    run(&rig).await.unwrap();
}

#[tokio::test]
async fn rule_2_wins_over_rule_3() {
    let rig = rig(3);
    set_anchor(&rig, 5);
    rig.chain.plant_row(seq(8), 10);
    assert_eq!(chain_seq_of(run(&rig).await), Some(8));
}

/// Assert that the run named `expected`, recorded the anchor `anchor`, and read no batch.
async fn assert_break_without_batch(rig: &Rig, expected: i64, anchor: i64) {
    assert_eq!(chain_seq_of(run(rig).await), Some(expected));
    assert_eq!(stored(rig, SEQ_KEY), Some(anchor.to_string()));
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 0);
}

#[tokio::test]
async fn rule_4_a_head_above_zero_and_an_empty_table_name_one() {
    let rig = rig(0);
    rig.chain.set_head(Some(head_at(3)));
    set_anchor(&rig, 2);
    assert_break_without_batch(&rig, 1, 2).await;
}

#[tokio::test]
async fn rule_4_a_minimum_above_one_names_one() {
    let rig = rig(0);
    rig.chain.plant_row(seq(2), 10);
    rig.chain.plant_row(seq(3), 10);
    rig.chain.set_head(Some(head_at(3)));
    assert_eq!(chain_seq_of(run(&rig).await), Some(1));
}

#[tokio::test]
async fn rule_5_a_head_after_the_last_row_names_the_maximum_plus_one() {
    let rig = rig(3);
    rig.chain.set_head(Some(head_at(5)));
    set_anchor(&rig, 2);
    assert_break_without_batch(&rig, 4, 2).await;
}

#[tokio::test]
async fn rule_3_wins_over_rule_4() {
    let rig = rig(0);
    rig.chain.set_head(Some(head_at(3)));
    set_anchor(&rig, 6);
    assert_eq!(chain_seq_of(run(&rig).await), Some(4));
}

/// Rules 3 and 5 both apply. The head 5 is below the anchor 7 and the last row is 3.
/// Rule 3 names 6 and rule 5 would name 4.
#[tokio::test]
async fn rule_3_wins_over_rule_5() {
    let rig = rig(3);
    rig.chain.set_head(Some(head_at(5)));
    set_anchor(&rig, 7);
    assert_eq!(chain_seq_of(run(&rig).await), Some(6));
}

/// Rules 4 and 5 both apply. The first row is 2 and the last row is 3, but the head is 5.
/// Rule 4 names 1 and rule 5 would name 4.
#[tokio::test]
async fn rule_4_wins_over_rule_5() {
    let rig = rig(0);
    rig.chain.plant_row(seq(2), 10);
    rig.chain.plant_row(seq(3), 10);
    rig.chain.set_head(Some(head_at(5)));
    assert_eq!(chain_seq_of(run(&rig).await), Some(1));
}

#[tokio::test]
async fn an_empty_table_with_the_head_zero_is_no_break() {
    let rig = rig(0);
    run(&rig).await.unwrap();
    assert!(events(&rig).is_empty());
}

/// An empty table with the head 0 and a hash is no break in step 1 (ADR-0086 (c)).
/// The end check then finds another hash at `H` and names 0. The anchor stays 0 (ADR-0086 (e)).
#[tokio::test]
async fn an_empty_table_with_the_head_zero_and_a_hash_breaks_only_in_the_end_check() {
    use crate::jobs::audit_chain_check::rules::step_one;
    let rig = rig(0);
    let head = ChainPosition {
        seq: seq(0),
        hash: Some(Sha256::from([9_u8; 32])),
    };
    rig.chain.set_head(Some(head));
    let read = rig.chain.head().await.unwrap();
    assert_eq!(step_one(&read).unwrap(), None);
    assert_break_without_batch(&rig, 0, 0).await;
}

#[tokio::test]
async fn a_break_in_step_1_reads_no_batch() {
    let rig = rig(3);
    rig.chain.plant_row(seq(7), 10);
    run(&rig).await.unwrap_err();
    assert_eq!(rig.chain.calls(AuditChainMethod::CheckBatch), 0);
}

#[tokio::test]
async fn a_break_in_step_1_records_the_anchor_unchanged() {
    let rig = rig(3);
    set_anchor(&rig, 2);
    rig.chain.plant_row(seq(7), 10);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("2"));
}

#[tokio::test]
async fn a_break_in_step_1_with_a_missing_anchor_records_zero() {
    let rig = rig(3);
    rig.chain.plant_row(seq(0), 10);
    run(&rig).await.unwrap_err();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("0"));
}

#[tokio::test]
async fn next_seq_refuses_an_overflow() {
    use crate::jobs::audit_chain_check::rules::next_seq;
    assert_eq!(next_seq(seq(4)).unwrap(), seq(5));
    assert!(next_seq(seq(i64::MAX)).is_err());
}
