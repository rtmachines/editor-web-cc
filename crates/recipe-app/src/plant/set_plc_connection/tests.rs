use std::sync::Arc;

use parking_lot::Mutex;
use recipe_domain::audit::{AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    CommMode, ConnectionFields, DataBlockNumber, MachineCode, MachineId, PasswordState, PlcAddress,
    PlcConnection, PlcHost, PlcLogin, PlcPassword, PlcPlatform, Rack, Slot, TcpPort,
};
use serde_json::json;
use uuid::Uuid;

use super::{SetPlcConnection, SetPlcConnectionInput};
use crate::auth::Operation;
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing, TaskStage};
use crate::memory::fixtures::{Fixture, actor, lock_line, meta, mid, timestamp};
use crate::memory::{CipherCall, Fault, JournalEntry, LockMode, MemoryCredentialCipher};
use crate::ports::{CipherFault, CredentialCipher as _, Isolation, SealedSecret, Transaction};

mod audit;
mod passwords;
mod saves;

/// The use case over a cipher that the test does not watch.
fn use_case(context: AppContext) -> SetPlcConnection {
    SetPlcConnection::new(context, Arc::new(MemoryCredentialCipher::default()))
}

/// The use case over the cipher that the test watches.
fn use_case_with(context: AppContext, cipher: &MemoryCredentialCipher) -> SetPlcConnection {
    SetPlcConnection::new(context, Arc::new(cipher.clone()))
}

fn input(code: &str) -> SetPlcConnectionInput {
    SetPlcConnectionInput {
        code: MachineCode::parse(code).unwrap(),
        expected_row_version: 0,
        fields: ConnectionFields::default(),
    }
}

fn save(code: &str, expected_row_version: i64, fields: ConnectionFields) -> SetPlcConnectionInput {
    SetPlcConnectionInput {
        code: MachineCode::parse(code).unwrap(),
        expected_row_version,
        fields,
    }
}

fn s7_fields() -> ConnectionFields {
    ConnectionFields {
        host: Some(PlcHost::parse("10.0.0.5").unwrap()),
        rack: Some(Rack::parse("0").unwrap()),
        slot: Some(Slot::parse("2").unwrap()),
        ..ConnectionFields::default()
    }
}

fn modbus_fields() -> ConnectionFields {
    ConnectionFields {
        host: Some(PlcHost::parse("plc.local").unwrap()),
        port: Some(TcpPort::parse("502").unwrap()),
        address: Some(PlcAddress::parse("plc.local:502").unwrap()),
        mode: Some(CommMode::ModbusTcp),
        ..ConnectionFields::default()
    }
}

fn s7_record() -> PlcConnection {
    PlcConnection::S7 {
        host: PlcHost::parse("10.0.0.9").unwrap(),
        rack: Rack::parse("0").unwrap(),
        slot: Slot::parse("1").unwrap(),
    }
}

/// Give machine `n` the data block number `text` and leave its record as it is.
fn set_machine_number(fixture: &Fixture, n: u128, text: &str) {
    fixture.store.update_state(|state, _now| {
        let row = state.machines.get_mut(&mid(n)).unwrap();
        row.db_number = Some(DataBlockNumber::parse(text).unwrap());
    });
}

/// Return the `db_number` column of the record of machine 1 as the audit image shows it.
async fn stored_copy(fixture: &Fixture) -> serde_json::Value {
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let locked = tx
        .plc_connections()
        .lock_connection_for_update(mid(1))
        .await
        .unwrap()
        .unwrap();
    locked.before.as_value().get("db_number").unwrap().clone()
}

const NEW_PASSWORD: &str = "new-secret-1";

/// The fields of an `OpcUa` request on a `TwinCat3` machine. `OpcUa` needs a login and a password.
fn opcua_fields(password: Option<&str>) -> ConnectionFields {
    ConnectionFields {
        host: Some(PlcHost::parse("plc.local").unwrap()),
        port: Some(TcpPort::parse("4840").unwrap()),
        address: Some(PlcAddress::parse("plc.local:4840").unwrap()),
        mode: Some(CommMode::OpcUa),
        login: Some(PlcLogin::parse("adapter").unwrap()),
        password: password.map(|text| PlcPassword::parse(text).unwrap()),
        ..ConnectionFields::default()
    }
}

/// An `OpcUa` record that exists before the command runs.
fn opcua_record() -> PlcConnection {
    PlcConnection::Iec {
        host: PlcHost::parse("plc.local").unwrap(),
        port: TcpPort::parse("4840").unwrap(),
        address: PlcAddress::parse("plc.local:4840").unwrap(),
        mode: CommMode::OpcUa,
        login: Some(PlcLogin::parse("adapter").unwrap()),
    }
}

/// Seal `text` for `machine` with a cipher that the test does not watch.
fn sealed_before(machine: MachineId, text: &str) -> SealedSecret {
    MemoryCredentialCipher::default()
        .seal(machine, &PlcPassword::parse(text).unwrap())
        .unwrap()
}

/// Return the sealed password and the password state of the stored record.
async fn stored_password(
    fixture: &Fixture,
    machine: MachineId,
) -> (Option<SealedSecret>, PasswordState) {
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let shared = tx
        .plc_connections()
        .lock_connection_shared(machine)
        .await
        .unwrap()
        .unwrap();
    (shared.sealed, shared.stored.password)
}

/// The keys that a `PlcConnectionSet` image may hold. `password_sealed` is not among them.
const ALLOWED_IMAGE_KEYS: [&str; 16] = [
    "id",
    "machine_id",
    "plc_platform",
    "host",
    "rack",
    "slot",
    "db_number",
    "port",
    "address",
    "comm_mode",
    "login",
    "row_version",
    "created_at",
    "updated_at",
    "password_set",
    "password_changed",
];

/// Return the only entry that the store holds.
fn only_entry(fixture: &Fixture) -> recipe_domain::audit::AuditEntry {
    let mut entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    entries.remove(0)
}

/// Assert that an image holds no key outside [`ALLOWED_IMAGE_KEYS`].
fn assert_only_allowed_keys(image: Option<&serde_json::Value>) {
    let Some(object) = image.and_then(serde_json::Value::as_object) else {
        return;
    };
    for key in object.keys() {
        assert!(ALLOWED_IMAGE_KEYS.contains(&key.as_str()), "{key}");
    }
}

/// Return the id of the stored connection row of `machine`.
async fn stored_id(fixture: &Fixture, machine: MachineId) -> Uuid {
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let stored = tx
        .plc_connections()
        .connection_by_machine(machine)
        .await
        .unwrap()
        .unwrap();
    *stored.id.as_uuid()
}
