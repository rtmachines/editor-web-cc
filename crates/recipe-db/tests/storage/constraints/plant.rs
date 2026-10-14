//! Tests of `machine` and `unit` (`data-model.md` sections 3.1 and 3.2, task 7.1).
//!
//! Each violating row changes one column of a valid row, so the named constraint is the only one
//! that fails. The tests run as `recipe_migrator`, the owner of the tables.
//!
//! `machine_id_platform_unique` and `unit_machine_id_unique` have no test of their own. The primary
//! key on `id` rejects every duplicate id first, so these unique constraints cannot fail alone.
//! A foreign key of `recipe_machine_platform` targets the first one. No foreign key of section 3
//! targets the second one, because trigger 4.3 does the scope check.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{
    CHECK_VIOLATION, FOREIGN_KEY_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION, assert_constraint,
};

/// The constraints and unique indexes that the tests of this module violate.
pub const COVERED: &[&str] = &[
    "machine_pkey",
    "machine_code_format",
    "machine_name_length",
    "machine_platform_value",
    "machine_code_live_unique",
    "unit_pkey",
    "unit_path_format",
    "unit_depth_range",
    "unit_name_length",
    "unit_path_unique",
    "unit_parent_exists",
    "unit_machine_id_fkey",
];

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    let database = TestDatabase::create().await;
    database.connect(Role::Migrator).await
}

/// Insert a machine with a code, a name, and a platform. Return its id.
async fn insert_machine(
    conn: &mut PgConnection,
    code: &str,
    name: &str,
    platform: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO machine (code, name, plc_platform) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(code)
    .bind(name)
    .bind(platform)
    .fetch_one(conn)
    .await
}

/// Set the archive time of a machine.
async fn archive_machine(conn: &mut PgConnection, machine: Uuid) {
    sqlx::query("UPDATE machine SET archived_at = now() WHERE id = $1")
        .bind(machine)
        .execute(conn)
        .await
        .unwrap();
}

/// Insert a unit with a path and a name. Return its id.
async fn insert_unit(
    conn: &mut PgConnection,
    machine: Uuid,
    path: &str,
    name: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO unit (machine_id, path, name) VALUES ($1, $2, $3) RETURNING id")
        .bind(machine)
        .bind(path)
        .bind(name)
        .fetch_one(conn)
        .await
}

// ---- machine ----

#[tokio::test]
async fn machine_row_with_valid_values_inserts() {
    let mut conn = connection().await;

    insert_machine(&mut conn, "M7000", "Press", TWIN_CAT)
        .await
        .unwrap();
}

#[tokio::test]
async fn machine_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = connection().await;
    let id = insert_machine(&mut conn, "M7001", "Press", TWIN_CAT)
        .await
        .unwrap();

    let result = sqlx::query(
        "INSERT INTO machine (id, code, name, plc_platform) VALUES ($1, 'M7002', 'Press', $2)",
    )
    .bind(id)
    .bind(TWIN_CAT)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "machine_pkey");
}

#[tokio::test]
async fn machine_code_format_rejects_a_code_that_starts_with_zero() {
    let mut conn = connection().await;

    let result = insert_machine(&mut conn, "M0", "Press", TWIN_CAT).await;

    assert_constraint(result, CHECK_VIOLATION, "machine_code_format");
}

#[tokio::test]
async fn machine_name_length_rejects_an_empty_name() {
    let mut conn = connection().await;

    let result = insert_machine(&mut conn, "M7003", "", TWIN_CAT).await;

    assert_constraint(result, CHECK_VIOLATION, "machine_name_length");
}

#[tokio::test]
async fn machine_platform_value_rejects_an_unknown_platform() {
    let mut conn = connection().await;

    let result = insert_machine(&mut conn, "M7004", "Press", "S7_400").await;

    assert_constraint(result, CHECK_VIOLATION, "machine_platform_value");
}

#[tokio::test]
async fn machine_code_live_unique_rejects_a_second_live_machine_with_the_same_code() {
    let mut conn = connection().await;
    insert_machine(&mut conn, "M7005", "Press", TWIN_CAT)
        .await
        .unwrap();

    let result = insert_machine(&mut conn, "M7005", "Press 2", TWIN_CAT).await;

    assert_constraint(result, UNIQUE_VIOLATION, "machine_code_live_unique");
}

#[tokio::test]
async fn an_archived_machine_frees_its_code() {
    let mut conn = connection().await;
    let first = insert_machine(&mut conn, "M7006", "Press", TWIN_CAT)
        .await
        .unwrap();
    archive_machine(&mut conn, first).await;

    insert_machine(&mut conn, "M7006", "Press 2", TWIN_CAT)
        .await
        .unwrap();
}

#[tokio::test]
async fn two_archived_machines_can_share_a_code() {
    let mut conn = connection().await;
    let first = insert_machine(&mut conn, "M7007", "Press", TWIN_CAT)
        .await
        .unwrap();
    archive_machine(&mut conn, first).await;
    let second = insert_machine(&mut conn, "M7007", "Press 2", TWIN_CAT)
        .await
        .unwrap();
    archive_machine(&mut conn, second).await;

    let archived: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM machine WHERE code = 'M7007' AND archived_at IS NOT NULL",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(archived, 2);
}

// ---- unit ----

#[tokio::test]
async fn unit_rows_of_a_main_unit_and_its_child_insert() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    fixtures::unit(&mut conn, machine, "U1").await;

    insert_unit(&mut conn, machine, "U1.2", "child")
        .await
        .unwrap();
}

#[tokio::test]
async fn unit_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let id = fixtures::unit(&mut conn, machine, "U1").await;

    let result =
        sqlx::query("INSERT INTO unit (id, machine_id, path, name) VALUES ($1, $2, 'U9', 'copy')")
            .bind(id)
            .bind(machine)
            .execute(&mut conn)
            .await;

    assert_constraint(result, UNIQUE_VIOLATION, "unit_pkey");
}

#[tokio::test]
async fn unit_path_format_rejects_a_segment_of_zero() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = insert_unit(&mut conn, machine, "U0", "unit").await;

    assert_constraint(result, CHECK_VIOLATION, "unit_path_format");
}

#[tokio::test]
async fn unit_depth_range_rejects_a_path_of_six_segments() {
    // The path regex also rejects six segments. PostgreSQL checks the CHECK constraints in
    // name order, and `unit_depth_range` sorts before `unit_path_format`.
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = insert_unit(&mut conn, machine, "U1.2.3.4.5.6", "unit").await;

    assert_constraint(result, CHECK_VIOLATION, "unit_depth_range");
}

#[tokio::test]
async fn unit_name_length_rejects_an_empty_name() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = insert_unit(&mut conn, machine, "U1", "").await;

    assert_constraint(result, CHECK_VIOLATION, "unit_name_length");
}

#[tokio::test]
async fn unit_path_unique_rejects_a_second_unit_with_the_same_path_in_a_machine() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    fixtures::unit(&mut conn, machine, "U1").await;

    let result = insert_unit(&mut conn, machine, "U1", "again").await;

    assert_constraint(result, UNIQUE_VIOLATION, "unit_path_unique");
}

#[tokio::test]
async fn unit_parent_exists_rejects_a_child_without_its_parent() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = insert_unit(&mut conn, machine, "U2.1", "orphan").await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "unit_parent_exists");
}

#[tokio::test]
async fn unit_machine_id_fkey_rejects_an_unknown_machine() {
    let mut conn = connection().await;

    let result = insert_unit(&mut conn, Uuid::nil(), "U1", "unit").await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "unit_machine_id_fkey");
}
