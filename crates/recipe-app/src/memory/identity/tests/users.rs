//! Tests of `UserRepository` on the in-memory transaction.

use recipe_domain::identity::Role;
use serde_json::Value;

use super::{at, begin, hash_a, hash_b, locks, login, new_user, store, text, user_id};
use crate::error::{APP_USER_LOGIN_UNIQUE, LockFailure, StorageError};
use crate::memory::{Fault, FaultPoint, Lock, LockMode};
use crate::ports::Transaction as _;

fn without_hash(image: &Value) -> bool {
    image.get("password_hash").is_none()
}

#[tokio::test]
async fn find_by_login_returns_the_row_and_takes_no_lock() {
    let store = store();
    let seeded = store.seed_user(&new_user(1, "anna", Role::Operator));
    let mut tx = begin(&store);
    let found = tx
        .users()
        .find_by_login(&login("anna"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, seeded.id);
    assert_eq!(found.password_hash, hash_a());
    assert_eq!(found.role, Role::Operator);
    let missing = tx.users().find_by_login(&login("bob")).await.unwrap();
    assert!(missing.is_none());
    assert_eq!(locks(&store), Vec::new());
}

#[tokio::test]
async fn lock_by_login_takes_for_no_key_update_and_gives_the_image_without_the_hash() {
    let store = store();
    let seeded = store.seed_user(&new_user(1, "anna", Role::Operator));
    let mut tx = begin(&store);
    let locked = tx
        .users()
        .lock_by_login(&login("anna"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(locked.state.id, seeded.id);
    assert_eq!(
        locks(&store),
        vec![Lock {
            table: "app_user",
            mode: LockMode::ForNoKeyUpdate,
            key: seeded.id.to_string(),
        }]
    );
    let image = locked.before.as_value();
    assert!(without_hash(image));
    assert_eq!(image["id"], seeded.id.to_string());
    assert_eq!(image["login"], "anna");
    assert_eq!(image["role"], "operator");
    assert_eq!(image["is_active"], true);
    assert_eq!(image["session_epoch"], 0);
    assert_eq!(image["must_change_password"], false);
    assert_eq!(image["deactivated_at"], Value::Null);
    assert_eq!(image["created_at"], text(at(0)));
    assert_eq!(image["updated_at"], text(at(0)));
}

#[tokio::test]
async fn lock_by_login_of_an_unknown_login_gives_none_and_no_lock() {
    let store = store();
    let mut tx = begin(&store);
    let locked = tx.users().lock_by_login(&login("anna")).await.unwrap();
    assert!(locked.is_none());
    assert_eq!(locks(&store), Vec::new());
}

#[tokio::test]
async fn lock_administrator_guard_journals_the_advisory_lock_of_class_4_and_key_0() {
    let store = store();
    let mut tx = begin(&store);
    tx.users().lock_administrator_guard().await.unwrap();
    assert_eq!(
        locks(&store),
        vec![Lock {
            table: "advisory_class_4",
            mode: LockMode::Advisory,
            key: "0".to_owned(),
        }]
    );
}

#[tokio::test]
async fn a_lock_timeout_at_the_guard_lock_gives_the_lock_failure_and_no_journal_line() {
    let store = store();
    store.fail_next(FaultPoint::LockAdministratorGuard, Fault::LockTimeout, 1);
    let mut tx = begin(&store);
    let error = tx.users().lock_administrator_guard().await.unwrap_err();
    assert!(
        matches!(
            error,
            StorageError::Deadlock {
                cause: LockFailure::LockTimeout
            }
        ),
        "{error:?}"
    );
    assert_eq!(locks(&store), Vec::new());
    // The fault fires once. The next call takes the lock.
    tx.users().lock_administrator_guard().await.unwrap();
    assert_eq!(locks(&store).len(), 1);
}

#[tokio::test]
async fn count_active_administrators_ignores_inactive_users_and_other_roles() {
    let store = store();
    store.seed_user(&new_user(1, "admin-a", Role::Administrator));
    store.seed_user(&new_user(2, "admin-b", Role::Administrator));
    store.seed_user(&new_user(3, "engineer", Role::ProcessEngineer));
    let mut tx = begin(&store);
    assert_eq!(tx.users().count_active_administrators().await.unwrap(), 2);
    tx.users().deactivate(user_id(2)).await.unwrap();
    assert_eq!(tx.users().count_active_administrators().await.unwrap(), 1);
}

#[tokio::test]
async fn insert_creates_an_active_user_with_epoch_zero_and_returns_the_image() {
    let store = store();
    let mut tx = begin(&store);
    let mut user = new_user(7, "carol", Role::ProcessEngineer);
    user.must_change_password = true;
    let image = tx.users().insert(&user).await.unwrap();
    let value = image.as_value();
    assert!(without_hash(value));
    assert_eq!(value["id"], user_id(7).to_string());
    assert_eq!(value["login"], "carol");
    assert_eq!(value["role"], "process_engineer");
    assert_eq!(value["is_active"], true);
    assert_eq!(value["session_epoch"], 0);
    assert_eq!(value["must_change_password"], true);
    assert_eq!(value["created_at"], text(at(0)));
    let found = tx
        .users()
        .find_by_login(&login("carol"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.password_hash, hash_a());
}

#[tokio::test]
async fn insert_of_a_duplicate_login_fails_with_the_unique_constraint() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    let mut tx = begin(&store);
    let error = tx
        .users()
        .insert(&new_user(2, "anna", Role::Operator))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == APP_USER_LOGIN_UNIQUE),
        "{error:?}"
    );
}

#[tokio::test]
async fn set_role_changes_the_role_and_raises_the_epoch() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    store.clock().advance(time::Duration::seconds(60));
    let mut tx = begin(&store);
    let image = tx
        .users()
        .set_role(user_id(1), Role::Administrator)
        .await
        .unwrap();
    let value = image.as_value();
    assert!(without_hash(value));
    assert_eq!(value["role"], "administrator");
    assert_eq!(value["session_epoch"], 1);
    assert_eq!(value["is_active"], true);
    assert_eq!(value["created_at"], text(at(0)));
    assert_eq!(value["updated_at"], text(at(60)));
}

#[tokio::test]
async fn deactivate_clears_the_active_flag_and_raises_the_epoch() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    store.clock().advance(time::Duration::seconds(60));
    let mut tx = begin(&store);
    let image = tx.users().deactivate(user_id(1)).await.unwrap();
    let value = image.as_value();
    assert!(without_hash(value));
    assert_eq!(value["is_active"], false);
    assert_eq!(value["deactivated_at"], text(at(60)));
    assert_eq!(value["session_epoch"], 1);
    assert_eq!(value["role"], "operator");
}

#[tokio::test]
async fn activate_sets_the_active_flag_and_keeps_the_epoch() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    let mut first = begin(&store);
    first.users().deactivate(user_id(1)).await.unwrap();
    let mut tx = first;
    let image = tx.users().activate(user_id(1)).await.unwrap();
    let value = image.as_value();
    assert!(without_hash(value));
    assert_eq!(value["is_active"], true);
    assert_eq!(value["deactivated_at"], Value::Null);
    assert_eq!(value["session_epoch"], 1);
}

#[tokio::test]
async fn reset_password_replaces_the_hash_demands_a_change_and_raises_the_epoch() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    let mut tx = begin(&store);
    let image = tx
        .users()
        .reset_password(user_id(1), &hash_b())
        .await
        .unwrap();
    let value = image.as_value();
    assert!(without_hash(value));
    assert_eq!(value["must_change_password"], true);
    assert_eq!(value["session_epoch"], 1);
    let found = tx
        .users()
        .find_by_login(&login("anna"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.password_hash, hash_b());
}

#[tokio::test]
async fn change_own_password_replaces_the_hash_clears_the_flag_and_raises_the_epoch() {
    let store = store();
    let mut user = new_user(1, "anna", Role::Operator);
    user.must_change_password = true;
    store.seed_user(&user);
    let mut tx = begin(&store);
    let image = tx
        .users()
        .change_own_password(user_id(1), &hash_b())
        .await
        .unwrap();
    let value = image.as_value();
    assert!(without_hash(value));
    assert_eq!(value["must_change_password"], false);
    assert_eq!(value["session_epoch"], 1);
    let found = tx
        .users()
        .find_by_login(&login("anna"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.password_hash, hash_b());
}

#[tokio::test]
async fn a_write_to_a_missing_user_fails_with_a_storage_error() {
    let store = store();
    let mut tx = begin(&store);
    let error = tx
        .users()
        .set_role(user_id(9), Role::Operator)
        .await
        .unwrap_err();
    assert!(matches!(error, StorageError::Other { .. }), "{error:?}");
}

#[tokio::test]
async fn list_orders_by_login_in_byte_order() {
    let store = store();
    for (n, name) in [
        (1, "carol"),
        (2, "anna"),
        (3, "bob"),
        (4, "a_b"),
        (5, "axb"),
    ] {
        store.seed_user(&new_user(n, name, Role::Operator));
    }
    let mut tx = begin(&store);
    let rows = tx.users().list(None, 0, 100).await.unwrap();
    let names: Vec<&str> = rows.iter().map(|row| row.login.as_str()).collect();
    assert_eq!(names, ["a_b", "anna", "axb", "bob", "carol"]);
}

#[tokio::test]
async fn list_filters_by_substring_and_treats_the_underscore_as_a_plain_character() {
    let store = store();
    for (n, name) in [(1, "anna"), (2, "a_b"), (3, "axb")] {
        store.seed_user(&new_user(n, name, Role::Operator));
    }
    let mut tx = begin(&store);
    let underscore = tx.users().list(Some("_"), 0, 100).await.unwrap();
    assert_eq!(underscore.len(), 1);
    assert_eq!(underscore.first().unwrap().login.as_str(), "a_b");
    let inner = tx.users().list(Some("nn"), 0, 100).await.unwrap();
    assert_eq!(inner.len(), 1);
    assert_eq!(inner.first().unwrap().login.as_str(), "anna");
}

#[tokio::test]
async fn list_returns_one_page_of_page_size_rows() {
    let store = store();
    for (n, name) in [(1, "aaa"), (2, "bbb"), (3, "ccc"), (4, "ddd"), (5, "eee")] {
        store.seed_user(&new_user(n, name, Role::Operator));
    }
    let mut tx = begin(&store);
    let names = |rows: Vec<crate::ports::UserSummary>| -> Vec<String> {
        rows.iter().map(|row| row.login.to_string()).collect()
    };
    assert_eq!(
        names(tx.users().list(None, 0, 2).await.unwrap()),
        ["aaa", "bbb"]
    );
    assert_eq!(
        names(tx.users().list(None, 1, 2).await.unwrap()),
        ["ccc", "ddd"]
    );
    assert_eq!(names(tx.users().list(None, 2, 2).await.unwrap()), ["eee"]);
    assert!(tx.users().list(None, 3, 2).await.unwrap().is_empty());
}

#[tokio::test]
async fn list_with_an_offset_above_i64_max_is_a_storage_error_as_in_the_sql_adapter() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Operator));
    let mut tx = begin(&store);

    let beyond = tx.users().list(None, u32::MAX, u32::MAX).await;

    assert!(matches!(
        beyond.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}

#[tokio::test]
async fn summary_by_login_gives_the_user_without_a_hash() {
    let store = store();
    store.seed_user(&new_user(1, "anna", Role::Administrator));
    let mut tx = begin(&store);
    let summary = tx
        .users()
        .summary_by_login(&login("anna"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(summary.id, user_id(1));
    assert_eq!(summary.role, Role::Administrator);
    assert!(summary.is_active);
    assert!(
        tx.users()
            .summary_by_login(&login("bob"))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn debug_of_a_user_row_and_a_new_user_hides_the_hash() {
    let store = store();
    let new = new_user(1, "anna", Role::Operator);
    let row = store.seed_user(&new);
    for shown in [format!("{new:?}"), format!("{row:?}")] {
        assert!(!shown.contains("argon2id"), "{shown}");
        assert!(shown.contains("[redacted]"), "{shown}");
    }
}
