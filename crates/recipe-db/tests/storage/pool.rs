//! Tests of `Database::connect` and of the settings of the pool (task 8.3, design D9).
//!
//! The settings of a pooled connection reach the tests only through the public ports. A test
//! trigger of the harness reports them, and the behavior tests change one of them (task 8.5).

use std::error::Error as _;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use recipe_app::error::{LockFailure, StorageError};
use recipe_app::ports::{Isolation, UnitOfWork};
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Login;
use sqlx::Connection as _;
use uuid::Uuid;

use crate::support::audit::{audit_count, entries, install_trigger};
use crate::support::fixtures;
use crate::support::{Role, TestDatabase};

/// The idle time in a transaction that the idle test sets.
const SHORT_IDLE_IN_TRANSACTION: Duration = Duration::from_millis(300);
/// The wait that is longer than every short timeout of the tests.
const LONGER_THAN_TIMEOUTS: Duration = Duration::from_millis(1500);

/// Connect as `recipe_app` with the given settings. Return the unit of work and a user id.
async fn pooled(database: &TestDatabase, settings: &PoolSettings) -> (Arc<dyn UnitOfWork>, Uuid) {
    let mut migrator = database.connect(Role::Migrator).await;
    let user = fixtures::user(&mut migrator).await;
    let connected = Database::connect(&database.target(Role::App), settings)
        .await
        .unwrap();
    (connected.unit_of_work(), user)
}

/// The wait for a free connection in the tests that expect a failure.
///
/// `Database::connect` opens its first connection inside this time. An instrumented build under
/// load (`cargo llvm-cov`) needs more than 300 ms for that. The value stays below
/// `LONGER_THAN_TIMEOUTS`.
const SHORT_ACQUIRE: Duration = Duration::from_millis(1000);

/// Return a TCP port on which nothing listens.
fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[tokio::test]
async fn connect_to_a_closed_port_gives_unavailable() {
    let database = TestDatabase::create().await;
    let mut target = database.target(Role::App);
    target.host = "127.0.0.1".to_owned();
    target.port = closed_port();
    let settings = PoolSettings {
        acquire_timeout: SHORT_ACQUIRE,
        ..PoolSettings::default()
    };

    let error = Database::connect(&target, &settings).await.unwrap_err();

    assert!(matches!(error, StorageError::Unavailable), "{error:?}");
    assert_eq!(error.code(), ErrorCode::DB_005);
}

#[tokio::test]
async fn connect_as_recipe_app_to_a_test_database_succeeds() {
    let database = TestDatabase::create().await;
    let target = database.target(Role::App);

    let connected = Database::connect(&target, &PoolSettings::default()).await;

    assert!(connected.is_ok(), "{:?}", connected.err());
}

#[tokio::test]
async fn connect_with_a_wrong_password_gives_a_storage_error_without_the_password() {
    let database = TestDatabase::create().await;
    let mut target = database.target(Role::App);
    target.password = "wrong-password-text".to_owned().into();

    let error = Database::connect(&target, &PoolSettings::default())
        .await
        .unwrap_err();

    assert!(
        !format!("{error:?}").contains("wrong-password-text"),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_pooled_connection_shows_the_three_timeouts_and_the_time_zone() {
    let database = TestDatabase::create().await;
    install_trigger(
        &database,
        "BEFORE",
        "RAISE EXCEPTION 'tz=% statement=% lock=% idle=%', current_setting('TimeZone'), \
             current_setting('statement_timeout'), current_setting('lock_timeout'), \
             current_setting('idle_in_transaction_session_timeout');",
    )
    .await;
    let settings = PoolSettings {
        statement_timeout: Duration::from_secs(7),
        lock_timeout: Duration::from_secs(3),
        idle_in_transaction_timeout: Duration::from_secs(20),
        ..PoolSettings::default()
    };
    let (unit_of_work, user) = pooled(&database, &settings).await;
    let (list, _) = entries(Some(user), 1);
    let mut tx = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();

    let error = tx.append_audit(&list).await.unwrap_err();

    let cause = error.source().unwrap().to_string();
    assert!(
        cause.contains("tz=UTC statement=7s lock=3s idle=20s"),
        "{cause}"
    );
}

#[tokio::test]
async fn a_statement_that_runs_too_long_gives_statement_timeout() {
    let database = TestDatabase::create().await;
    install_trigger(&database, "BEFORE", "PERFORM pg_sleep(3);").await;
    let settings = PoolSettings {
        statement_timeout: Duration::from_millis(300),
        ..PoolSettings::default()
    };
    let (unit_of_work, user) = pooled(&database, &settings).await;
    let (list, _) = entries(Some(user), 1);
    let mut tx = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();

    let error = tx.append_audit(&list).await.unwrap_err();

    assert!(matches!(error, StorageError::StatementTimeout), "{error:?}");
    assert_eq!(error.code(), ErrorCode::DB_002);
}

#[tokio::test]
async fn a_lock_wait_that_passes_the_lock_timeout_gives_deadlock() {
    let database = TestDatabase::create().await;
    let settings = PoolSettings {
        lock_timeout: Duration::from_millis(300),
        ..PoolSettings::default()
    };
    let (unit_of_work, user) = pooled(&database, &settings).await;
    // The hash chain trigger locks the head row, so a lock on the table blocks every audit insert.
    let mut admin = database.connect(Role::Superuser).await;
    let mut blocker = admin.begin().await.unwrap();
    sqlx::query("LOCK TABLE audit_chain_head IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let (list, _) = entries(Some(user), 1);
    let mut tx = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();

    let error = tx.append_audit(&list).await.unwrap_err();

    assert!(
        matches!(
            error,
            StorageError::Deadlock {
                cause: LockFailure::LockTimeout
            }
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_001);
}

#[tokio::test]
async fn an_idle_transaction_that_passes_its_limit_ends_and_its_write_is_gone() {
    let database = TestDatabase::create().await;
    let settings = PoolSettings {
        idle_in_transaction_timeout: SHORT_IDLE_IN_TRANSACTION,
        ..PoolSettings::default()
    };
    let (unit_of_work, user) = pooled(&database, &settings).await;
    let mut admin = database.connect(Role::Superuser).await;
    let (first, _) = entries(Some(user), 1);
    let (second, _) = entries(Some(user), 1);
    let mut tx = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();
    tx.append_audit(&first).await.unwrap();

    tokio::time::sleep(LONGER_THAN_TIMEOUTS).await;

    assert!(tx.append_audit(&second).await.is_err());
    drop(tx);
    assert_eq!(audit_count(&mut admin).await, 0);
}

#[tokio::test]
async fn a_pool_without_a_free_connection_gives_pool_timeout() {
    let database = TestDatabase::create().await;
    let settings = PoolSettings {
        size: 1,
        acquire_timeout: SHORT_ACQUIRE,
        ..PoolSettings::default()
    };
    let (unit_of_work, _) = pooled(&database, &settings).await;
    let _held = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();

    let error = unit_of_work
        .begin(Isolation::ReadCommitted)
        .await
        .err()
        .unwrap();

    assert!(matches!(error, StorageError::PoolTimeout), "{error:?}");
    assert_eq!(error.code(), ErrorCode::DB_004);
}

/// The statement of the leak test: it opens a read-only transaction and leaves it open.
const LEAKED_BEGIN: &str = "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY";
/// The statement of the failed transaction test: `BEGIN` and then a statement that fails.
const ABORTED_TRANSACTION: &str = "BEGIN; SELECT 1 / 0";

/// A pool of size 1, so each acquire waits for the release of the one connection.
async fn single_connection_pool(database: &TestDatabase) -> Database {
    let settings = PoolSettings {
        size: 1,
        ..PoolSettings::default()
    };
    Database::connect(&database.target(Role::App), &settings)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_released_connection_with_an_open_transaction_is_not_reused() {
    let database = TestDatabase::create().await;
    let pool = single_connection_pool(&database).await;
    let before = pool.session_facts().await.unwrap();
    pool.send_raw_statement(LEAKED_BEGIN).await.unwrap();

    let after = pool.session_facts().await.unwrap();

    assert_ne!(after.backend_pid, before.backend_pid, "the pid stayed");
    assert_eq!(after.transaction_read_only, "off");
    assert_eq!(after.transaction_isolation, "read committed");
    assert!(!after.in_transaction);
}

#[tokio::test]
async fn a_released_connection_with_a_failed_transaction_is_not_reused() {
    let database = TestDatabase::create().await;
    let pool = single_connection_pool(&database).await;
    let before = pool.session_facts().await.unwrap();
    let failure = pool.send_raw_statement(ABORTED_TRANSACTION).await;
    assert!(failure.is_err(), "the statement must fail");

    let after = pool
        .session_facts()
        .await
        .expect("the next acquire must get a new connection, not the aborted one");

    assert_ne!(after.backend_pid, before.backend_pid, "the pid stayed");
}

#[tokio::test]
async fn a_connection_stays_open_over_a_command_and_a_query() {
    let database = TestDatabase::create().await;
    let mut migrator = database.connect(Role::Migrator).await;
    let user = fixtures::user(&mut migrator).await;
    let pool = single_connection_pool(&database).await;
    let unit_of_work = pool.unit_of_work();
    let first = pool.session_facts().await.unwrap();

    let (list, _) = entries(Some(user), 1);
    let command = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();
    unit_of_work.commit(command, list).await.unwrap();
    let mut query = unit_of_work.begin(Isolation::RepeatableRead).await.unwrap();
    let login = Login::parse("nobody").unwrap();
    assert!(query.users().find_by_login(&login).await.unwrap().is_none());
    drop(query);
    let last = pool.session_facts().await.unwrap();

    assert_eq!(first.backend_pid, last.backend_pid);
}
