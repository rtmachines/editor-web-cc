//! Tests of T1 of `Login`: `LoginAttemptStore::begin_attempt` on the in-memory store.

use recipe_domain::identity::ThrottlePolicy;

use super::{at, ip, locks, store, typed};
use crate::error::StorageError;
use crate::memory::{Fault, JournalEntry, LOGIN_LOCK_TABLE, Lock, LockMode};
use crate::ports::{AttemptKey, Isolation, LoginAttemptStore as _};

/// The default window is 15 minutes, so 901 seconds is outside it.
const OUTSIDE: i64 = -901;

fn key(login: &str, address: u8) -> AttemptKey {
    AttemptKey {
        login: typed(login),
        client_ip: ip(address),
    }
}

fn budget(limit: u32) -> ThrottlePolicy {
    ThrottlePolicy {
        address_budget: limit,
        ..ThrottlePolicy::default()
    }
}

#[tokio::test]
async fn it_counts_the_failures_of_the_pair_and_of_the_address() {
    let store = store();
    store.seed_attempt(&typed("anna"), ip(1), at(-100), false);
    store.seed_attempt(&typed("anna"), ip(1), at(-50), false);
    store.seed_attempt(&typed("bob"), ip(1), at(-80), false);
    store.seed_attempt(&typed("anna"), ip(2), at(-70), false);
    let outcome = store
        .begin_attempt(&key("anna", 1), &ThrottlePolicy::default())
        .await
        .unwrap();
    assert_eq!(outcome.count.pair_failures, 2);
    assert_eq!(outcome.count.last_pair_failure, Some(at(-50)));
    assert_eq!(outcome.count.address_failures, 3);
    assert_eq!(outcome.count.oldest_address_failure, Some(at(-100)));
    assert_eq!(outcome.count.now, at(0));
}

#[tokio::test]
async fn it_inserts_one_failed_attempt_with_the_database_time() {
    let store = store();
    let outcome = store
        .begin_attempt(&key("anna", 1), &ThrottlePolicy::default())
        .await
        .unwrap();
    let id = outcome.attempt.unwrap();
    let state = store.state();
    assert_eq!(state.attempts.len(), 1);
    let row = state.attempts.get(&id).unwrap();
    assert_eq!(row.login, typed("anna"));
    assert_eq!(row.client_ip, ip(1));
    assert_eq!(row.attempted_at, at(0));
    assert!(!row.succeeded);
}

#[tokio::test]
async fn it_ignores_attempts_outside_the_window_and_successful_attempts() {
    let store = store();
    store.seed_attempt(&typed("anna"), ip(1), at(OUTSIDE), false);
    store.seed_attempt(&typed("bob"), ip(1), at(OUTSIDE), false);
    store.seed_attempt(&typed("anna"), ip(1), at(-10), true);
    let outcome = store
        .begin_attempt(&key("anna", 1), &ThrottlePolicy::default())
        .await
        .unwrap();
    assert_eq!(outcome.count.pair_failures, 0);
    assert_eq!(outcome.count.last_pair_failure, None);
    assert_eq!(outcome.count.address_failures, 0);
    assert_eq!(outcome.count.oldest_address_failure, None);
}

#[tokio::test]
async fn it_inserts_no_row_when_the_budget_of_the_address_is_reached() {
    let store = store();
    store.seed_attempt(&typed("anna"), ip(1), at(-30), false);
    store.seed_attempt(&typed("bob"), ip(1), at(-20), false);
    store.seed_attempt(&typed("carol"), ip(1), at(-10), false);
    let outcome = store
        .begin_attempt(&key("dave", 1), &budget(3))
        .await
        .unwrap();
    assert_eq!(outcome.attempt, None);
    assert_eq!(outcome.count.address_failures, 3);
    assert_eq!(store.state().attempts.len(), 3);
}

#[tokio::test]
async fn it_inserts_a_row_one_failure_below_the_budget() {
    let store = store();
    store.seed_attempt(&typed("anna"), ip(1), at(-30), false);
    store.seed_attempt(&typed("bob"), ip(1), at(-20), false);
    let outcome = store
        .begin_attempt(&key("dave", 1), &budget(3))
        .await
        .unwrap();
    assert!(outcome.attempt.is_some());
    assert_eq!(store.state().attempts.len(), 3);
}

#[tokio::test]
async fn the_journal_records_the_transaction_and_the_class_1_lock() {
    let store = store();
    store
        .begin_attempt(&key("anna", 1), &ThrottlePolicy::default())
        .await
        .unwrap();
    let lock = Lock {
        table: LOGIN_LOCK_TABLE,
        mode: LockMode::Advisory,
        key: "anna".to_owned(),
    };
    assert_eq!(
        store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadCommitted),
            JournalEntry::Lock(lock.clone())
        ]
    );
    assert_eq!(locks(&store), vec![lock]);
    assert_eq!(LOGIN_LOCK_TABLE, "advisory_class_1");
}

#[tokio::test]
async fn a_second_attempt_counts_the_row_of_the_first() {
    let store = store();
    let policy = ThrottlePolicy::default();
    let first = store.begin_attempt(&key("anna", 1), &policy).await.unwrap();
    let second = store.begin_attempt(&key("anna", 1), &policy).await.unwrap();
    assert_eq!(first.count.pair_failures, 0);
    assert_eq!(second.count.pair_failures, 1);
    assert_eq!(second.count.last_pair_failure, Some(at(0)));
    assert_ne!(first.attempt, second.attempt);
}

#[tokio::test]
async fn a_failed_begin_returns_the_fault_and_writes_nothing() {
    let store = store();
    store.fail_begin(Fault::Unavailable);
    let error = store
        .begin_attempt(&key("anna", 1), &ThrottlePolicy::default())
        .await
        .unwrap_err();
    assert!(matches!(error, StorageError::Unavailable), "{error:?}");
    assert!(store.state().attempts.is_empty());
}
