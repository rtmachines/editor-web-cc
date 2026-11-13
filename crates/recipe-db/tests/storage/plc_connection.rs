//! Tests of the table `plc_connection` of migration `0012_plc_connection.sql` (ADR-0154 (b),
//! design D1, task 4.1).
//!
//! Each violating row changes one column of a valid row, so the named constraint is the only one
//! that fails. The tests run as `recipe_migrator`, the owner of the tables.

mod shape;
mod trigger;
mod values;

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{
    CHECK_VIOLATION, FOREIGN_KEY_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION,
    assert_constraint, assert_privilege_denied,
};

const S7_1500: &str = "S7_1500";
const S7_300: &str = "S7_300";
const S7_1200: &str = "S7_1200";
const CODESYS: &str = "CodesysIec";

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await
}

/// Insert an S7 record for a machine. `port` is the only column that a test changes on this row.
async fn insert_s7(
    conn: &mut PgConnection,
    machine: Uuid,
    platform: &str,
    port: Option<i32>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, rack, slot, db_number, port) \
         VALUES ($1, $2, '192.168.1.20', 0, 2, 10, $3) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(port)
    .fetch_one(conn)
    .await
}

/// Insert an IEC record for a machine. `rack` is the only column that a test changes on this row.
async fn insert_iec(
    conn: &mut PgConnection,
    machine: Uuid,
    platform: &str,
    rack: Option<i16>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO plc_connection \
             (machine_id, plc_platform, host, rack, port, address, comm_mode) \
         VALUES ($1, $2, 'plc-line1.plant.local', $3, 4840, '5.12.34.56.1.1', 'Ads') RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(rack)
    .fetch_one(conn)
    .await
}

/// Assert that the row was accepted.
fn assert_accepted(case: &str, result: Result<Uuid, sqlx::Error>) {
    assert!(result.is_ok(), "{case}: {result:?}");
}

/// Assert that the row violates the check `constraint`. `case` names the tested value.
fn assert_check(case: &str, result: Result<Uuid, sqlx::Error>, constraint: &str) {
    let error = result.expect_err(&format!("{case}: the check rejects the row"));
    let database = error
        .as_database_error()
        .expect("the failure is a database error");
    assert_eq!(
        database.code().as_deref(),
        Some(CHECK_VIOLATION),
        "{case}: {error}"
    );
    assert_eq!(database.constraint(), Some(constraint), "{case}: {error}");
}

/// Assert the verdict of one case: accepted, or a violation of `constraint`.
fn assert_verdict(case: &str, result: Result<Uuid, sqlx::Error>, is_valid: bool, constraint: &str) {
    if is_valid {
        assert_accepted(case, result);
    } else {
        assert_check(case, result, constraint);
    }
}
