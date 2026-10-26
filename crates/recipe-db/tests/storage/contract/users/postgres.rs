//! Postgres only: the locks of `lock_by_login` and `lock_administrator_guard`, and the error text
//! of a `NOT NULL` failure.

use std::time::Duration;

use recipe_app::error::StorageError;
use recipe_domain::identity::Role;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::contract::chain::display_chain;
use crate::contract::harness::{Harness, begin, commit, new_user, seed_user};
use crate::contract::locks::{LOCK_NOT_AVAILABLE, try_row_lock, wait_until_released};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::seed::{insert, machine};
use crate::support::{Role as DbRole, probe};

/// The class of the guard lock of the last Administrator (`data-model.md` section 1.4, ADR-0096).
const GUARD_CLASS: i32 = 4;
/// The only key of the guard lock.
const GUARD_KEY: i32 = 0;
/// The pause between two looks at `pg_locks`.
const POLL_PAUSE: Duration = Duration::from_millis(20);
/// The number of looks at `pg_locks` before the case gives up.
const POLL_LIMIT: u32 = 250;

/// Count the granted advisory locks of class 4 and key 0 in the current database.
///
/// `pg_advisory_xact_lock(int, int)` shows as `classid` = first argument, `objid` = second
/// argument, and `objsubid` = 2.
async fn guard_lock_holders(session: &mut PgConnection) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_locks          WHERE locktype = 'advisory' AND classid = $1 AND objid = $2 AND objsubid = 2 AND granted          AND database = (SELECT oid FROM pg_database WHERE datname = current_database())",
    )
    .bind(GUARD_CLASS)
    .bind(GUARD_KEY)
    .fetch_one(session)
    .await
    .unwrap()
}

/// Wait until no session holds the guard lock. A dropped transaction rolls back in the background.
async fn wait_until_guard_released(session: &mut PgConnection) {
    for _ in 0..POLL_LIMIT {
        if guard_lock_holders(session).await == 0 {
            return;
        }
        tokio::time::sleep(POLL_PAUSE).await;
    }
    assert_eq!(
        guard_lock_holders(session).await,
        0,
        "the guard stays locked"
    );
}

/// `lock_administrator_guard` holds the lock of class 4 and key 0 until the transaction rolls back.
#[tokio::test]
async fn postgres_only_the_guard_lock_is_held_until_the_transaction_rolls_back() {
    let h = PgHarness::create().await;
    let mut session = h.database.connect(DbRole::Migrator).await;
    assert_eq!(guard_lock_holders(&mut session).await, 0);
    let mut tx = begin(&h).await;

    tx.users().lock_administrator_guard().await.unwrap();

    assert_eq!(guard_lock_holders(&mut session).await, 1);
    // A second call in the same transaction adds no second holder.
    tx.users().lock_administrator_guard().await.unwrap();
    assert_eq!(guard_lock_holders(&mut session).await, 1);
    drop(tx);
    wait_until_guard_released(&mut session).await;
}

/// `lock_administrator_guard` holds the lock until the commit and not after it.
#[tokio::test]
async fn postgres_only_the_guard_lock_is_released_by_the_commit() {
    let h = PgHarness::create().await;
    let built = machine(160, "M160");
    insert(&h, &built).await;
    let mut session = h.database.connect(DbRole::Migrator).await;
    let mut tx = begin(&h).await;

    tx.users().lock_administrator_guard().await.unwrap();

    assert_eq!(guard_lock_holders(&mut session).await, 1);
    commit(&h, tx, built.id()).await;
    assert_eq!(guard_lock_holders(&mut session).await, 0);
}

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
