//! Postgres only: the row locks of the methods of `PlcConnections` (task 4.7, `data-model.md` section 6).
//!
//! A probe tries one row lock with `NOWAIT` from a second session. The SQLSTATE `55P03` shows
//! that the adapter holds a conflicting lock.

use recipe_domain::plant::{MachineId, PlcPlatform};
use uuid::Uuid;

use super::fixtures::{iec_connection, insert_connection, s7_connection, save_machine, sealed};
use crate::contract::harness::begin;
use crate::contract::images::text;
use crate::contract::locks::{LOCK_NOT_AVAILABLE, try_row_lock};
use crate::contract::postgres_harness::PgHarness;
use crate::support::Role;

/// Create a harness and insert an S7 record. Return the machine and the id of the record.
async fn harness_with_record(n: u128) -> (PgHarness, MachineId, Uuid) {
    let h = PgHarness::create().await;
    let machine = save_machine(&h, n, &format!("M{n}"), PlcPlatform::S7_1500).await;
    let image = insert_connection(
        &h,
        machine,
        &s7_connection("192.168.1.20", "0", "2", "10"),
        None,
    )
    .await;
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
