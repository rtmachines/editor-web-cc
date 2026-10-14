//! Tests of the baseline requirement "Migrator lock and retries" and of the template databases.

use std::time::{Duration, Instant};

use recipe_app::error::StorageError;
use recipe_db::migrate_from_directory;
use recipe_db::settings::MigrateSettings;
use recipe_domain::error_codes::ErrorCode;
use sqlx::PgConnection;

use crate::support::{Role, TempMigrations, TestDatabase, connect, server};

const CREATE_TARGET: &str = "CREATE TABLE lock_target (id INTEGER PRIMARY KEY)";
const LOCK_TARGET: &str = "LOCK TABLE lock_target IN ACCESS EXCLUSIVE MODE";
/// A migration that waits for the lock of `lock_target`.
const TOUCH_MIGRATION: [(&str, &str); 1] =
    [("0001_touch.sql", "INSERT INTO lock_target (id) VALUES (1);")];

fn settings(lock_ms: u64, retries: u32, pause_ms: u64) -> MigrateSettings {
    MigrateSettings {
        lock_timeout: Duration::from_millis(lock_ms),
        retries,
        retry_pause: Duration::from_millis(pause_ms),
    }
}

/// The template carries the history of the real migrations. A test migration directory has other
/// files, so the history goes away and the test starts with a clean migrator.
const DROP_HISTORY: &str = "DROP TABLE _sqlx_migrations";

async fn database_with_target() -> TestDatabase {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    sqlx::query(DROP_HISTORY).execute(&mut conn).await.unwrap();
    sqlx::query(CREATE_TARGET).execute(&mut conn).await.unwrap();
    database
}

/// Open a transaction that holds an `ACCESS EXCLUSIVE` lock on `lock_target`.
async fn hold_lock(database: &TestDatabase) -> PgConnection {
    let mut holder = database.connect(Role::Migrator).await;
    sqlx::query("BEGIN").execute(&mut holder).await.unwrap();
    sqlx::query(LOCK_TARGET).execute(&mut holder).await.unwrap();
    holder
}

async fn rows_in_target(database: &TestDatabase) -> i64 {
    let mut conn = database.connect(Role::Migrator).await;
    sqlx::query_scalar("SELECT count(*) FROM lock_target")
        .fetch_one(&mut conn)
        .await
        .unwrap()
}

async fn migrator_sessions(database: &TestDatabase) -> i64 {
    let mut admin = connect(server(), Role::Superuser, "postgres").await;
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_stat_activity WHERE datname = $1 AND usename = 'recipe_migrator'",
    )
    .bind(database.name())
    .fetch_one(&mut admin)
    .await
    .unwrap()
}

#[tokio::test]
async fn migrate_applies_the_files_of_a_directory_once() {
    let database = database_with_target().await;
    let files = TempMigrations::new(&TOUCH_MIGRATION);
    let target = database.target(Role::Migrator);

    migrate_from_directory(&target, &MigrateSettings::default(), files.path())
        .await
        .unwrap();
    migrate_from_directory(&target, &MigrateSettings::default(), files.path())
        .await
        .unwrap();

    assert_eq!(rows_in_target(&database).await, 1);
}

#[tokio::test]
async fn migrate_maps_a_held_table_lock_to_e_db_001_when_no_retry_remains() {
    let database = database_with_target().await;
    let mut holder = hold_lock(&database).await;
    let files = TempMigrations::new(&TOUCH_MIGRATION);

    let error = migrate_from_directory(
        &database.target(Role::Migrator),
        &settings(150, 0, 50),
        files.path(),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, StorageError::Deadlock), "{error:?}");
    assert_eq!(error.code(), ErrorCode::DB_001);
    sqlx::query("COMMIT").execute(&mut holder).await.unwrap();
    assert_eq!(rows_in_target(&database).await, 0);
}

#[tokio::test]
async fn migrate_succeeds_on_a_new_connection_after_the_lock_is_released() {
    let database = database_with_target().await;
    let mut holder = hold_lock(&database).await;
    let release = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(700)).await;
        sqlx::query("COMMIT").execute(&mut holder).await.unwrap();
    });
    let files = TempMigrations::new(&TOUCH_MIGRATION);
    let started = Instant::now();

    migrate_from_directory(
        &database.target(Role::Migrator),
        &settings(200, 20, 100),
        files.path(),
    )
    .await
    .unwrap();

    release.await.unwrap();
    assert!(started.elapsed() >= Duration::from_millis(600));
    assert_eq!(rows_in_target(&database).await, 1);
}

#[tokio::test]
async fn migrate_gives_e_db_001_after_the_last_retry_and_closes_its_connections() {
    let database = database_with_target().await;
    let mut holder = hold_lock(&database).await;
    let sessions_before = migrator_sessions(&database).await;
    let files = TempMigrations::new(&TOUCH_MIGRATION);
    let started = Instant::now();

    let error = migrate_from_directory(
        &database.target(Role::Migrator),
        &settings(100, 2, 50),
        files.path(),
    )
    .await
    .unwrap_err();

    // Three attempts wait 100 ms each. Two pauses of 50 ms separate them.
    assert!(started.elapsed() >= Duration::from_millis(390));
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(migrator_sessions(&database).await, sessions_before);
    sqlx::query("COMMIT").execute(&mut holder).await.unwrap();
    assert_eq!(rows_in_target(&database).await, 0);
}

#[tokio::test]
async fn migrate_does_not_retry_an_error_that_is_not_a_lock_timeout() {
    let database = database_with_target().await;
    let files = TempMigrations::new(&[("0001_broken.sql", "SELECT * FROM no_such_table;")]);
    let started = Instant::now();

    let error = migrate_from_directory(
        &database.target(Role::Migrator),
        &settings(100, 10, 2_000),
        files.path(),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, StorageError::Other { .. }), "{error:?}");
    assert!(started.elapsed() < Duration::from_millis(1_500));
}

#[tokio::test]
async fn a_new_test_database_belongs_to_the_migrator_and_accepts_its_tables() {
    let database = TestDatabase::create().await;
    let mut admin = connect(server(), Role::Superuser, "postgres").await;

    let owner: String =
        sqlx::query_scalar("SELECT pg_get_userbyid(datdba) FROM pg_database WHERE datname = $1")
            .bind(database.name())
            .fetch_one(&mut admin)
            .await
            .unwrap();
    let mut migrator = database.connect(Role::Migrator).await;
    sqlx::query("CREATE TABLE probe (id INTEGER)")
        .execute(&mut migrator)
        .await
        .unwrap();
    let has_history: bool =
        sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
            .fetch_one(&mut migrator)
            .await
            .unwrap();

    assert_eq!(owner, "recipe_migrator");
    assert!(has_history, "the template carries the migration table");
}

#[tokio::test]
async fn a_new_test_database_repeats_the_connect_grants_of_the_init_script() {
    let database = TestDatabase::create().await;
    let mut admin = connect(server(), Role::Superuser, "postgres").await;

    for (role, expected) in [
        (Role::Migrator, true),
        (Role::App, true),
        (Role::Backup, true),
        (Role::Replicator, false),
    ] {
        let allowed: bool = sqlx::query_scalar("SELECT has_database_privilege($1, $2, 'CONNECT')")
            .bind(role.name())
            .bind(database.name())
            .fetch_one(&mut admin)
            .await
            .unwrap();
        assert_eq!(allowed, expected, "CONNECT of {}", role.name());
    }
    let public_temporary: bool =
        sqlx::query_scalar("SELECT has_database_privilege('recipe_replicator', $1, 'TEMPORARY')")
            .bind(database.name())
            .fetch_one(&mut admin)
            .await
            .unwrap();
    assert!(!public_temporary);
}
