//! Tests of T2 of the Basic check: `LoginAttemptStore::confirm_basic` on the in-memory store.

use recipe_domain::identity::Role;

use super::{at, hash_a, hash_b, ip, locks, new_user, store, typed, user_id};
use crate::memory::{Lock, LockMode, MemoryStore};
use crate::ports::{AttemptId, BasicChange, BasicConfirmation, LoginAttemptStore as _};

fn seeded(role: Role) -> (MemoryStore, AttemptId) {
    let store = store();
    store.seed_user(&new_user(1, "anna", role));
    let attempt = store.seed_attempt(&typed("anna"), ip(1), at(0), false);
    (store, attempt)
}

fn succeeded(store: &MemoryStore) -> bool {
    store.state().attempts.values().all(|row| row.succeeded)
}

#[tokio::test]
async fn it_marks_the_attempt_when_the_account_is_unchanged() {
    let (store, attempt) = seeded(Role::ExportClient);
    let answer = store
        .confirm_basic(attempt, user_id(1), &hash_a())
        .await
        .unwrap();
    assert_eq!(answer, BasicConfirmation::Confirmed);
    assert!(succeeded(&store));
    assert!(store.audit_entries().is_empty());
}

#[tokio::test]
async fn it_locks_the_user_row_for_no_key_update() {
    let (store, attempt) = seeded(Role::ExportClient);
    store
        .confirm_basic(attempt, user_id(1), &hash_a())
        .await
        .unwrap();
    assert_eq!(
        locks(&store),
        vec![Lock {
            table: "app_user",
            mode: LockMode::ForNoKeyUpdate,
            key: user_id(1).to_string(),
        }]
    );
}

#[tokio::test]
async fn it_reports_each_change_and_marks_nothing() {
    let cases = [
        BasicChange::NotExportClient,
        BasicChange::PasswordChanged,
        BasicChange::Inactive,
    ];
    for reason in cases {
        let (store, attempt) = seeded(Role::ExportClient);
        store.update_state(|state, _now| {
            let row = state.users.get_mut(&user_id(1)).unwrap();
            match reason {
                BasicChange::NotExportClient => row.role = Role::Operator,
                BasicChange::PasswordChanged => row.password_hash = hash_b(),
                _ => row.is_active = false,
            }
        });
        let answer = store
            .confirm_basic(attempt, user_id(1), &hash_a())
            .await
            .unwrap();
        assert_eq!(answer, BasicConfirmation::Changed { reason });
        assert!(!succeeded(&store), "{reason:?}");
    }
}

#[tokio::test]
async fn it_reports_a_missing_user() {
    let (store, attempt) = seeded(Role::ExportClient);
    let answer = store
        .confirm_basic(attempt, user_id(2), &hash_a())
        .await
        .unwrap();
    assert_eq!(
        answer,
        BasicConfirmation::Changed {
            reason: BasicChange::Missing
        }
    );
}

#[tokio::test]
async fn a_deleted_attempt_does_not_fail_the_check() {
    let (store, attempt) = seeded(Role::ExportClient);
    store.update_state(|state, _now| state.attempts.clear());
    let answer = store
        .confirm_basic(attempt, user_id(1), &hash_a())
        .await
        .unwrap();
    assert_eq!(answer, BasicConfirmation::Confirmed);
    assert!(store.state().attempts.is_empty());
}
