//! Postgres only: the row locks of the methods of `PlcConnections` (task 4.7, `data-model.md` section 6).
//!
//! A probe tries one row lock with `NOWAIT` from a second session. The SQLSTATE `55P03` shows
//! that the adapter holds a conflicting lock.

use recipe_domain::plant::{DataBlockNumber, MachineId, PlcPlatform};
use uuid::Uuid;

use super::fixtures::{iec_connection, insert_connection, s7_connection, save_machine, sealed};
use crate::contract::harness::begin;
use crate::contract::images::text;
use crate::contract::locks::{LOCK_NOT_AVAILABLE, try_row_lock};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::seed::code;
use crate::support::Role;

/// Create a harness and insert an S7 record. Return the machine and the id of the record.
async fn harness_with_record(n: u128) -> (PgHarness, MachineId, Uuid) {
    let h = PgHarness::create().await;
    let machine = save_machine(&h, n, &format!("M{n}"), PlcPlatform::S7_1500).await;
    let image =
        insert_connection(&h, machine, &s7_connection("192.168.1.20", "0", "2"), None).await;
    let id = Uuid::parse_str(text(&image, "id")).unwrap();
    (h, machine, id)
}

#[tokio::test]
async fn postgres_only_lock_connection_for_update_holds_for_update_on_the_row() {
    let (h, machine, id) = harness_with_record(60).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let mut tx = begin(&h).await;

    tx.plc_connections()
        .lock_connection_for_update(machine)
        .await
        .unwrap()
        .expect("the record exists");

    for mode in ["UPDATE", "NO KEY UPDATE", "SHARE", "KEY SHARE"] {
        assert_eq!(
            try_row_lock(&mut session, "plc_connection", mode, id)
                .await
                .as_deref(),
            Some(LOCK_NOT_AVAILABLE),
            "FOR {mode}"
        );
    }
}

#[tokio::test]
async fn postgres_only_lock_connection_shared_holds_for_share_on_the_row() {
    let h = PgHarness::create().await;
    let machine = save_machine(&h, 61, "M61", PlcPlatform::TwinCat3).await;
    let image = insert_connection(
        &h,
        machine,
        &iec_connection(
            "plc-line1.plant.local",
            "4840",
            "urn:plc:1",
            "OpcUa",
            Some("operator"),
        ),
        Some(&sealed(&[1, 2, 3])),
    )
    .await;
    let id = Uuid::parse_str(text(&image, "id")).unwrap();
    let mut session = h.database.connect(Role::Migrator).await;
    let mut tx = begin(&h).await;

    tx.plc_connections()
        .lock_connection_shared(machine)
        .await
        .unwrap()
        .expect("the record exists");

    for mode in ["UPDATE", "NO KEY UPDATE"] {
        assert_eq!(
            try_row_lock(&mut session, "plc_connection", mode, id)
                .await
                .as_deref(),
            Some(LOCK_NOT_AVAILABLE),
            "FOR {mode}"
        );
    }
    for mode in ["SHARE", "KEY SHARE"] {
        assert_eq!(
            try_row_lock(&mut session, "plc_connection", mode, id).await,
            None,
            "FOR {mode}"
        );
    }
}

#[tokio::test]
async fn postgres_only_connection_by_machine_takes_no_row_lock() {
    let (h, machine, id) = harness_with_record(62).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let mut tx = begin(&h).await;

    tx.plc_connections()
        .connection_by_machine(machine)
        .await
        .unwrap()
        .expect("the record exists");

    assert_eq!(
        try_row_lock(&mut session, "plc_connection", "UPDATE", id).await,
        None
    );
}

/// A drift between `plc_connection.db_number` and `machine.db_number` never reaches the export.
/// The copy is a stored fact of the expand step. `ExportPlcConnection` reads the machine (ADR-0156 (g)).
#[tokio::test]
async fn postgres_only_export_reads_the_machine_number_when_the_copy_drifts() {
    let (h, machine, _id) = harness_with_record(63).await;
    let mut session = h.database.connect(Role::Migrator).await;
    // The trigger of 0014 keeps the copy equal. The case switches it off to make the drift.
    sqlx::query("ALTER TABLE plc_connection DISABLE TRIGGER plc_connection_sync_db_number")
        .execute(&mut session)
        .await
        .unwrap();
    let drifted = sqlx::query("UPDATE plc_connection SET db_number = 999 WHERE machine_id = $1")
        .bind(machine.as_uuid())
        .execute(&mut session)
        .await
        .unwrap();
    assert_eq!(drifted.rows_affected(), 1);
    let mut tx = begin(&h).await;

    let locked = tx
        .plant()
        .lock_machine_shared(machine)
        .await
        .unwrap()
        .expect("the machine exists");
    let resolved = tx
        .plant()
        .export_machine_by_code(code("M63"))
        .await
        .unwrap()
        .expect("the machine exists");
    tx.plc_connections()
        .lock_connection_shared(machine)
        .await
        .unwrap()
        .expect("the record exists");

    let expected = Some(DataBlockNumber::parse("1").unwrap());
    assert_eq!(locked.state.db_number(), expected);
    assert_eq!(resolved.db_number(), expected);
}
