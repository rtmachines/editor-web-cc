//! Contract cases of `UserRepository` (task 5.4 of `identity-and-sessions`).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The cases that
//! need PostgreSQL carry the prefix `postgres_only`.

mod list;
mod postgres;
mod writes;

use recipe_app::error::{APP_USER_LOGIN_UNIQUE, StorageError};
use recipe_domain::identity::Role;
use serde_json::json;

use crate::contract::harness::{FIXTURE_PASSWORD_HASH, Harness, begin, new_user, seed_user};
use crate::contract::images::{assert_shape, field, text, user_shape};

async fn insert_gives_an_image_without_the_hash(h: &dyn Harness) {
    let mut user = new_user("akowal", Role::Operator);
    user.must_change_password = true;
    let mut tx = begin(h).await;

    let image = tx.users().insert(&user).await.unwrap();

    assert_shape(&image, &user_shape(false));
    assert_eq!(text(&image, "id"), user.id.to_string());
    assert_eq!(text(&image, "login"), "akowal");
    assert_eq!(text(&image, "role"), "operator");
    assert_eq!(field(&image, "is_active"), &json!(true));
    assert_eq!(field(&image, "session_epoch").as_i64(), Some(0));
    assert_eq!(field(&image, "must_change_password"), &json!(true));
    assert!(!image.as_value().to_string().contains("argon2id"));
    assert!(!image.as_value().to_string().contains(FIXTURE_PASSWORD_HASH));
}

async fn a_duplicate_login_gives_the_login_constraint(h: &dyn Harness) {
    let first = new_user("akowal", Role::Operator);
    seed_user(h.unit_of_work().as_ref(), &first).await;
    let second = new_user("akowal", Role::Administrator);
    let mut tx = begin(h).await;

    let error = tx.users().insert(&second).await.unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == APP_USER_LOGIN_UNIQUE),
        "{error:?}"
    );
}

async fn find_by_login_gives_the_row_with_the_hash(h: &dyn Harness) {
    let user = new_user("akowal", Role::ProcessEngineer);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let mut tx = begin(h).await;

    let row = tx
        .users()
        .find_by_login(&user.login)
        .await
        .unwrap()
        .unwrap();
    let unknown = new_user("unknown", Role::Operator);
    let missing = tx.users().find_by_login(&unknown.login).await.unwrap();

    assert_eq!(row.id, user.id);
    assert_eq!(row.login, user.login);
    assert_eq!(row.password_hash, user.password_hash);
    assert_eq!(row.role, Role::ProcessEngineer);
    assert!(row.is_active);
    assert_eq!(row.session_epoch, 0);
    assert!(!row.must_change_password);
    assert_eq!(row.deactivated_at, None);
    assert!(missing.is_none());
}

async fn lock_by_login_gives_the_state_and_the_image_before_the_change(h: &dyn Harness) {
    let user = new_user("akowal", Role::Operator);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let mut tx = begin(h).await;

    let locked = tx
        .users()
        .lock_by_login(&user.login)
        .await
        .unwrap()
        .expect("the user exists");
    let unknown = new_user("unknown", Role::Operator);
    let missing = tx.users().lock_by_login(&unknown.login).await.unwrap();

    assert_eq!(locked.state.id, user.id);
    assert_eq!(locked.state.password_hash, user.password_hash);
    assert_shape(&locked.before, &user_shape(false));
    assert_eq!(text(&locked.before, "id"), user.id.to_string());
    assert_eq!(text(&locked.before, "role"), "operator");
    assert!(!locked.before.as_value().to_string().contains("argon2id"));
    assert!(missing.is_none());
}

async fn count_active_administrators_counts_only_active_administrators(h: &dyn Harness) {
    let mut tx = begin(h).await;
    // The harness inserted one active administrator.
    let start = tx.users().count_active_administrators().await.unwrap();
    let boss = new_user("boss", Role::Administrator);
    tx.users().insert(&boss).await.unwrap();
    tx.users()
        .insert(&new_user("worker", Role::Operator))
        .await
        .unwrap();
    let with_boss = tx.users().count_active_administrators().await.unwrap();

    tx.users().deactivate(boss.id).await.unwrap();
    let without_boss = tx.users().count_active_administrators().await.unwrap();

    assert_eq!((start, with_boss, without_boss), (1, 2, 1));
}

contract_case!(
    insert_gives_an_image_without_the_hash,
    insert_gives_an_image_without_the_hash_memory,
    insert_gives_an_image_without_the_hash_postgres
);
contract_case!(
    a_duplicate_login_gives_the_login_constraint,
    a_duplicate_login_gives_the_login_constraint_memory,
    a_duplicate_login_gives_the_login_constraint_postgres
);
contract_case!(
    find_by_login_gives_the_row_with_the_hash,
    find_by_login_gives_the_row_with_the_hash_memory,
    find_by_login_gives_the_row_with_the_hash_postgres
);
contract_case!(
    lock_by_login_gives_the_state_and_the_image_before_the_change,
    lock_by_login_gives_the_state_and_the_image_before_the_change_memory,
    lock_by_login_gives_the_state_and_the_image_before_the_change_postgres
);
contract_case!(
    count_active_administrators_counts_only_active_administrators,
    count_active_administrators_counts_only_active_administrators_memory,
    count_active_administrators_counts_only_active_administrators_postgres
);
