//! Contract cases of the intent writes of `UserRepository`.
//!
//! Each case compares the image before and after the write. Only the columns of the row in
//! `data-model.md` section 6 change.

use recipe_app::ports::{RowImage, Transaction};
use recipe_domain::identity::{Login, PasswordHash, Role, UserId};
use uuid::Uuid;

use crate::contract::harness::{Harness, begin, new_user, seed_user};
use crate::contract::images::{changed_keys, field, text};

/// Seed a user and lock it in a new transaction. Return the transaction, the id, and the image.
async fn locked(
    h: &dyn Harness,
    login: &str,
    must_change_password: bool,
) -> (Box<dyn Transaction>, UserId, RowImage) {
    let mut user = new_user(login, Role::Operator);
    user.must_change_password = must_change_password;
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let mut tx = begin(h).await;
    let before = tx
        .users()
        .lock_by_login(&user.login)
        .await
        .unwrap()
        .unwrap()
        .before;
    (tx, user.id, before)
}

fn hash(text: &str) -> PasswordHash {
    PasswordHash::parse(text).unwrap()
}

async fn stored_hash(tx: &mut Box<dyn Transaction>, login: &str) -> PasswordHash {
    let login = Login::parse(login).unwrap();
    tx.users()
        .find_by_login(&login)
        .await
        .unwrap()
        .unwrap()
        .password_hash
}

async fn set_role_changes_the_role_and_the_epoch_only(h: &dyn Harness) {
    let (mut tx, id, before) = locked(h, "akowal", false).await;

    let after = tx
        .users()
        .set_role(id, Role::ProcessEngineer)
        .await
        .unwrap();

    assert_eq!(changed_keys(&before, &after), ["role", "session_epoch"]);
    assert_eq!(text(&after, "role"), "process_engineer");
    assert_eq!(field(&after, "session_epoch").as_i64(), Some(1));
    assert_eq!(
        stored_hash(&mut tx, "akowal").await,
        hash("$argon2id$fixture")
    );
}

async fn deactivate_clears_the_flag_sets_the_time_and_raises_the_epoch(h: &dyn Harness) {
    let (mut tx, id, before) = locked(h, "akowal", false).await;

    let after = tx.users().deactivate(id).await.unwrap();

    assert_eq!(
        changed_keys(&before, &after),
        ["deactivated_at", "is_active", "session_epoch"]
    );
    assert_eq!(field(&after, "is_active"), &serde_json::json!(false));
    assert!(field(&after, "deactivated_at").is_string());
    assert_eq!(field(&after, "session_epoch").as_i64(), Some(1));
}

async fn activate_sets_the_flag_clears_the_time_and_keeps_the_epoch(h: &dyn Harness) {
    let (mut tx, id, _) = locked(h, "akowal", false).await;
    let deactivated = tx.users().deactivate(id).await.unwrap();

    let after = tx.users().activate(id).await.unwrap();

    assert_eq!(
        changed_keys(&deactivated, &after),
        ["deactivated_at", "is_active"]
    );
    assert_eq!(field(&after, "is_active"), &serde_json::json!(true));
    assert!(field(&after, "deactivated_at").is_null());
    assert_eq!(field(&after, "session_epoch").as_i64(), Some(1));
}

async fn reset_password_replaces_the_hash_and_demands_a_change(h: &dyn Harness) {
    let (mut tx, id, before) = locked(h, "akowal", false).await;

    let after = tx
        .users()
        .reset_password(id, &hash("$argon2id$reset"))
        .await
        .unwrap();

    assert_eq!(
        changed_keys(&before, &after),
        ["must_change_password", "session_epoch"]
    );
    assert_eq!(
        field(&after, "must_change_password"),
        &serde_json::json!(true)
    );
    assert_eq!(field(&after, "session_epoch").as_i64(), Some(1));
    assert!(!after.as_value().to_string().contains("argon2id"));
    assert_eq!(
        stored_hash(&mut tx, "akowal").await,
        hash("$argon2id$reset")
    );
}

async fn change_own_password_replaces_the_hash_and_clears_the_demand(h: &dyn Harness) {
    let (mut tx, id, before) = locked(h, "akowal", true).await;

    let after = tx
        .users()
        .change_own_password(id, &hash("$argon2id$own"))
        .await
        .unwrap();

    assert_eq!(
        changed_keys(&before, &after),
        ["must_change_password", "session_epoch"]
    );
    assert_eq!(
        field(&after, "must_change_password"),
        &serde_json::json!(false)
    );
    assert_eq!(field(&after, "session_epoch").as_i64(), Some(1));
    assert_eq!(stored_hash(&mut tx, "akowal").await, hash("$argon2id$own"));
}

async fn a_write_on_a_missing_user_is_an_error(h: &dyn Harness) {
    let mut tx = begin(h).await;
    let id = UserId::from(Uuid::from_u128(77));

    let results = [
        tx.users().set_role(id, Role::Operator).await.is_err(),
        tx.users().deactivate(id).await.is_err(),
        tx.users().activate(id).await.is_err(),
        tx.users()
            .reset_password(id, &hash("$argon2id$x"))
            .await
            .is_err(),
        tx.users()
            .change_own_password(id, &hash("$argon2id$x"))
            .await
            .is_err(),
    ];

    assert_eq!(results, [true; 5]);
}

contract_case!(
    set_role_changes_the_role_and_the_epoch_only,
    set_role_changes_the_role_and_the_epoch_only_memory,
    set_role_changes_the_role_and_the_epoch_only_postgres
);
contract_case!(
    deactivate_clears_the_flag_sets_the_time_and_raises_the_epoch,
    deactivate_clears_the_flag_sets_the_time_and_raises_the_epoch_memory,
    deactivate_clears_the_flag_sets_the_time_and_raises_the_epoch_postgres
);
contract_case!(
    activate_sets_the_flag_clears_the_time_and_keeps_the_epoch,
    activate_sets_the_flag_clears_the_time_and_keeps_the_epoch_memory,
    activate_sets_the_flag_clears_the_time_and_keeps_the_epoch_postgres
);
contract_case!(
    reset_password_replaces_the_hash_and_demands_a_change,
    reset_password_replaces_the_hash_and_demands_a_change_memory,
    reset_password_replaces_the_hash_and_demands_a_change_postgres
);
contract_case!(
    change_own_password_replaces_the_hash_and_clears_the_demand,
    change_own_password_replaces_the_hash_and_clears_the_demand_memory,
    change_own_password_replaces_the_hash_and_clears_the_demand_postgres
);
contract_case!(
    a_write_on_a_missing_user_is_an_error,
    a_write_on_a_missing_user_is_an_error_memory,
    a_write_on_a_missing_user_is_an_error_postgres
);
