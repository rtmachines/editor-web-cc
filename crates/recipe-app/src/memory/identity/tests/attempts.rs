//! Tests of `AttemptRepository` on the in-memory transaction.

use std::time::Duration;

use super::{at, begin, ip, store, typed};
use crate::ports::{AttemptId, Transaction as _};

const WINDOW: Duration = Duration::from_secs(15 * 60);

#[tokio::test]
async fn mark_succeeded_sets_the_flag_of_one_attempt() {
    let store = store();
    let first = store.seed_attempt(&typed("anna"), ip(1), at(-10), false);
    let second = store.seed_attempt(&typed("anna"), ip(1), at(-5), false);
    let mut tx = begin(&store);
    tx.attempts().mark_succeeded(second).await.unwrap();
    let attempts = &tx.state().attempts;
    assert!(!attempts.get(&first).unwrap().succeeded);
    assert!(attempts.get(&second).unwrap().succeeded);
}

#[tokio::test]
async fn mark_succeeded_of_a_missing_attempt_is_ok_and_changes_nothing() {
    let store = store();
    let mut tx = begin(&store);
    tx.attempts()
        .mark_succeeded(AttemptId::from(99))
        .await
        .unwrap();
    assert!(store.state().attempts.is_empty());
}

#[tokio::test]
async fn delete_for_login_returns_the_newest_rows_and_the_full_count() {
    let store = store();
    let mut ids = Vec::new();
    for (index, seconds) in [-50_i64, -40, -30, -20, -10].into_iter().enumerate() {
        let address = ip(u8::try_from(index).unwrap());
        ids.push(store.seed_attempt(&typed("anna"), address, at(seconds), false));
    }
    let other = store.seed_attempt(&typed("bob"), ip(1), at(-5), false);
    let mut tx = begin(&store);
    let deleted = tx
        .attempts()
        .delete_for_login(&typed("anna"), 3)
        .await
        .unwrap();
    assert_eq!(deleted.count, 5);
    let newest: Vec<AttemptId> = deleted.newest.iter().map(|row| row.id).collect();
    assert_eq!(
        newest,
        [
            *ids.get(4).unwrap(),
            *ids.get(3).unwrap(),
            *ids.get(2).unwrap()
        ]
    );
    let first = deleted.newest.first().unwrap();
    assert_eq!(first.attempted_at, at(-10));
    assert_eq!(first.client_ip, ip(4));
    assert!(!first.succeeded);
    let left: Vec<AttemptId> = tx.state().attempts.keys().copied().collect();
    assert_eq!(left, [other]);
}

#[tokio::test]
async fn delete_for_login_of_a_login_without_rows_gives_an_empty_result() {
    let store = store();
    let mut tx = begin(&store);
    let deleted = tx
        .attempts()
        .delete_for_login(&typed("anna"), 100)
        .await
        .unwrap();
    assert_eq!(deleted.count, 0);
    assert!(deleted.newest.is_empty());
}

#[tokio::test]
async fn throttle_state_counts_failures_of_the_window_over_every_address() {
    let store = store();
    store.seed_attempt(&typed("anna"), ip(1), at(-600), false);
    store.seed_attempt(&typed("anna"), ip(2), at(-300), false);
    store.seed_attempt(&typed("anna"), ip(1), at(-60), true);
    store.seed_attempt(&typed("anna"), ip(1), at(-901), false);
    store.seed_attempt(&typed("bob"), ip(1), at(-30), false);
    let mut tx = begin(&store);
    let state = tx
        .attempts()
        .throttle_state(&typed("anna"), WINDOW)
        .await
        .unwrap();
    assert_eq!(state.failures, 2);
    assert_eq!(state.last_failure, Some(at(-300)));
}

#[tokio::test]
async fn throttle_state_of_a_quiet_login_is_empty() {
    let store = store();
    let mut tx = begin(&store);
    let state = tx
        .attempts()
        .throttle_state(&typed("anna"), WINDOW)
        .await
        .unwrap();
    assert_eq!(state.failures, 0);
    assert_eq!(state.last_failure, None);
}
