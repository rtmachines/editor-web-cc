//! Postgres only: the trigger `plc_connection_sync_db_number` of migration `0014` (ADR-0157).
//!
//! During the expand period the previous release writes `plc_connection.db_number` only. The
//! trigger keeps `machine.db_number` equal. The cases run as `recipe_app`, the role of the release.
//! The record has the same limits as the machine, so its range check fails first (ADR-0157 (d)).

use recipe_app::ports::PasswordWrite;
use recipe_domain::plant::{MachineId, PlcPlatform};
use sqlx::PgConnection;
use time::OffsetDateTime;

use super::fixtures::{insert_connection, s7_connection, save_machine};
use crate::contract::harness::{begin, commit};
use crate::contract::postgres_harness::PgHarness;
use crate::support::{CHECK_VIOLATION, Role};

const FIRST_NUMBER: i32 = 100;
const SECOND_NUMBER: i32 = 200;
const OUT_OF_RANGE_NUMBER: i32 = 20_000;
const THIRD_NUMBER: i32 = 300;
const FOURTH_NUMBER: i32 = 400;

/// Create a harness, a machine of the platform with the number `100`, and its S7 record.
async fn harness_with_record(n: u128, platform: PlcPlatform) -> (PgHarness, MachineId) {
    let h = PgHarness::create().await;
    let machine = save_machine(&h, n, &format!("M{n}"), platform).await;
    let mut owner = h.database.connect(Role::Migrator).await;
    set_machine_number(&mut owner, machine, FIRST_NUMBER).await;
    insert_connection(&h, machine, &s7_connection("192.168.1.20", "0", "2"), None).await;
    (h, machine)
}

/// Set `machine.db_number` as the owner of the tables.
async fn set_machine_number(conn: &mut PgConnection, machine: MachineId, number: i32) {
    sqlx::query("UPDATE machine SET db_number = $2 WHERE id = $1")
        .bind(*machine.as_uuid())
        .bind(number)
        .execute(conn)
        .await
        .unwrap();
}

/// The statement of the previous release: it writes the record only.
async fn previous_release_update(
    conn: &mut PgConnection,
    machine: MachineId,
    number: i32,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query("UPDATE plc_connection SET db_number = $2 WHERE machine_id = $1")
        .bind(*machine.as_uuid())
        .bind(number)
        .execute(conn)
        .await
}

/// Return `machine.db_number` and `plc_connection.db_number` of a machine.
async fn numbers(conn: &mut PgConnection, machine: MachineId) -> (Option<i32>, Option<i32>) {
    sqlx::query_as(
        "SELECT m.db_number, c.db_number FROM machine m \
         JOIN plc_connection c ON c.machine_id = m.id WHERE m.id = $1",
    )
    .bind(*machine.as_uuid())
    .fetch_one(conn)
    .await
    .unwrap()
}

async fn machine_updated_at(conn: &mut PgConnection, machine: MachineId) -> OffsetDateTime {
    sqlx::query_scalar("SELECT updated_at FROM machine WHERE id = $1")
        .bind(*machine.as_uuid())
        .fetch_one(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn postgres_only_a_record_update_of_the_previous_release_moves_the_machine_number() {
    let (h, machine) = harness_with_record(70, PlcPlatform::S7_1500).await;
    let mut app = h.database.connect(Role::App).await;

    previous_release_update(&mut app, machine, SECOND_NUMBER)
        .await
        .unwrap();

    assert_eq!(
        numbers(&mut app, machine).await,
        (Some(SECOND_NUMBER), Some(SECOND_NUMBER))
    );
}

#[tokio::test]
async fn postgres_only_an_out_of_range_record_number_fails_and_changes_neither_table() {
    let (h, machine) = harness_with_record(71, PlcPlatform::S7_300).await;
    let mut app = h.database.connect(Role::App).await;

    let result = previous_release_update(&mut app, machine, OUT_OF_RANGE_NUMBER).await;

    let error = result.expect_err("the range check rejects the number");
    let database = error.as_database_error().expect("a database error");
    assert_eq!(database.code().as_deref(), Some(CHECK_VIOLATION), "{error}");
    assert_eq!(
        numbers(&mut app, machine).await,
        (Some(FIRST_NUMBER), Some(FIRST_NUMBER))
    );
}

#[tokio::test]
async fn postgres_only_update_connection_keeps_the_updated_at_of_the_machine() {
    let (h, machine) = harness_with_record(73, PlcPlatform::S7_1500).await;
    let mut app = h.database.connect(Role::App).await;
    let before = machine_updated_at(&mut app, machine).await;

    let mut tx = begin(&h).await;
    tx.plc_connections()
        .update_connection(
            machine,
            &s7_connection("192.168.1.21", "0", "2"),
            &PasswordWrite::Keep,
        )
        .await
        .unwrap();
    commit(&h, tx, machine).await;

    let host: String = sqlx::query_scalar("SELECT host FROM plc_connection WHERE machine_id = $1")
        .bind(*machine.as_uuid())
        .fetch_one(&mut app)
        .await
        .unwrap();
    assert_eq!(host, "192.168.1.21");
    assert_eq!(machine_updated_at(&mut app, machine).await, before);
}

/// The statement of the previous release for a first save: it inserts the record only.
async fn previous_release_insert(
    conn: &mut PgConnection,
    machine: MachineId,
    number: i32,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, rack, slot, db_number) \
         SELECT m.id, m.plc_platform, '192.168.1.22', 0, 2, $2 FROM machine m WHERE m.id = $1",
    )
    .bind(*machine.as_uuid())
    .bind(number)
    .execute(conn)
    .await
}

#[tokio::test]
async fn postgres_only_a_record_insert_of_the_previous_release_moves_the_machine_number() {
    let h = PgHarness::create().await;
    let machine = save_machine(&h, 74, "M74", PlcPlatform::S7_300).await;
    let mut owner = h.database.connect(Role::Migrator).await;
    set_machine_number(&mut owner, machine, THIRD_NUMBER).await;
    let mut app = h.database.connect(Role::App).await;

    previous_release_insert(&mut app, machine, FOURTH_NUMBER)
        .await
        .unwrap();

    assert_eq!(
        numbers(&mut app, machine).await,
        (Some(FOURTH_NUMBER), Some(FOURTH_NUMBER))
    );
}
