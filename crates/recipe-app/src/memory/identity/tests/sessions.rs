//! Tests of `SessionRepository` on the in-memory transaction.

use recipe_domain::identity::Role;

use super::{IDLE, at, begin, csrf, new_session, new_user, session_id, store, user_id};
use crate::error::StorageError;
use crate::ports::Transaction as _;

#[tokio::test]
async fn insert_sets_the_times_from_the_transaction_and_the_idle_timeout() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    store.clock().advance(time::Duration::seconds(10));
    let mut tx = begin(&store);
    tx.sessions()
        .insert(&new_session(1, user_id(1), 0))
        .await
        .unwrap();
    let stored = tx.state().sessions.get(&session_id(1)).unwrap().clone();
    assert_eq!(stored.user_id, user_id(1));
    assert_eq!(stored.epoch, 0);
    assert_eq!(stored.created_at, at(10));
    assert_eq!(stored.last_seen_at, at(10));
    let idle = i64::try_from(IDLE.as_secs()).unwrap();
    assert_eq!(stored.expires_at, at(10_i64.checked_add(idle).unwrap()));
}

#[tokio::test]
async fn insert_for_an_unknown_user_fails_with_the_foreign_key() {
    let store = store();
    let mut tx = begin(&store);
    let error = tx
        .sessions()
        .insert(&new_session(1, user_id(9), 0))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::MissingReference { constraint } if constraint == "session_user_id_fkey"),
        "{error:?}"
    );
}

#[tokio::test]
async fn insert_of_a_known_id_fails_with_the_primary_key() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    let mut tx = begin(&store);
    tx.sessions()
        .insert(&new_session(1, user_id(1), 0))
        .await
        .unwrap();
    let error = tx
        .sessions()
        .insert(&new_session(1, user_id(1), 0))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == "session_pkey"),
        "{error:?}"
    );
}

#[tokio::test]
async fn delete_tells_whether_a_row_was_removed() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    store.seed_session(&new_session(1, user_id(1), 0));
    let mut tx = begin(&store);
    assert!(tx.sessions().delete(&session_id(1)).await.unwrap());
    assert!(!tx.sessions().delete(&session_id(1)).await.unwrap());
}

#[tokio::test]
async fn delete_all_of_user_removes_only_the_sessions_of_that_user() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    store.seed_user(&new_user(2, "bob", Role::Operator));
    store.seed_session(&new_session(1, user_id(1), 0));
    store.seed_session(&new_session(2, user_id(1), 0));
    store.seed_session(&new_session(3, user_id(2), 0));
    let mut tx = begin(&store);
    assert_eq!(
        tx.sessions().delete_all_of_user(user_id(1)).await.unwrap(),
        2
    );
    assert_eq!(
        tx.sessions().delete_all_of_user(user_id(1)).await.unwrap(),
        0
    );
    assert!(tx.state().sessions.contains_key(&session_id(3)));
}

#[test]
fn debug_of_a_new_session_hides_the_session_id_and_the_csrf_token() {
    let session = new_session(1, user_id(1), 0);
    let shown = format!("{session:?}");
    assert!(!shown.contains(session_id(1).as_str()), "{shown}");
    assert!(!shown.contains(&csrf(1).to_hex()), "{shown}");
    assert!(shown.contains("[redacted]"), "{shown}");
}
