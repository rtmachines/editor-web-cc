//! Tests of the migration `0013_machine_db_number.sql` (ADR-0156, design D2).
//!
//! The first tests compare the constraints of `data-model.md` section 3.1 with the file. The other
//! tests run the file on a schema that is put back to the state of `0012`.

use std::time::Duration;

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::{Role, TestDatabase, check_constraint, document_sql, migration_sql};

const MIGRATION: &str = "0013_machine_db_number.sql";
const S7_1500: &str = "S7_1500";
const CHECKS: [&str; 2] = ["machine_db_number_shape", "machine_db_number_range"];
const LIVE_CODE: &str = "M12";
const ARCHIVED_CODE: &str = "M9001";
const CONNECTED_CODE: &str = "M9002";
/// The text that the check function raises for one live S7 machine `M12` without a record.
const STOP_MESSAGE: &str = "migration 0013 stopped: live S7 machines without a PLC connection \
    record: M12. Save the PLC connection of each listed machine with the previous release \
    (docs/ops/runbook.md).";

#[test]
fn migration_0013_adds_the_constraints_of_section_3_1_as_not_valid() {
    let document = document_sql(&["3.1"], &[]);
    let file = migration_sql(MIGRATION);

    for name in CHECKS {
        let in_document = check_constraint(&document, name);
        let in_file = check_constraint(&file, name);
        assert_eq!(in_file, in_document, "constraint {name}");
        assert!(
            file.contains(&format!("{in_document} NOT VALID")),
            "constraint {name} must be NOT VALID in 0013"
        );
    }
}

#[test]
fn migration_0013_adds_the_column_of_section_3_1_and_the_grant_of_section_5_2() {
    let document = document_sql(&["3.1"], &[]);
    let file = migration_sql(MIGRATION);

    assert!(document.contains(" db_number INTEGER,"), "{document}");
    assert!(
        file.contains("ALTER TABLE machine ADD COLUMN db_number INTEGER;"),
        "0013 lacks the column"
    );
    assert!(
        file.contains("GRANT UPDATE (db_number) ON machine TO recipe_app;"),
        "0013 lacks the grant"
    );
}

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await
}

/// Put the schema back to the state before `0013`: no column, no constraint, and no sync trigger
/// of `0014`. The trigger names the column, so it goes first.
async fn reset_to_the_state_of_0012(conn: &mut PgConnection) {
    sqlx::raw_sql(
        "DROP TRIGGER IF EXISTS plc_connection_sync_db_number ON plc_connection; \
         DROP FUNCTION IF EXISTS plc_connection_sync_db_number(); \
         ALTER TABLE machine \
         DROP CONSTRAINT IF EXISTS machine_db_number_shape, \
         DROP CONSTRAINT IF EXISTS machine_db_number_range, \
         DROP COLUMN IF EXISTS db_number",
    )
    .execute(conn)
    .await
    .unwrap();
}

/// Run the whole text of `0013` as one simple query, so it commits or rolls back as one unit.
async fn run_0013(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/0013_machine_db_number.sql"
    ))
    .unwrap();
    sqlx::raw_sql(&text).execute(conn).await.map(|_| ())
}

/// Insert an S7 machine with a fixed code, with or without a connection record.
///
/// The schema is at the state of 0012 here, so the insert names no `db_number` column.
async fn s7_machine(conn: &mut PgConnection, code: &str, has_record: bool) -> Uuid {
    let machine: Uuid = sqlx::query_scalar(
        "INSERT INTO machine (code, name, plc_platform) VALUES ($1, 'machine', $2) RETURNING id",
    )
    .bind(code)
    .bind(S7_1500)
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    if has_record {
        sqlx::query(
            "INSERT INTO plc_connection (machine_id, plc_platform, host, rack, slot, db_number) \
             VALUES ($1, 'S7_1500', '192.168.1.20', 0, 2, 10)",
        )
        .bind(machine)
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    machine
}

/// Return whether `machine.db_number` exists.
async fn column_exists(conn: &mut PgConnection) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
         WHERE table_name = 'machine' AND column_name = 'db_number')",
    )
    .fetch_one(conn)
    .await
    .unwrap()
}

#[tokio::test]
async fn migration_0013_stops_on_a_live_s7_machine_without_a_record_and_changes_nothing() {
    let mut conn = connection().await;
    reset_to_the_state_of_0012(&mut conn).await;
    s7_machine(&mut conn, LIVE_CODE, false).await;
    let archived = s7_machine(&mut conn, ARCHIVED_CODE, false).await;
    sqlx::query("UPDATE machine SET archived_at = now() WHERE id = $1")
        .bind(archived)
        .execute(&mut conn)
        .await
        .unwrap();
    s7_machine(&mut conn, CONNECTED_CODE, true).await;

    let error = run_0013(&mut conn)
        .await
        .expect_err("the check function stops the file");

    let message = error
        .as_database_error()
        .expect("a database error")
        .message();
    assert_eq!(message, STOP_MESSAGE);
    assert!(!message.contains(ARCHIVED_CODE), "{message}");
    assert!(!message.contains(CONNECTED_CODE), "{message}");
    assert!(
        !column_exists(&mut conn).await,
        "machine.db_number rolled back with the file"
    );
}

#[tokio::test]
async fn migration_0013_runs_when_each_live_s7_machine_has_a_record() {
    let mut conn = connection().await;
    reset_to_the_state_of_0012(&mut conn).await;
    s7_machine(&mut conn, LIVE_CODE, true).await;
    let archived = s7_machine(&mut conn, ARCHIVED_CODE, false).await;
    sqlx::query("UPDATE machine SET archived_at = now() WHERE id = $1")
        .bind(archived)
        .execute(&mut conn)
        .await
        .unwrap();

    run_0013(&mut conn)
        .await
        .expect("each live S7 machine has a record");

    assert!(column_exists(&mut conn).await, "the column exists");
    let states: Vec<(String, bool)> = sqlx::query_as(
        "SELECT conname::text, convalidated FROM pg_constraint \
         WHERE conrelid = 'machine'::regclass \
           AND conname IN ('machine_db_number_shape', 'machine_db_number_range') \
         ORDER BY conname",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();
    assert_eq!(
        states,
        vec![
            ("machine_db_number_range".to_owned(), false),
            ("machine_db_number_shape".to_owned(), false),
        ]
    );
    let granted: bool = sqlx::query_scalar(
        "SELECT has_column_privilege('recipe_app', 'machine', 'db_number', 'UPDATE')",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert!(granted, "recipe_app has UPDATE (db_number)");
    let function_left: bool =
        sqlx::query_scalar("SELECT to_regprocedure('machine_db_number_check()') IS NOT NULL")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert!(!function_left, "the check function is dropped");
}

#[tokio::test]
async fn migration_0013_runs_with_a_live_iec_machine_without_a_record() {
    let mut conn = connection().await;
    reset_to_the_state_of_0012(&mut conn).await;
    let machine: Uuid = sqlx::query_scalar(
        "INSERT INTO machine (code, name, plc_platform) \
         VALUES ('M9003', 'IEC machine', 'CodesysIec') RETURNING id",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();

    run_0013(&mut conn)
        .await
        .expect("a live IEC machine needs no PLC connection record");

    assert!(column_exists(&mut conn).await, "the column exists");
    let state: (bool, Option<i32>, bool) = sqlx::query_as(
        "SELECT archived_at IS NULL, db_number, \
         NOT EXISTS (SELECT 1 FROM plc_connection WHERE machine_id = machine.id) \
         FROM machine WHERE id = $1",
    )
    .bind(machine)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(state, (true, None, true));
}

/// The pause between two reads of `pg_locks`.
const POLL_PAUSE: Duration = Duration::from_millis(20);
/// The number of reads before the case gives up. The bound is about 5 s.
const POLL_LIMIT: u32 = 250;

/// Poll until the backend `pid` waits for a lock, and fail after the bound.
async fn wait_for_lock_wait(observer: &mut PgConnection, pid: i32) {
    let mut waits = false;
    for _ in 0..POLL_LIMIT {
        waits = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_locks WHERE pid = $1 AND NOT granted)",
        )
        .bind(pid)
        .fetch_one(&mut *observer)
        .await
        .unwrap();
        if waits {
            break;
        }
        tokio::time::sleep(POLL_PAUSE).await;
    }
    assert!(waits, "session {pid} never waited for a lock");
}

/// A previous release inserts a live S7 machine without a record, and it commits after `0013` starts.
///
/// The file must lock `machine` before the check function reads it. Then the file waits for the
/// insert, reads the committed machine, and stops.
#[tokio::test]
async fn migration_0013_stops_on_a_machine_that_a_running_previous_release_commits_during_the_file()
{
    let database = TestDatabase::create().await;
    let mut session_a = database.connect(Role::Migrator).await;
    let mut session_b = database.connect(Role::Migrator).await;
    let mut observer = database.connect(Role::Migrator).await;
    reset_to_the_state_of_0012(&mut observer).await;
    let pid_a: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut session_a)
        .await
        .unwrap();
    sqlx::query("BEGIN").execute(&mut session_b).await.unwrap();
    s7_machine(&mut session_b, LIVE_CODE, false).await;

    // Session A runs the file. Session B commits after A waits for a lock.
    let release = async {
        wait_for_lock_wait(&mut observer, pid_a).await;
        sqlx::query("COMMIT").execute(&mut session_b).await.unwrap();
    };
    let (result, ()) = tokio::join!(run_0013(&mut session_a), release);

    let error = result.expect_err("the check function sees the machine that session B committed");
    let message = error
        .as_database_error()
        .expect("a database error")
        .message();
    assert_eq!(message, STOP_MESSAGE);
    assert!(
        !column_exists(&mut observer).await,
        "machine.db_number rolled back with the file"
    );
    database.keep_until_thread_ends();
}

fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|e| e.code().map(|c| c.into_owned()))
}

/// Asserts that a result is neither a deadlock nor any other error,.
fn assert_no_deadlock<T: std::fmt::Debug>(result: &Result<T, sqlx::Error>, who: &str) {
    const DEADLOCK: &str = "40P01";
    let state = result.as_ref().err().and_then(sqlstate);
    assert_ne!(state.as_deref(), Some(DEADLOCK), "{who}: {result:?}");
}

/// A previous release locks a machine row, and its next statement writes it after `0013` starts.
///
/// The command holds `ROW SHARE` on `machine` from its `SELECT ... FOR NO KEY UPDATE`. Then its
/// `UPDATE` needs `ROW EXCLUSIVE`. The file must not hold a mode that blocks that `UPDATE` while
/// its own `ALTER TABLE` waits for the command. Else the two sessions deadlock (SQLSTATE `40P01`).
#[tokio::test]
async fn migration_0013_does_not_deadlock_with_a_running_previous_release_command() {
    let database = TestDatabase::create().await;
    let mut session_a = database.connect(Role::Migrator).await;
    let mut session_b = database.connect(Role::Migrator).await;
    let mut observer = database.connect(Role::Migrator).await;
    reset_to_the_state_of_0012(&mut observer).await;
    s7_machine(&mut observer, LIVE_CODE, true).await;
    let pid_a: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut session_a)
        .await
        .unwrap();
    sqlx::query("BEGIN").execute(&mut session_b).await.unwrap();
    let machine: Uuid =
        sqlx::query_scalar("SELECT id FROM machine WHERE code = $1 FOR NO KEY UPDATE")
            .bind(LIVE_CODE)
            .fetch_one(&mut session_b)
            .await
            .unwrap();

    // Session A runs the file. Session B writes the locked row after A waits for a lock.
    let command = async {
        wait_for_lock_wait(&mut observer, pid_a).await;
        let update = sqlx::query("UPDATE machine SET name = 'renamed' WHERE id = $1")
            .bind(machine)
            .execute(&mut session_b)
            .await;
        // After an error the transaction is aborted, and COMMIT then rolls it back.
        sqlx::query("COMMIT").execute(&mut session_b).await.unwrap();
        update
    };
    let (file, update) = tokio::join!(run_0013(&mut session_a), command);

    assert_no_deadlock(&update, "command");
    assert_no_deadlock(&file, "file");
    update.expect("the command updates its locked machine");
    file.expect("the file waits for the command and then runs");
    assert!(
        column_exists(&mut observer).await,
        "the file added the column"
    );
    database.keep_until_thread_ends();
}
