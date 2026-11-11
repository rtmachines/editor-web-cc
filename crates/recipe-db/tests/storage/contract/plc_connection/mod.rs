//! Contract cases of the port `PlcConnections` (task 4.7, ADR-0154, `data-model.md` sections 3.18 and 6).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The cases in
//! `postgres_locks` check the row locks and run on `Database` only.

mod fixtures;
mod postgres_locks;

use recipe_app::error::{
    PLC_CONNECTION_MACHINE_PLATFORM, PLC_CONNECTION_MACHINE_UNIQUE, StorageError,
};
use recipe_app::ports::{PasswordWrite, SealedSecret};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{MachineId, PasswordState, PlcConnection, PlcPlatform};

use super::harness::{Harness, begin, commit};
use super::images::{Kind, assert_shape, changed_keys, field, text};
use super::seed::machine_id;
use fixtures::{
    connection_text, iec_connection, insert_connection, s7_connection, save_machine, sealed,
};

/// The shape of an S7 `plc_connection` image. It never holds `password_sealed` (ADR-0154 (h)).
fn s7_shape() -> Vec<(&'static str, Kind)> {
    vec![
        ("id", Kind::String),
        ("machine_id", Kind::String),
        ("plc_platform", Kind::String),
        ("host", Kind::String),
        ("rack", Kind::Number),
        ("slot", Kind::Number),
        ("db_number", Kind::Number),
        ("port", Kind::Null),
        ("address", Kind::Null),
        ("comm_mode", Kind::Null),
        ("login", Kind::Null),
        ("row_version", Kind::Number),
        ("created_at", Kind::String),
        ("updated_at", Kind::String),
    ]
}

/// The shape of an IEC `plc_connection` image.
fn iec_shape(has_login: bool) -> Vec<(&'static str, Kind)> {
    vec![
        ("id", Kind::String),
        ("machine_id", Kind::String),
        ("plc_platform", Kind::String),
        ("host", Kind::String),
        ("rack", Kind::Null),
        ("slot", Kind::Null),
        ("db_number", Kind::Null),
        ("port", Kind::Number),
        ("address", Kind::String),
        ("comm_mode", Kind::String),
        ("login", if has_login { Kind::String } else { Kind::Null }),
        ("row_version", Kind::Number),
        ("created_at", Kind::String),
        ("updated_at", Kind::String),
    ]
}

/// An OPC UA connection with a login. The mode needs a sealed password.
fn opc_ua_connection() -> PlcConnection {
    iec_connection(
        "plc-line1.plant.local",
        "4840",
        "urn:plc:1",
        "OpcUa",
        Some("operator"),
    )
}

/// An ADS connection. A login is optional in this mode.
fn ads_connection(login: Option<&str>) -> PlcConnection {
    iec_connection(
        "plc-line1.plant.local",
        "48898",
        "5.12.34.56.1.1",
        "Ads",
        login,
    )
}

fn is_check_error(error: &StorageError) -> bool {
    matches!(error, StorageError::Constraint { code, .. } if *code == ErrorCode::DB_003)
}

async fn a_machine_without_a_record_gives_none_on_every_method(h: &dyn Harness) {
    let machine = save_machine(h, 1, "M1", PlcPlatform::S7_1500).await;
    let mut tx = begin(h).await;

    let read = tx.plc_connections().connection_by_machine(machine).await;
    let for_update = tx
        .plc_connections()
        .lock_connection_for_update(machine)
        .await;
    let shared = tx.plc_connections().lock_connection_shared(machine).await;

    assert!(read.unwrap().is_none());
    assert!(for_update.unwrap().is_none());
    assert!(shared.unwrap().is_none());
}

async fn insert_of_an_s7_record_starts_at_row_version_1_and_reads_back(h: &dyn Harness) {
    let machine = save_machine(h, 2, "M2", PlcPlatform::S7_1500).await;
    let built = s7_connection("192.168.1.20", "0", "2", "10");
    let mut tx = begin(h).await;

    let image = tx
        .plc_connections()
        .insert_connection(machine, &built, None)
        .await
        .unwrap();
    commit(h, tx, machine).await;

    assert_shape(&image, &s7_shape());
    assert_eq!(field(&image, "row_version"), 1);
    assert_eq!(text(&image, "machine_id"), machine.to_string());
    assert_eq!(text(&image, "plc_platform"), "S7_1500");
    assert_eq!(text(&image, "host"), "192.168.1.20");
    assert_eq!(field(&image, "rack"), 0);
    assert_eq!(field(&image, "slot"), 2);
    assert_eq!(field(&image, "db_number"), 10);
    let mut tx = begin(h).await;
    let stored = tx
        .plc_connections()
        .connection_by_machine(machine)
        .await
        .unwrap()
        .expect("the record exists");
    assert_eq!(connection_text(&stored.connection), connection_text(&built));
    assert_eq!(stored.row_version, 1);
    assert_eq!(stored.password, PasswordState::Absent);
    assert_eq!(stored.id.to_string(), text(&image, "id"));
}

async fn insert_of_an_iec_record_with_a_password_reads_back_the_password_state(h: &dyn Harness) {
    let machine = save_machine(h, 3, "M3", PlcPlatform::TwinCat3).await;
    let built = opc_ua_connection();

    let image = insert_connection(h, machine, &built, Some(&sealed(&[1, 2, 3]))).await;

    assert_shape(&image, &iec_shape(true));
    assert_eq!(field(&image, "port"), 4840);
    assert_eq!(text(&image, "comm_mode"), "OpcUa");
    assert_eq!(text(&image, "login"), "operator");
    let mut tx = begin(h).await;
    let stored = tx
        .plc_connections()
        .connection_by_machine(machine)
        .await
        .unwrap()
        .expect("the record exists");
    assert_eq!(connection_text(&stored.connection), connection_text(&built));
    assert_eq!(stored.password, PasswordState::Stored);
}

async fn a_second_record_of_one_machine_gives_the_unique_constraint(h: &dyn Harness) {
    let machine = save_machine(h, 4, "M4", PlcPlatform::S7_1500).await;
    let built = s7_connection("192.168.1.20", "0", "2", "10");
    insert_connection(h, machine, &built, None).await;
    let mut tx = begin(h).await;

    let error = tx
        .plc_connections()
        .insert_connection(machine, &built, None)
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint }
            if constraint == PLC_CONNECTION_MACHINE_UNIQUE),
        "{error:?}"
    );
}

async fn an_insert_for_a_missing_machine_gives_the_platform_key(h: &dyn Harness) {
    let built = s7_connection("192.168.1.20", "0", "2", "10");
    let mut tx = begin(h).await;

    let error = tx
        .plc_connections()
        .insert_connection(machine_id(99), &built, None)
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::MissingReference { constraint }
            if constraint == PLC_CONNECTION_MACHINE_PLATFORM),
        "{error:?}"
    );
}

async fn an_insert_of_the_wrong_shape_gives_the_check_error(h: &dyn Harness) {
    let machine = save_machine(h, 5, "M5", PlcPlatform::TwinCat3).await;
    let built = s7_connection("192.168.1.20", "0", "2", "10");
    let mut tx = begin(h).await;

    let error = tx
        .plc_connections()
        .insert_connection(machine, &built, None)
        .await
        .unwrap_err();

    assert!(is_check_error(&error), "{error:?}");
}

async fn an_insert_without_the_password_of_the_mode_gives_the_check_error(h: &dyn Harness) {
    let machine = save_machine(h, 6, "M6", PlcPlatform::TwinCat3).await;
    let mut tx = begin(h).await;

    let error = tx
        .plc_connections()
        .insert_connection(machine, &opc_ua_connection(), None)
        .await
        .unwrap_err();

    assert!(is_check_error(&error), "{error:?}");
}

async fn lock_for_update_gives_the_state_and_the_image_before_the_change(h: &dyn Harness) {
    let machine = save_machine(h, 7, "M7", PlcPlatform::TwinCat3).await;
    let built = opc_ua_connection();
    let inserted = insert_connection(h, machine, &built, Some(&sealed(&[1, 2, 3]))).await;
    let mut tx = begin(h).await;

    let locked = tx
        .plc_connections()
        .lock_connection_for_update(machine)
        .await
        .unwrap()
        .expect("the record exists");

    assert_shape(&locked.before, &iec_shape(true));
    assert!(changed_keys(&locked.before, &inserted).is_empty());
    assert_eq!(
        connection_text(&locked.state.connection),
        connection_text(&built)
    );
    assert_eq!(locked.state.row_version, 1);
    assert_eq!(locked.state.password, PasswordState::Stored);
    assert_eq!(locked.state.id.to_string(), text(&locked.before, "id"));
    assert_eq!(locked.state.id.to_string(), text(&inserted, "id"));
}

async fn an_update_adds_1_to_the_row_version_and_gives_the_image_after_the_change(h: &dyn Harness) {
    let machine = save_machine(h, 8, "M8", PlcPlatform::S7_1500).await;
    insert_connection(
        h,
        machine,
        &s7_connection("192.168.1.20", "0", "2", "10"),
        None,
    )
    .await;
    let second = s7_connection("192.168.1.21", "1", "3", "11");
    let mut tx = begin(h).await;
    let locked = tx
        .plc_connections()
        .lock_connection_for_update(machine)
        .await
        .unwrap()
        .expect("the record exists");

    let image = tx
        .plc_connections()
        .update_connection(machine, &second, &PasswordWrite::Keep)
        .await
        .unwrap();
    commit(h, tx, machine).await;

    assert_shape(&image, &s7_shape());
    assert_eq!(field(&image, "row_version"), 2);
    assert_eq!(
        changed_keys(&locked.before, &image),
        vec!["db_number", "host", "rack", "row_version", "slot"]
    );
    let mut tx = begin(h).await;
    let stored = tx
        .plc_connections()
        .connection_by_machine(machine)
        .await
        .unwrap()
        .expect("the record exists");
    assert_eq!(
        connection_text(&stored.connection),
        connection_text(&second)
    );
    assert_eq!(stored.row_version, 2);
    assert_eq!(stored.id.to_string(), text(&image, "id"));
    assert_eq!(stored.id, locked.state.id);
}

/// Update the record in its own committed transaction. Return the sealed bytes that
/// `lock_connection_shared` reads afterwards.
async fn update_and_read_sealed(
    h: &dyn Harness,
    machine: MachineId,
    connection: &PlcConnection,
    password: &PasswordWrite,
) -> Option<Vec<u8>> {
    let mut tx = begin(h).await;
    tx.plc_connections()
        .lock_connection_for_update(machine)
        .await
        .unwrap()
        .expect("the record exists");
    tx.plc_connections()
        .update_connection(machine, connection, password)
        .await
        .unwrap();
    commit(h, tx, machine).await;
    let mut tx = begin(h).await;
    let shared = tx
        .plc_connections()
        .lock_connection_shared(machine)
        .await
        .unwrap()
        .expect("the record exists");
    shared.sealed.map(|secret| secret.as_bytes().to_vec())
}

async fn keep_set_and_clear_decide_the_stored_password(h: &dyn Harness) {
    let machine = save_machine(h, 9, "M9", PlcPlatform::TwinCat3).await;
    let ads = ads_connection(Some("svc"));
    insert_connection(h, machine, &ads, Some(&sealed(&[1, 2, 3]))).await;

    let kept = update_and_read_sealed(h, machine, &ads, &PasswordWrite::Keep).await;
    let replaced =
        update_and_read_sealed(h, machine, &ads, &PasswordWrite::Set(sealed(&[9, 9]))).await;
    let cleared =
        update_and_read_sealed(h, machine, &ads_connection(None), &PasswordWrite::Clear).await;

    assert_eq!(kept, Some(vec![1, 2, 3]));
    assert_eq!(replaced, Some(vec![9, 9]));
    assert_eq!(cleared, None);
}

async fn an_update_without_the_password_of_the_mode_gives_the_check_error(h: &dyn Harness) {
    let machine = save_machine(h, 10, "M10", PlcPlatform::TwinCat3).await;
    let built = opc_ua_connection();
    insert_connection(h, machine, &built, Some(&sealed(&[1]))).await;
    let mut tx = begin(h).await;
    tx.plc_connections()
        .lock_connection_for_update(machine)
        .await
        .unwrap();

    let error = tx
        .plc_connections()
        .update_connection(machine, &built, &PasswordWrite::Clear)
        .await
        .unwrap_err();

    assert!(is_check_error(&error), "{error:?}");
}

async fn an_update_of_a_missing_record_gives_a_defect_error(h: &dyn Harness) {
    let machine = save_machine(h, 11, "M11", PlcPlatform::S7_1500).await;
    let built = s7_connection("192.168.1.20", "0", "2", "10");
    let mut tx = begin(h).await;

    let error = tx
        .plc_connections()
        .update_connection(machine, &built, &PasswordWrite::Keep)
        .await
        .unwrap_err();

    assert!(matches!(error, StorageError::Other { .. }), "{error:?}");
}

async fn lock_shared_reads_the_sealed_password_of_an_iec_record(h: &dyn Harness) {
    let machine = save_machine(h, 12, "M12", PlcPlatform::TwinCat3).await;
    let built = opc_ua_connection();
    let inserted = insert_connection(h, machine, &built, Some(&sealed(&[7, 8, 9]))).await;
    let mut tx = begin(h).await;

    let shared = tx
        .plc_connections()
        .lock_connection_shared(machine)
        .await
        .unwrap()
        .expect("the record exists");

    assert_eq!(shared.sealed, Some(SealedSecret::from_bytes(vec![7, 8, 9])));
    assert_eq!(shared.stored.password, PasswordState::Stored);
    assert_eq!(shared.stored.row_version, 1);
    assert_eq!(shared.stored.id.to_string(), text(&inserted, "id"));
    assert_eq!(
        connection_text(&shared.stored.connection),
        connection_text(&built)
    );
}

async fn lock_shared_of_an_s7_record_gives_no_sealed_password(h: &dyn Harness) {
    let machine = save_machine(h, 13, "M13", PlcPlatform::S7_300).await;
    insert_connection(
        h,
        machine,
        &s7_connection("192.168.1.20", "0", "2", "10"),
        None,
    )
    .await;
    let mut tx = begin(h).await;

    let shared = tx
        .plc_connections()
        .lock_connection_shared(machine)
        .await
        .unwrap()
        .expect("the record exists");

    assert_eq!(shared.sealed, None);
    assert_eq!(shared.stored.password, PasswordState::Absent);
}

async fn a_dropped_transaction_writes_no_record(h: &dyn Harness) {
    let machine = save_machine(h, 14, "M14", PlcPlatform::S7_1500).await;
    let built = s7_connection("192.168.1.20", "0", "2", "10");
    let mut tx = begin(h).await;
    tx.plc_connections()
        .insert_connection(machine, &built, None)
        .await
        .unwrap();

    drop(tx);

    let mut tx = begin(h).await;
    assert!(
        tx.plc_connections()
            .connection_by_machine(machine)
            .await
            .unwrap()
            .is_none()
    );
}

contract_case!(
    a_machine_without_a_record_gives_none_on_every_method,
    a_machine_without_a_record_gives_none_on_every_method_memory,
    a_machine_without_a_record_gives_none_on_every_method_postgres
);
contract_case!(
    insert_of_an_s7_record_starts_at_row_version_1_and_reads_back,
    insert_of_an_s7_record_starts_at_row_version_1_and_reads_back_memory,
    insert_of_an_s7_record_starts_at_row_version_1_and_reads_back_postgres
);
contract_case!(
    insert_of_an_iec_record_with_a_password_reads_back_the_password_state,
    insert_of_an_iec_record_with_a_password_reads_back_the_password_state_memory,
    insert_of_an_iec_record_with_a_password_reads_back_the_password_state_postgres
);
contract_case!(
    a_second_record_of_one_machine_gives_the_unique_constraint,
    a_second_record_of_one_machine_gives_the_unique_constraint_memory,
    a_second_record_of_one_machine_gives_the_unique_constraint_postgres
);
contract_case!(
    an_insert_for_a_missing_machine_gives_the_platform_key,
    an_insert_for_a_missing_machine_gives_the_platform_key_memory,
    an_insert_for_a_missing_machine_gives_the_platform_key_postgres
);
contract_case!(
    an_insert_of_the_wrong_shape_gives_the_check_error,
    an_insert_of_the_wrong_shape_gives_the_check_error_memory,
    an_insert_of_the_wrong_shape_gives_the_check_error_postgres
);
contract_case!(
    an_insert_without_the_password_of_the_mode_gives_the_check_error,
    an_insert_without_the_password_of_the_mode_gives_the_check_error_memory,
    an_insert_without_the_password_of_the_mode_gives_the_check_error_postgres
);
contract_case!(
    lock_for_update_gives_the_state_and_the_image_before_the_change,
    lock_for_update_gives_the_state_and_the_image_before_the_change_memory,
    lock_for_update_gives_the_state_and_the_image_before_the_change_postgres
);
contract_case!(
    an_update_adds_1_to_the_row_version_and_gives_the_image_after_the_change,
    an_update_adds_1_to_the_row_version_and_gives_the_image_after_the_change_memory,
    an_update_adds_1_to_the_row_version_and_gives_the_image_after_the_change_postgres
);
contract_case!(
    keep_set_and_clear_decide_the_stored_password,
    keep_set_and_clear_decide_the_stored_password_memory,
    keep_set_and_clear_decide_the_stored_password_postgres
);
contract_case!(
    an_update_without_the_password_of_the_mode_gives_the_check_error,
    an_update_without_the_password_of_the_mode_gives_the_check_error_memory,
    an_update_without_the_password_of_the_mode_gives_the_check_error_postgres
);
contract_case!(
    an_update_of_a_missing_record_gives_a_defect_error,
    an_update_of_a_missing_record_gives_a_defect_error_memory,
    an_update_of_a_missing_record_gives_a_defect_error_postgres
);
contract_case!(
    lock_shared_reads_the_sealed_password_of_an_iec_record,
    lock_shared_reads_the_sealed_password_of_an_iec_record_memory,
    lock_shared_reads_the_sealed_password_of_an_iec_record_postgres
);
contract_case!(
    lock_shared_of_an_s7_record_gives_no_sealed_password,
    lock_shared_of_an_s7_record_gives_no_sealed_password_memory,
    lock_shared_of_an_s7_record_gives_no_sealed_password_postgres
);
contract_case!(
    a_dropped_transaction_writes_no_record,
    a_dropped_transaction_writes_no_record_memory,
    a_dropped_transaction_writes_no_record_postgres
);
