//! Contract cases of the machine methods of `PlantRepository` (tasks 9.1 and 9.2).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions.

use recipe_app::error::{MACHINE_CODE_LIVE_UNIQUE, StorageError};
use recipe_domain::plant::PlcPlatform;
use sqlx::PgConnection;
use uuid::Uuid;

use super::harness::{Harness, begin};
use super::images::{assert_shape, machine_shape, text, unit_shape, without_times};
use super::locks::{LOCK_NOT_AVAILABLE, try_row_lock};
use super::postgres_harness::PgHarness;
use super::seed::{
    archive, code, insert, machine, machine_id, machine_with_archived_unit, name, path, read,
    unit_id,
};
use crate::support::Role;

async fn insert_machine_then_lock_shared(h: &dyn Harness) {
    let machine = machine(1, "M1");
    let inserted = insert(h, &machine).await;
    let mut tx = begin(h).await;

    let locked = tx
        .plant()
        .lock_machine_shared(machine.id())
        .await
        .unwrap()
        .expect("the machine exists");

    assert_eq!(locked.state, machine);
    assert_shape(&locked.before, &machine_shape(false));
    assert_eq!(
        without_times(&locked.before),
        without_times(&inserted.machine)
    );
    assert_eq!(text(&locked.before, "id"), machine.id().to_string());
    assert_eq!(text(&locked.before, "code"), "M1");
    assert_eq!(text(&locked.before, "name"), "Press");
    assert_eq!(text(&locked.before, "plc_platform"), "TwinCat3");
}

async fn lock_for_no_key_update_gives_state_and_before_image(h: &dyn Harness) {
    let machine = machine(2, "M2");
    let inserted = insert(h, &machine).await;
    let mut tx = begin(h).await;

    let locked = tx
        .plant()
        .lock_machine_for_no_key_update(machine.id())
        .await
        .unwrap()
        .expect("the machine exists");

    assert_eq!(locked.state, machine);
    assert_shape(&locked.before, &machine_shape(false));
    assert_eq!(
        without_times(&locked.before),
        without_times(&inserted.machine)
    );
}

async fn lock_of_a_missing_machine_gives_none(h: &dyn Harness) {
    let mut tx = begin(h).await;

    let shared = tx
        .plant()
        .lock_machine_shared(machine_id(99))
        .await
        .unwrap();
    let exclusive = tx
        .plant()
        .lock_machine_for_no_key_update(machine_id(99))
        .await
        .unwrap();

    assert!(shared.is_none());
    assert!(exclusive.is_none());
}

async fn insert_machine_gives_the_images_of_the_machine_and_its_units(h: &dyn Harness) {
    let mut built = machine(3, "M3");
    built
        .add_unit(unit_id(31), None, "2", name("Second main"))
        .unwrap();
    let main = path("U1");
    built
        .add_unit(unit_id(32), Some(&main), "1", name("Child"))
        .unwrap();

    let inserted = insert(h, &built).await;

    assert_shape(&inserted.machine, &machine_shape(false));
    assert_eq!(text(&inserted.machine, "code"), "M3");
    let paths: Vec<&str> = inserted
        .units
        .iter()
        .map(|unit| text(unit, "path"))
        .collect();
    assert_eq!(paths, vec!["U1", "U1.1", "U2"]);
    let expected_order: Vec<String> = built.units().map(|unit| unit.path().to_string()).collect();
    assert_eq!(paths, expected_order);
    assert_shape(&inserted.units[0], &unit_shape(false, false));
    assert_shape(&inserted.units[1], &unit_shape(true, false));
    assert_eq!(text(&inserted.units[1], "parent_path"), "U1");
    assert_eq!(inserted.units[1].as_value()["depth"], 2);
    assert_eq!(
        text(&inserted.units[0], "machine_id"),
        built.id().to_string()
    );
    assert_eq!(read(h, built.id()).await, Some(built));
}

async fn an_archived_unit_survives_the_round_trip(h: &dyn Harness) {
    let built = machine_with_archived_unit(4, "M4");

    let inserted = insert(h, &built).await;

    assert_shape(&inserted.units[0], &unit_shape(false, false));
    assert_shape(&inserted.units[1], &unit_shape(false, true));
    assert_eq!(read(h, built.id()).await, Some(built));
}

async fn a_live_duplicate_code_gives_the_constraint_of_the_live_index(h: &dyn Harness) {
    insert(h, &machine(5, "M5")).await;
    let mut tx = begin(h).await;

    let error = tx
        .plant()
        .insert_machine(&machine(6, "M5"))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint }
            if constraint == MACHINE_CODE_LIVE_UNIQUE),
        "{error:?}"
    );
}

async fn an_archived_machine_frees_its_code(h: &dyn Harness) {
    let first = machine(7, "M7");
    insert(h, &first).await;
    archive(h, first.id()).await;

    let second = machine(8, "M7");
    insert(h, &second).await;

    let mut tx = begin(h).await;
    let live = tx.plant().machine_by_code(code("M7")).await.unwrap();
    assert_eq!(live, Some(second));
}

async fn machine_by_code_ignores_archived_machines(h: &dyn Harness) {
    let archived = machine(10, "M10");
    insert(h, &archived).await;
    archive(h, archived.id()).await;
    let mut tx = begin(h).await;

    assert!(
        tx.plant()
            .machine_by_code(code("M10"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        tx.plant()
            .machine_by_code(code("M11"))
            .await
            .unwrap()
            .is_none()
    );
}

async fn archived_machine_by_code_returns_the_first_by_id(h: &dyn Harness) {
    // The larger identifier goes in first, so the order of the result is not the insert order.
    for n in [20, 12] {
        let built = machine(n, "M12");
        insert(h, &built).await;
        archive(h, built.id()).await;
    }
    let live = machine(30, "M12");
    insert(h, &live).await;
    let mut tx = begin(h).await;

    let archived = tx
        .plant()
        .archived_machine_by_code(code("M12"))
        .await
        .unwrap();
    let current = tx.plant().machine_by_code(code("M12")).await.unwrap();
    let none = tx
        .plant()
        .archived_machine_by_code(code("M13"))
        .await
        .unwrap();

    assert_eq!(archived.map(|found| found.id()), Some(machine_id(12)));
    assert_eq!(current, Some(live));
    assert!(none.is_none());
}

async fn a_live_machine_gives_no_archived_machine_by_code(h: &dyn Harness) {
    insert(h, &machine(14, "M14")).await;
    let mut tx = begin(h).await;

    let found = tx
        .plant()
        .archived_machine_by_code(code("M14"))
        .await
        .unwrap();

    assert!(found.is_none());
}

async fn machine_by_id_reads_live_and_archived_machines(h: &dyn Harness) {
    let live = machine(15, "M15");
    let archived = machine(16, "M16");
    insert(h, &live).await;
    insert(h, &archived).await;
    archive(h, archived.id()).await;
    let mut tx = begin(h).await;

    let found_live = tx.plant().machine_by_id(live.id()).await.unwrap();
    let found_archived = tx.plant().machine_by_id(archived.id()).await.unwrap();
    let missing = tx.plant().machine_by_id(machine_id(99)).await.unwrap();

    assert_eq!(found_live, Some(live));
    let found_archived = found_archived.expect("the archived machine is readable");
    assert!(!found_archived.is_live());
    assert_eq!(found_archived.id(), archived.id());
    assert!(missing.is_none());
}

async fn rename_machine_gives_the_image_after_the_change(h: &dyn Harness) {
    let built = machine(17, "M17");
    insert(h, &built).await;
    let mut tx = begin(h).await;
    let locked = tx
        .plant()
        .lock_machine_for_no_key_update(built.id())
        .await
        .unwrap()
        .expect("the machine exists");

    let image = tx
        .plant()
        .rename_machine(built.id(), &name("Press 2"))
        .await
        .unwrap();
    super::harness::commit(h, tx, built.id()).await;

    assert_shape(&image, &machine_shape(false));
    assert_eq!(text(&image, "name"), "Press 2");
    assert_eq!(text(&image, "code"), "M17");
    assert_eq!(text(&locked.before, "name"), "Press");
    let reloaded = read(h, built.id()).await.expect("the machine exists");
    assert_eq!(reloaded.name().as_str(), "Press 2");
}

async fn archive_machine_gives_the_image_after_the_change(h: &dyn Harness) {
    let built = machine(18, "M18");
    insert(h, &built).await;
    let mut tx = begin(h).await;
    tx.plant()
        .lock_machine_for_no_key_update(built.id())
        .await
        .unwrap()
        .expect("the machine exists");

    let image = tx.plant().archive_machine(built.id()).await.unwrap();
    super::harness::commit(h, tx, built.id()).await;

    assert_shape(&image, &machine_shape(true));
    assert_eq!(text(&image, "code"), "M18");
    let reloaded = read(h, built.id()).await.expect("the machine exists");
    assert!(!reloaded.is_live());
    assert_eq!(reloaded.units().count(), 1);
}

async fn a_dropped_transaction_writes_nothing(h: &dyn Harness) {
    let built = machine(19, "M19");
    let mut tx = begin(h).await;
    tx.plant().insert_machine(&built).await.unwrap();

    drop(tx);

    assert!(read(h, built.id()).await.is_none());
}

async fn the_platform_of_the_machine_is_kept(h: &dyn Harness) {
    let built = machine(21, "M21");
    insert(h, &built).await;

    let reloaded = read(h, built.id()).await.expect("the machine exists");

    assert_eq!(reloaded.platform(), PlcPlatform::TwinCat3);
}

contract_case!(
    insert_machine_then_lock_shared,
    insert_machine_then_lock_shared_memory,
    insert_machine_then_lock_shared_postgres
);
contract_case!(
    lock_for_no_key_update_gives_state_and_before_image,
    lock_for_no_key_update_gives_state_and_before_image_memory,
    lock_for_no_key_update_gives_state_and_before_image_postgres
);
contract_case!(
    lock_of_a_missing_machine_gives_none,
    lock_of_a_missing_machine_gives_none_memory,
    lock_of_a_missing_machine_gives_none_postgres
);
contract_case!(
    insert_machine_gives_the_images_of_the_machine_and_its_units,
    insert_machine_gives_the_images_of_the_machine_and_its_units_memory,
    insert_machine_gives_the_images_of_the_machine_and_its_units_postgres
);
contract_case!(
    an_archived_unit_survives_the_round_trip,
    an_archived_unit_survives_the_round_trip_memory,
    an_archived_unit_survives_the_round_trip_postgres
);
contract_case!(
    a_live_duplicate_code_gives_the_constraint_of_the_live_index,
    a_live_duplicate_code_gives_the_constraint_of_the_live_index_memory,
    a_live_duplicate_code_gives_the_constraint_of_the_live_index_postgres
);
contract_case!(
    an_archived_machine_frees_its_code,
    an_archived_machine_frees_its_code_memory,
    an_archived_machine_frees_its_code_postgres
);
contract_case!(
    machine_by_code_ignores_archived_machines,
    machine_by_code_ignores_archived_machines_memory,
    machine_by_code_ignores_archived_machines_postgres
);
contract_case!(
    archived_machine_by_code_returns_the_first_by_id,
    archived_machine_by_code_returns_the_first_by_id_memory,
    archived_machine_by_code_returns_the_first_by_id_postgres
);
contract_case!(
    a_live_machine_gives_no_archived_machine_by_code,
    a_live_machine_gives_no_archived_machine_by_code_memory,
    a_live_machine_gives_no_archived_machine_by_code_postgres
);
contract_case!(
    machine_by_id_reads_live_and_archived_machines,
    machine_by_id_reads_live_and_archived_machines_memory,
    machine_by_id_reads_live_and_archived_machines_postgres
);
contract_case!(
    rename_machine_gives_the_image_after_the_change,
    rename_machine_gives_the_image_after_the_change_memory,
    rename_machine_gives_the_image_after_the_change_postgres
);
contract_case!(
    archive_machine_gives_the_image_after_the_change,
    archive_machine_gives_the_image_after_the_change_memory,
    archive_machine_gives_the_image_after_the_change_postgres
);
contract_case!(
    a_dropped_transaction_writes_nothing,
    a_dropped_transaction_writes_nothing_memory,
    a_dropped_transaction_writes_nothing_postgres
);
contract_case!(
    the_platform_of_the_machine_is_kept,
    the_platform_of_the_machine_is_kept_memory,
    the_platform_of_the_machine_is_kept_postgres
);

/// Try one row lock with `NOWAIT` in a transaction of a second session.
///
/// Return the SQLSTATE of the failure, or `None` when the lock was granted.
async fn second_session_lock(session: &mut PgConnection, mode: &str, id: Uuid) -> Option<String> {
    try_row_lock(session, "machine", mode, id).await
}

/// Postgres only: `lock_machine_shared` holds `FOR SHARE` on the row until the transaction ends.
///
/// `FOR SHARE` lets another `FOR SHARE` and `FOR KEY SHARE` pass. It blocks `FOR NO KEY UPDATE`
/// and `FOR UPDATE`.
#[tokio::test]
async fn postgres_only_lock_machine_shared_holds_for_share_on_the_row() {
    let h = PgHarness::create().await;
    let built = machine(40, "M40");
    insert(&h, &built).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let id = *built.id().as_uuid();
    let mut tx = begin(&h).await;

    tx.plant()
        .lock_machine_shared(built.id())
        .await
        .unwrap()
        .expect("the machine exists");

    assert_eq!(
        second_session_lock(&mut session, "UPDATE", id)
            .await
            .as_deref(),
        Some(LOCK_NOT_AVAILABLE)
    );
    assert_eq!(
        second_session_lock(&mut session, "NO KEY UPDATE", id)
            .await
            .as_deref(),
        Some(LOCK_NOT_AVAILABLE)
    );
    assert_eq!(second_session_lock(&mut session, "SHARE", id).await, None);
    assert_eq!(
        second_session_lock(&mut session, "KEY SHARE", id).await,
        None
    );
    drop(tx);
    crate::contract::locks::wait_until_released(&mut session, "machine", id).await;
}

/// Postgres only: `lock_machine_for_no_key_update` holds `FOR NO KEY UPDATE`, not `FOR UPDATE`.
///
/// `FOR NO KEY UPDATE` blocks `FOR SHARE`. It lets `FOR KEY SHARE` pass. `FOR UPDATE` would block
/// `FOR KEY SHARE` too.
#[tokio::test]
async fn postgres_only_lock_for_no_key_update_holds_for_no_key_update_on_the_row() {
    let h = PgHarness::create().await;
    let built = machine(41, "M41");
    insert(&h, &built).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let id = *built.id().as_uuid();
    let mut tx = begin(&h).await;

    tx.plant()
        .lock_machine_for_no_key_update(built.id())
        .await
        .unwrap()
        .expect("the machine exists");

    assert_eq!(
        second_session_lock(&mut session, "SHARE", id)
            .await
            .as_deref(),
        Some(LOCK_NOT_AVAILABLE)
    );
    assert_eq!(
        second_session_lock(&mut session, "NO KEY UPDATE", id)
            .await
            .as_deref(),
        Some(LOCK_NOT_AVAILABLE)
    );
    assert_eq!(
        second_session_lock(&mut session, "KEY SHARE", id).await,
        None
    );
    drop(tx);
    crate::contract::locks::wait_until_released(&mut session, "machine", id).await;
}

/// Postgres only: a lock of a missing machine takes no lock and does not fail.
#[tokio::test]
async fn postgres_only_a_missing_machine_takes_no_lock() {
    let h = PgHarness::create().await;
    let mut tx = begin(&h).await;

    let locked = tx
        .plant()
        .lock_machine_shared(machine_id(98))
        .await
        .unwrap();

    assert!(locked.is_none());
}
