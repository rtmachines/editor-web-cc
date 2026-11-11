use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::plant::{
    CommMode, DataBlockNumber, MachineId, PasswordState, PlcAddress, PlcConnection, PlcHost,
    PlcLogin, PlcPlatform, Rack, Slot, TcpPort,
};
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{PLC_CONNECTION_MACHINE_PLATFORM, PLC_CONNECTION_MACHINE_UNIQUE, StorageError};
use crate::memory::fixtures::machine_on;
use crate::memory::{JournalEntry, Lock, LockMode, MemoryClock, MemoryStore, MemoryTransaction};
use crate::ports::{Isolation, NonEmpty, PasswordWrite, SealedSecret, Transaction, UnitOfWork};
use recipe_domain::error_codes::ErrorCode;

const NOW_SECONDS: i64 = 1_000;

fn store() -> MemoryStore {
    MemoryStore::with_clock(MemoryClock::new(
        OffsetDateTime::from_unix_timestamp(NOW_SECONDS).unwrap(),
    ))
}

fn mid(n: u128) -> MachineId {
    MachineId::from(Uuid::from_u128(n))
}

fn begin(store: &MemoryStore, isolation: Isolation) -> MemoryTransaction {
    store.begin_memory(isolation).unwrap()
}

/// A store with the machine 1 on `S7_1500`, the machine 2 on `TwinCat3`, and the machine 3 on `CodesysIec`.
fn seeded() -> MemoryStore {
    let store = store();
    store.seed_machine(&machine_on(PlcPlatform::S7_1500, 1, "M1"));
    store.seed_machine(&machine_on(PlcPlatform::TwinCat3, 2, "M2"));
    store.seed_machine(&machine_on(PlcPlatform::CodesysIec, 3, "M3"));
    store
}

fn s7() -> PlcConnection {
    PlcConnection::S7 {
        host: PlcHost::parse("10.0.0.5").unwrap(),
        rack: Rack::parse("0").unwrap(),
        slot: Slot::parse("2").unwrap(),
        db_number: DataBlockNumber::parse("100").unwrap(),
    }
}

fn iec(mode: CommMode, login: Option<&str>) -> PlcConnection {
    PlcConnection::Iec {
        host: PlcHost::parse("plc.local").unwrap(),
        port: TcpPort::parse("4840").unwrap(),
        address: PlcAddress::parse("opc.tcp://plc.local:4840").unwrap(),
        mode,
        login: login.map(|text| PlcLogin::parse(text).unwrap()),
    }
}

fn sealed(byte: u8) -> SealedSecret {
    SealedSecret::from_bytes(vec![1, byte, byte])
}

fn now_text() -> String {
    OffsetDateTime::from_unix_timestamp(NOW_SECONDS)
        .unwrap()
        .to_string()
}

fn audit_entry() -> NonEmpty<AuditEntry> {
    NonEmpty::new(AuditEntry::row(
        RowAction::MachineCreated,
        AuditTarget::machine(mid(1)),
        RowChange::Create {
            after: json!({ "n": 1 }),
        },
        AuditContext {
            actor: None,
            request_id: RequestId::from(Uuid::from_u128(1)),
            client_ip: None,
            session_ref: None,
        },
    ))
}

fn locks(store: &MemoryStore) -> Vec<JournalEntry> {
    store
        .journal()
        .into_iter()
        .filter(|line| matches!(line, JournalEntry::Lock(_)))
        .collect()
}

#[tokio::test]
async fn a_machine_without_a_record_gives_none_and_takes_no_lock() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    assert!(port.connection_by_machine(mid(1)).await.unwrap().is_none());
    assert!(
        port.lock_connection_for_update(mid(1))
            .await
            .unwrap()
            .is_none()
    );
    assert!(port.lock_connection_shared(mid(1)).await.unwrap().is_none());
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn an_insert_starts_at_row_version_1_and_returns_the_image_without_the_password() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let image = tx
        .plc_connections()
        .insert_connection(
            mid(2),
            &iec(CommMode::OpcUa, Some("operator")),
            Some(&sealed(7)),
        )
        .await
        .unwrap();
    let value = image.into_value();
    let object = value.as_object().unwrap();
    assert_eq!(object.get("row_version"), Some(&json!(1)));
    assert_eq!(object.get("machine_id"), Some(&json!(mid(2).to_string())));
    assert_eq!(object.get("plc_platform"), Some(&json!("TwinCat3")));
    assert_eq!(object.get("host"), Some(&json!("plc.local")));
    assert_eq!(object.get("port"), Some(&json!(4840)));
    assert_eq!(object.get("comm_mode"), Some(&json!("OpcUa")));
    assert_eq!(object.get("login"), Some(&json!("operator")));
    assert_eq!(object.get("rack"), Some(&Value::Null));
    assert_eq!(object.get("created_at"), Some(&json!(now_text())));
    assert!(!object.contains_key("password_sealed"));
}

#[tokio::test]
async fn an_s7_insert_stores_the_s7_columns_and_no_iec_column() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let image = tx
        .plc_connections()
        .insert_connection(mid(1), &s7(), None)
        .await
        .unwrap()
        .into_value();
    let object = image.as_object().unwrap();
    assert_eq!(object.get("rack"), Some(&json!(0)));
    assert_eq!(object.get("slot"), Some(&json!(2)));
    assert_eq!(object.get("db_number"), Some(&json!(100)));
    assert_eq!(object.get("port"), Some(&Value::Null));
    assert_eq!(object.get("address"), Some(&Value::Null));
    assert_eq!(object.get("comm_mode"), Some(&Value::Null));
    assert_eq!(object.get("login"), Some(&Value::Null));
}

#[tokio::test]
async fn a_read_gives_the_record_and_the_password_state_and_never_the_sealed_bytes() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    port.insert_connection(mid(2), &iec(CommMode::OpcUa, Some("op")), Some(&sealed(7)))
        .await
        .unwrap();
    let stored = port.connection_by_machine(mid(2)).await.unwrap().unwrap();
    assert_eq!(stored.row_version, 1);
    assert_eq!(stored.password, PasswordState::Stored);
    assert!(matches!(
        stored.connection,
        PlcConnection::Iec { mode: CommMode::OpcUa, ref login, .. }
            if login.as_ref().map(PlcLogin::as_str) == Some("op")
    ));
    assert!(!format!("{stored:?}").contains("password_sealed"));
}

#[tokio::test]
async fn a_record_without_a_password_reads_as_absent() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    port.insert_connection(mid(1), &s7(), None).await.unwrap();
    let stored = port.connection_by_machine(mid(1)).await.unwrap().unwrap();
    assert_eq!(stored.password, PasswordState::Absent);
    assert!(matches!(stored.connection, PlcConnection::S7 { .. }));
}

#[tokio::test]
async fn the_update_lock_returns_the_record_and_the_image_before_the_change() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    port.insert_connection(mid(1), &s7(), None).await.unwrap();
    let locked = port
        .lock_connection_for_update(mid(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(locked.state.row_version, 1);
    assert_eq!(locked.before.as_value().get("row_version"), Some(&json!(1)));
    assert_eq!(
        locks(&store),
        vec![JournalEntry::Lock(Lock {
            table: "plc_connection",
            mode: LockMode::ForUpdate,
            key: mid(1).to_string(),
        })]
    );
}

#[tokio::test]
async fn the_shared_lock_returns_the_sealed_password_and_journals_for_share() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    port.insert_connection(mid(2), &iec(CommMode::OpcUa, Some("op")), Some(&sealed(9)))
        .await
        .unwrap();
    let shared = port.lock_connection_shared(mid(2)).await.unwrap().unwrap();
    assert_eq!(shared.sealed, Some(sealed(9)));
    assert_eq!(shared.stored.password, PasswordState::Stored);
    assert_eq!(
        locks(&store),
        vec![JournalEntry::Lock(Lock {
            table: "plc_connection",
            mode: LockMode::ForShare,
            key: mid(2).to_string(),
        })]
    );
}

#[tokio::test]
async fn an_update_adds_1_to_row_version_and_keeps_the_sealed_password_on_keep() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    port.insert_connection(mid(2), &iec(CommMode::OpcUa, Some("op")), Some(&sealed(7)))
        .await
        .unwrap();
    let image = port
        .update_connection(
            mid(2),
            &iec(CommMode::OpcUa, Some("other")),
            &PasswordWrite::Keep,
        )
        .await
        .unwrap()
        .into_value();
    assert_eq!(image.get("row_version"), Some(&json!(2)));
    assert_eq!(image.get("login"), Some(&json!("other")));
    let shared = port.lock_connection_shared(mid(2)).await.unwrap().unwrap();
    assert_eq!(shared.sealed, Some(sealed(7)));
}

#[tokio::test]
async fn an_update_with_set_replaces_the_sealed_password() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    port.insert_connection(mid(2), &iec(CommMode::OpcUa, Some("op")), Some(&sealed(7)))
        .await
        .unwrap();
    port.update_connection(
        mid(2),
        &iec(CommMode::OpcUa, Some("op")),
        &PasswordWrite::Set(sealed(8)),
    )
    .await
    .unwrap();
    let shared = port.lock_connection_shared(mid(2)).await.unwrap().unwrap();
    assert_eq!(shared.sealed, Some(sealed(8)));
}

#[tokio::test]
async fn an_update_with_clear_removes_the_sealed_password() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    port.insert_connection(mid(2), &iec(CommMode::Ads, Some("op")), Some(&sealed(7)))
        .await
        .unwrap();
    port.update_connection(mid(2), &iec(CommMode::Ads, None), &PasswordWrite::Clear)
        .await
        .unwrap();
    let shared = port.lock_connection_shared(mid(2)).await.unwrap().unwrap();
    assert_eq!(shared.sealed, None);
    assert_eq!(shared.stored.password, PasswordState::Absent);
    assert_eq!(shared.stored.row_version, 2);
}

#[tokio::test]
async fn an_update_without_a_row_is_a_defect_of_the_caller() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let error = tx
        .plc_connections()
        .update_connection(mid(1), &s7(), &PasswordWrite::Keep)
        .await
        .unwrap_err();
    assert!(matches!(error, StorageError::Other { .. }));
}

#[tokio::test]
async fn a_second_record_for_a_machine_gives_a_duplicate() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    port.insert_connection(mid(1), &s7(), None).await.unwrap();
    let error = port
        .insert_connection(mid(1), &s7(), None)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        StorageError::Duplicate { ref constraint } if constraint == PLC_CONNECTION_MACHINE_UNIQUE
    ));
}

#[tokio::test]
async fn a_record_for_a_missing_machine_gives_a_missing_reference() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let error = tx
        .plc_connections()
        .insert_connection(mid(99), &s7(), None)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        StorageError::MissingReference { ref constraint }
            if constraint == PLC_CONNECTION_MACHINE_PLATFORM
    ));
}

#[tokio::test]
async fn a_shape_of_another_platform_violates_the_check() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    let on_s7 = port
        .insert_connection(mid(1), &iec(CommMode::ModbusTcp, None), None)
        .await
        .unwrap_err();
    let on_iec = port
        .insert_connection(mid(2), &s7(), None)
        .await
        .unwrap_err();
    assert_eq!(on_s7.code(), ErrorCode::DB_003);
    assert_eq!(on_iec.code(), ErrorCode::DB_003);
}

#[tokio::test]
async fn the_credentials_of_the_mode_are_checked_like_the_table() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    // A login without a ciphertext.
    let login_only = port
        .insert_connection(mid(2), &iec(CommMode::Ads, Some("op")), None)
        .await
        .unwrap_err();
    // OpcUa without a login.
    let opcua = port
        .insert_connection(mid(2), &iec(CommMode::OpcUa, None), None)
        .await
        .unwrap_err();
    // ModbusTcp with a login.
    let modbus = port
        .insert_connection(
            mid(2),
            &iec(CommMode::ModbusTcp, Some("op")),
            Some(&sealed(1)),
        )
        .await
        .unwrap_err();
    // An S7 row with a ciphertext.
    let s7_password = port
        .insert_connection(mid(1), &s7(), Some(&sealed(1)))
        .await
        .unwrap_err();
    for error in [login_only, opcua, modbus, s7_password] {
        assert_eq!(error.code(), ErrorCode::DB_003);
    }
    assert!(port.connection_by_machine(mid(2)).await.unwrap().is_none());
}

#[tokio::test]
async fn an_update_that_leaves_a_login_without_a_ciphertext_violates_the_check() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    port.insert_connection(mid(2), &iec(CommMode::Ads, None), None)
        .await
        .unwrap();
    let error = port
        .update_connection(
            mid(2),
            &iec(CommMode::Ads, Some("op")),
            &PasswordWrite::Keep,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_003);
}

#[tokio::test]
async fn a_drop_without_finish_discards_the_record() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    tx.plc_connections()
        .insert_connection(mid(1), &s7(), None)
        .await
        .unwrap();
    drop(tx);
    let mut next = begin(&store, Isolation::ReadCommitted);
    assert!(
        next.plc_connections()
            .connection_by_machine(mid(1))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn a_commit_keeps_the_record() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    tx.plc_connections()
        .insert_connection(mid(1), &s7(), None)
        .await
        .unwrap();
    let unit_of_work: &dyn UnitOfWork = &store;
    unit_of_work
        .commit(Box::new(tx), audit_entry())
        .await
        .unwrap();
    let mut next = begin(&store, Isolation::ReadCommitted);
    let stored = next
        .plc_connections()
        .connection_by_machine(mid(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.row_version, 1);
}

#[tokio::test]
async fn every_read_of_one_record_gives_the_id_of_its_row_image() {
    let store = seeded();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let port = tx.plc_connections();
    let image = port.insert_connection(mid(1), &s7(), None).await.unwrap();
    let image_id = image.as_value().get("id").cloned();
    let read = port.connection_by_machine(mid(1)).await.unwrap().unwrap();
    let locked = port
        .lock_connection_for_update(mid(1))
        .await
        .unwrap()
        .unwrap();
    let shared = port.lock_connection_shared(mid(1)).await.unwrap().unwrap();
    let expected = image_id.and_then(|value| value.as_str().map(str::to_owned));
    assert!(expected.is_some());
    assert_eq!(Some(read.id.to_string()), expected);
    assert_eq!(Some(locked.state.id.to_string()), expected);
    assert_eq!(Some(shared.stored.id.to_string()), expected);
}
