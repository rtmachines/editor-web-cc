//! A chain without a break (`data-model.md` section 4.9).

use super::support::{AT_KEY, SEQ_KEY, at, events, rig, run, set_checked_at, stored, time_text};
use crate::memory::AuditChainMethod;

#[tokio::test]
async fn a_sound_chain_returns_ok_and_writes_no_security_event() {
    let rig = rig(5);
    run(&rig).await.unwrap();
    assert!(events(&rig).is_empty());
}

#[tokio::test]
async fn a_sound_chain_records_the_head_as_the_anchor() {
    let rig = rig(5);
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("5"));
}

#[tokio::test]
async fn a_sound_chain_records_the_end_time_of_the_check() {
    let rig = rig(5);
    rig.store.clock().set(at(90));
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig, AT_KEY), Some(time_text(at(90))));
}

#[tokio::test]
async fn an_empty_chain_is_sound_and_records_the_anchor_zero() {
    let rig = rig(0);
    run(&rig).await.unwrap();
    assert!(events(&rig).is_empty());
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("0"));
    assert!(stored(&rig, AT_KEY).is_some());
}

#[tokio::test]
async fn a_second_run_moves_the_anchor_to_the_new_head() {
    let rig = rig(3);
    run(&rig).await.unwrap();
    rig.chain.append_row(10);
    rig.chain.append_row(10);
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig, SEQ_KEY).as_deref(), Some("5"));
}

#[tokio::test]
async fn a_run_reads_the_head_once_and_ignores_an_old_check_time() {
    let rig = rig(3);
    set_checked_at(&rig, at(-10));
    run(&rig).await.unwrap();
    assert_eq!(rig.chain.calls(AuditChainMethod::Head), 1);
}
