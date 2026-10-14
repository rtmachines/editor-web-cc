//! Postgres only: the lock of `lock_by_login` and the error text of a `NOT NULL` failure.

use recipe_app::error::StorageError;
use recipe_domain::identity::Role;
use uuid::Uuid;

use crate::contract::chain::display_chain;
use crate::contract::harness::{Harness, begin, new_user, seed_user};
use crate::contract::locks::{LOCK_NOT_AVAILABLE, try_row_lock, wait_until_released};
use crate::contract::postgres_harness::PgHarness;
use crate::support::{Role as DbRole, probe};

/// `lock_by_login` holds `FOR NO KEY UPDATE`: `FOR UPDATE`, `FOR NO KEY UPDATE`, and `FOR SHARE`
/// wait, and `FOR KEY SHARE` passes. `FOR UPDATE` would block `FOR KEY SHARE` too.
#[tokio::test]
async fn postgres_only_lock_by_login_holds_for_no_key_update() {
    let h = PgHarness::create().await;
    let user = new_user("akowal", Role::Operator);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let id: Uuid = *user.id.as_uuid();
    let mut session = h.database.connect(DbRole::Migrator).await;
    let mut tx = begin(&h).await;

    tx.users()
        .lock_by_login(&user.login)
        .await
        .unwrap()
        .unwrap();

    for mode in ["UPDATE", "NO KEY UPDATE", "SHARE"] {
        assert_eq!(
            try_row_lock(&mut session, "app_user", mode, id)
                .await
                .as_deref(),
            Some(LOCK_NOT_AVAILABLE),
            "{mode}"
        );
    }
    assert_eq!(
        try_row_lock(&mut session, "app_user", "KEY SHARE", id).await,
        None
    );
    drop(tx);
    wait_until_released(&mut session, "app_user", id).await;
}

/// A lock of an unknown login takes no lock and does not fail.
#[tokio::test]
async fn postgres_only_lock_by_login_of_an_unknown_login_takes_no_lock() {
    let h = PgHarness::create().await;
    let mut tx = begin(&h).await;
    let unknown = new_user("unknown", Role::Operator);

    let locked = tx.users().lock_by_login(&unknown.login).await.unwrap();

    assert!(locked.is_none());
}

/// A `NOT NULL` failure on `app_user` reaches the error map. The text of the error chain holds
/// no hash (design D18). The failing row of the database holds the hash.
#[tokio::test]
async fn postgres_only_a_not_null_failure_on_app_user_leaks_no_hash() {
    let h = PgHarness::create().await;
    probe::install(
        &h.database,
        "app_user",
        "BEFORE",
        "INSERT",
        "NEW.role := NULL;",
    )
    .await;
    let mut tx = begin(&h).await;

    let error = tx
        .users()
        .insert(&new_user("akowal", Role::Operator))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Other { sqlstate, .. } if sqlstate.as_deref() == Some("23502")),
        "{error:?}"
    );
    let chain = display_chain(&error);
    assert!(chain.contains("not-null"), "{chain}");
    assert!(!chain.contains("$argon2id$"), "{chain}");
}
