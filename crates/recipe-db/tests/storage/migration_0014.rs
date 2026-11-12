//! Tests of the migration `0014_machine_db_number_copy.sql` (ADR-0156, design D2 item 2).
//!
//! The tests run the file on a schema where the shape constraint of `0013` and the objects of
//! `0014` are gone. That state lets a live S7 machine hold no number, as a database has it
//! between `0013` and `0014`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::support::{Role, TestDatabase};

const COPIED_CODE: &str = "M5";
const ARCHIVED_CODE: &str = "M9001";
const IEC_CODE: &str = "M9002";
const SET_CODE: &str = "M9003";
const RECORD_NUMBER: i32 = 100;
const SET_NUMBER: i32 = 7;
const NEW_RECORD_NUMBER: i32 = 200;

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await
}

/// Put the schema back to the state between `0013` and `0014`: no shape constraint, no trigger.
async fn drop_the_shape_constraint(conn: &mut PgConnection) {
    sqlx::raw_sql(
        "ALTER TABLE machine DROP CONSTRAINT IF EXISTS machine_db_number_shape; \
         DROP TRIGGER IF EXISTS plc_connection_sync_db_number ON plc_connection; \
         DROP FUNCTION IF EXISTS plc_connection_sync_db_number();",
    )
    .execute(conn)
    .await
    .unwrap();
}

/// Run the whole text of `0014` as one simple query.
async fn run_0014(conn: &mut PgConnection) {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/0014_machine_db_number_copy.sql"
    ))
    .unwrap();
    sqlx::raw_sql(&text).execute(conn).await.unwrap();
}

/// Insert a machine with a fixed code, a platform, and a `db_number` that may be `NULL`.
async fn machine(
    conn: &mut PgConnection,
    code: &str,
    platform: &str,
    db_number: Option<i32>,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO machine (code, name, plc_platform, db_number) \
         VALUES ($1, 'machine', $2, $3) RETURNING id",
    )
    .bind(code)
    .bind(platform)
    .bind(db_number)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert the S7 connection record of a machine with the data block number `100`.
async fn s7_record(conn: &mut PgConnection, machine: Uuid, platform: &str) {
    sqlx::query(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, rack, slot, db_number) \
         VALUES ($1, $2, '192.168.1.20', 0, 2, $3)",
    )
    .bind(machine)
    .bind(platform)
    .bind(RECORD_NUMBER)
    .execute(conn)
    .await
    .unwrap();
}

/// Return `machine.db_number` of the machine with the code.
async fn db_number_of(conn: &mut PgConnection, code: &str) -> Option<i32> {
    sqlx::query_scalar("SELECT db_number FROM machine WHERE code = $1")
        .bind(code)
        .fetch_one(conn)
        .await
        .unwrap()
}

/// Return `machine.updated_at` of the machine with the code.
async fn updated_at_of(conn: &mut PgConnection, code: &str) -> OffsetDateTime {
    sqlx::query_scalar("SELECT updated_at FROM machine WHERE code = $1")
        .bind(code)
        .fetch_one(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn migration_0014_copies_the_record_number_into_the_machine() {
    let mut conn = connection().await;
    drop_the_shape_constraint(&mut conn).await;
    let copied = machine(&mut conn, COPIED_CODE, "S7_1500", None).await;
    s7_record(&mut conn, copied, "S7_1500").await;

    run_0014(&mut conn).await;

    assert_eq!(
        db_number_of(&mut conn, COPIED_CODE).await,
        Some(RECORD_NUMBER)
    );
}

#[tokio::test]
async fn migration_0014_keeps_null_on_an_archived_s7_machine_without_a_record() {
    let mut conn = connection().await;
    drop_the_shape_constraint(&mut conn).await;
    machine(&mut conn, ARCHIVED_CODE, "S7_300", None).await;
    sqlx::query("UPDATE machine SET archived_at = now() WHERE code = $1")
        .bind(ARCHIVED_CODE)
        .execute(&mut conn)
        .await
        .unwrap();

    run_0014(&mut conn).await;

    assert_eq!(db_number_of(&mut conn, ARCHIVED_CODE).await, None);
}

#[tokio::test]
async fn migration_0014_keeps_null_on_an_iec_machine() {
    let mut conn = connection().await;
    drop_the_shape_constraint(&mut conn).await;
    let iec = machine(&mut conn, IEC_CODE, "CodesysIec", None).await;
    sqlx::query(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, port, address, comm_mode) \
         VALUES ($1, 'CodesysIec', '192.168.1.30', 502, 'plc', 'ModbusTcp')",
    )
    .bind(iec)
    .execute(&mut conn)
    .await
    .unwrap();

    run_0014(&mut conn).await;

    assert_eq!(db_number_of(&mut conn, IEC_CODE).await, None);
}

#[tokio::test]
async fn migration_0014_keeps_updated_at_of_an_iec_machine_with_a_record() {
    let mut conn = connection().await;
    drop_the_shape_constraint(&mut conn).await;
    let iec = machine(&mut conn, IEC_CODE, "CodesysIec", None).await;
    sqlx::query(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, port, address, comm_mode) \
         VALUES ($1, 'CodesysIec', '192.168.1.30', 502, 'plc', 'ModbusTcp')",
    )
    .bind(iec)
    .execute(&mut conn)
    .await
    .unwrap();
    let before = updated_at_of(&mut conn, IEC_CODE).await;

    run_0014(&mut conn).await;

    assert_eq!(updated_at_of(&mut conn, IEC_CODE).await, before);
}

#[tokio::test]
async fn migration_0014_creates_the_sync_trigger_after_the_copy() {
    let mut conn = connection().await;
    drop_the_shape_constraint(&mut conn).await;

    run_0014(&mut conn).await;

    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_trigger WHERE tgname = 'plc_connection_sync_db_number' \
         AND tgrelid = 'plc_connection'::regclass",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn migration_0014_does_not_overwrite_a_number_that_is_already_set() {
    let mut conn = connection().await;
    drop_the_shape_constraint(&mut conn).await;
    let set = machine(&mut conn, SET_CODE, "S7_1500", Some(SET_NUMBER)).await;
    s7_record(&mut conn, set, "S7_1500").await;

    run_0014(&mut conn).await;

    assert_eq!(db_number_of(&mut conn, SET_CODE).await, Some(SET_NUMBER));
}

#[tokio::test]
async fn migration_0014_waits_for_a_lock_on_any_machine_row_before_it_copies() {
    let database = TestDatabase::create().await;
    let mut holder = database.connect(Role::Migrator).await;
    let mut runner = database.connect(Role::Migrator).await;
    database.keep_until_thread_ends();
    drop_the_shape_constraint(&mut runner).await;
    // The copy never writes an IEC machine, so only the first statement can wait on its row.
    machine(&mut runner, IEC_CODE, "CodesysIec", None).await;
    sqlx::query("BEGIN").execute(&mut holder).await.unwrap();
    sqlx::query("SELECT id FROM machine WHERE code = $1 FOR NO KEY UPDATE")
        .bind(IEC_CODE)
        .fetch_one(&mut holder)
        .await
        .unwrap();

    let finished = AtomicBool::new(false);

    let run = async {
        run_0014(&mut runner).await;
        finished.store(true, Ordering::SeqCst);
    };
    let release = async {
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(
            !finished.load(Ordering::SeqCst),
            "0014 must wait for the machine row lock"
        );
        sqlx::query("COMMIT").execute(&mut holder).await.unwrap();
    };
    tokio::join!(run, release);
}

/// Wait until a session of the test database waits for a lock.
async fn wait_for_a_lock_waiter(observer: &mut PgConnection) {
    let mut found = false;
    for _ in 0..200 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE datname = current_database() AND wait_event_type = 'Lock'",
        )
        .fetch_one(&mut *observer)
        .await
        .unwrap();
        if waiting > 0 {
            found = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(found, "no session waits for a lock");
}

#[tokio::test]
async fn migration_0014_copies_a_number_that_a_concurrent_command_writes_while_it_waits() {
    let database = TestDatabase::create().await;
    let mut writer = database.connect(Role::Migrator).await;
    let mut runner = database.connect(Role::Migrator).await;
    let mut observer = database.connect(Role::Migrator).await;
    database.keep_until_thread_ends();
    drop_the_shape_constraint(&mut runner).await;
    let live = machine(&mut runner, COPIED_CODE, "S7_1500", None).await;
    s7_record(&mut runner, live, "S7_1500").await;
    // The previous release locks the machine row, then writes the record number only.
    sqlx::query("BEGIN").execute(&mut writer).await.unwrap();
    sqlx::query("SELECT id FROM machine WHERE id = $1 FOR NO KEY UPDATE")
        .bind(live)
        .fetch_one(&mut writer)
        .await
        .unwrap();
    sqlx::query("UPDATE plc_connection SET db_number = $1 WHERE machine_id = $2")
        .bind(NEW_RECORD_NUMBER)
        .bind(live)
        .execute(&mut writer)
        .await
        .unwrap();

    let run = run_0014(&mut runner);
    let release = async {
        wait_for_a_lock_waiter(&mut observer).await;
        sqlx::query("COMMIT").execute(&mut writer).await.unwrap();
    };
    tokio::join!(run, release);

    assert_eq!(
        db_number_of(&mut runner, COPIED_CODE).await,
        Some(NEW_RECORD_NUMBER)
    );
}

#[test]
fn migration_0014_trigger_function_equals_section_4_13_of_the_document() {
    let document = crate::support::document_sql(&["4.13"], &[]);
    let file = crate::support::migration_sql("0014_machine_db_number_copy.sql");

    assert!(document.starts_with("CREATE FUNCTION plc_connection_sync_db_number()"));
    assert!(
        file.contains(&document),
        "0014 lacks the SQL of section 4.13"
    );
}
