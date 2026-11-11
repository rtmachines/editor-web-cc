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

fn s7_fields(db_number: &str) -> ConnectionFields {
    ConnectionFields {
        host: Some(PlcHost::parse("10.0.0.5").unwrap()),
        rack: Some(Rack::parse("0").unwrap()),
        slot: Some(Slot::parse("2").unwrap()),
        db_number: Some(DataBlockNumber::parse(db_number).unwrap()),
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
        db_number: DataBlockNumber::parse("50").unwrap(),
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(SetPlcConnection::OPERATION, Operation::SetPlcConnection);
}

#[tokio::test]
async fn an_operator_gets_forbidden_before_any_read_and_one_security_event() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = use_case(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(matches!(
        error,
        AppError::Forbidden {
            operation: Operation::SetPlcConnection
        }
    ));
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert!(
        events
            .first()
            .is_some_and(|event| event.kind() == SecurityEventKind::Forbidden)
    );
}

#[tokio::test]
async fn an_export_client_gets_forbidden() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = use_case(fixture.context())
        .execute(&actor(Role::ExportClient), &meta(), input("M1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(fixture.store.journal().is_empty());
}

#[tokio::test]
async fn an_unknown_machine_gives_web_005() {
    let fixture = Fixture::new();
    let error = use_case(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M9"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound { entity: Missing::Machine(code) }
            if code == MachineCode::parse("M9").unwrap()
    ));
}

#[test]
fn the_command_stops_in_read_only_mode() {
    // The gate of `E-WEB-006` lives in `recipe-web` and reads this flag (`reliability.md` 7.8, A24).
    assert!(!SetPlcConnection::OPERATION.runs_in_read_only());
}

#[tokio::test]
async fn a_first_save_creates_the_record_at_row_version_1_and_writes_one_entry() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let output = use_case(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            save("M1", 0, s7_fields("100")),
        )
        .await
        .unwrap();
    assert_eq!(output.machine, mid(1));
    assert_eq!(output.row_version, 1);
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let stored = tx
        .plc_connections()
        .connection_by_machine(mid(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.row_version, 1);
    assert!(matches!(
        stored.connection,
        PlcConnection::S7 { ref host, .. } if host.as_str() == "10.0.0.5"
    ));
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::PlcConnectionSet);
    assert!(entry.before().is_none());
    assert_eq!(
        entry.after().and_then(|after| after.get("row_version")),
        Some(&json!(1))
    );
}

#[tokio::test]
async fn a_first_save_reads_first_and_then_locks_the_machine() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields("100")),
        )
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForNoKeyUpdate, mid(1).to_string()),
        ]
    );
}

#[tokio::test]
async fn an_update_locks_the_machine_before_the_record_and_adds_1_to_the_row_version() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);
    let output = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 1, s7_fields("100")),
        )
        .await
        .unwrap();
    assert_eq!(output.row_version, 2);
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForNoKeyUpdate, mid(1).to_string()),
            lock_line("plc_connection", LockMode::ForUpdate, mid(1).to_string()),
        ]
    );
    let entries = fixture.store.audit_entries();
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::PlcConnectionSet);
    assert_eq!(
        entry.before().and_then(|before| before.get("host")),
        Some(&json!("10.0.0.9"))
    );
    assert_eq!(
        entry.after().and_then(|after| after.get("host")),
        Some(&json!("10.0.0.5"))
    );
}

#[tokio::test]
async fn an_iec_first_save_without_credentials_passes() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let output = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, modbus_fields()),
        )
        .await
        .unwrap();
    assert_eq!(output.machine, mid(2));
    assert_eq!(output.row_version, 1);
}

#[tokio::test]
async fn a_stale_row_version_gives_web_013_with_both_versions_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);
    let error = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields("100")),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_013);
    assert!(matches!(
        error,
        AppError::ConnectionConcurrentEdit { expected: 0, actual: 1, code }
            if code == MachineCode::parse("M1").unwrap()
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_expected_version_above_zero_for_a_machine_without_a_record_gives_web_013() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let error = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 3, s7_fields("100")),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::ConnectionConcurrentEdit {
            expected: 3,
            actual: 0,
            ..
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn two_first_saves_that_race_give_one_record_and_one_web_013() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    // The hook runs after the read of step 2 and before the transaction of step 4.
    // It stands for the first save of another user, which commits in between.
    let context = fixture.context_racing(|store| {
        store.seed_connection(mid(1), &s7_record(), None);
    });
    let error = use_case(context)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields("100")),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_013);
    assert!(matches!(
        error,
        AppError::ConnectionConcurrentEdit {
            expected: 0,
            actual: 1,
            ..
        }
    ));
    assert_eq!(fixture.store.state().connections.len(), 1);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_machine_archived_between_the_read_and_the_lock_gives_plt_010_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived.archive(timestamp(), None).unwrap();
        store.seed_machine(&archived);
    });
    let error = use_case(context)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields("100")),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.store.state().connections.is_empty());
}

#[tokio::test]
async fn a_field_fault_gives_the_plant_errors_before_the_command_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let error = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields("60000")),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_024);
    assert!(matches!(error, AppError::Plants(_)));
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
}

#[tokio::test]
async fn a_failed_audit_insert_keeps_no_record() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.fail_append_audit(Fault::Unavailable);
    let error = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields("100")),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::AUD_001);
    assert!(fixture.store.state().connections.is_empty());
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

#[tokio::test]
async fn keep_leaves_the_stored_ciphertext_and_does_not_seal() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let before = sealed_before(mid(2), "stored-secret");
    fixture
        .store
        .seed_connection(mid(2), &opcua_record(), Some(before.clone()));
    let cipher = MemoryCredentialCipher::default();
    let output = use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, opcua_fields(None)),
        )
        .await
        .unwrap();
    assert_eq!(output.row_version, 2);
    let (sealed, state) = stored_password(&fixture, mid(2)).await;
    assert_eq!(sealed, Some(before));
    assert_eq!(state, PasswordState::Stored);
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn replace_stores_the_new_sealed_password_and_keeps_it_out_of_the_entry() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let before = sealed_before(mid(2), "stored-secret");
    fixture
        .store
        .seed_connection(mid(2), &opcua_record(), Some(before.clone()));
    let cipher = MemoryCredentialCipher::default();
    use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    assert_eq!(cipher.calls(), vec![CipherCall::Seal(mid(2))]);
    let (sealed, state) = stored_password(&fixture, mid(2)).await;
    let sealed = sealed.unwrap();
    assert_ne!(sealed, before);
    assert_eq!(state, PasswordState::Stored);
    let opened = cipher.open(mid(2), &sealed).unwrap();
    assert_eq!(opened.expose(), NEW_PASSWORD);
    let entries = fixture.store.audit_entries();
    let entry = entries.first().unwrap();
    let text = format!("{:?}", (entry.before(), entry.after()));
    assert!(!text.contains(NEW_PASSWORD));
    assert!(!text.contains("stored-secret"));
    assert!(!text.contains("password_sealed"));
}

#[tokio::test]
async fn a_first_save_with_a_password_stores_it_sealed() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let cipher = MemoryCredentialCipher::default();
    let output = use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    assert_eq!(output.row_version, 1);
    let (sealed, state) = stored_password(&fixture, mid(2)).await;
    assert_eq!(state, PasswordState::Stored);
    let opened = cipher.open(mid(2), &sealed.unwrap()).unwrap();
    assert_eq!(opened.expose(), NEW_PASSWORD);
}

#[tokio::test]
async fn clear_removes_the_stored_ciphertext_when_the_mode_has_no_credentials() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), "stored-secret")),
    );
    let cipher = MemoryCredentialCipher::default();
    let output = use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, modbus_fields()),
        )
        .await
        .unwrap();
    assert_eq!(output.row_version, 2);
    let (sealed, state) = stored_password(&fixture, mid(2)).await;
    assert!(sealed.is_none());
    assert_eq!(state, PasswordState::Absent);
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn an_s7_first_save_and_update_pass_without_a_password_and_never_seal() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let cipher = MemoryCredentialCipher::default();
    let command = use_case_with(fixture.context(), &cipher);
    command
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields("100")),
        )
        .await
        .unwrap();
    command
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 1, s7_fields("101")),
        )
        .await
        .unwrap();
    let (sealed, state) = stored_password(&fixture, mid(1)).await;
    assert!(sealed.is_none());
    assert_eq!(state, PasswordState::Absent);
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn the_password_is_sealed_before_the_command_transaction_opens() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let cipher = MemoryCredentialCipher::default();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let context = fixture.context_racing({
        let cipher = cipher.clone();
        let seen = Arc::clone(&seen);
        // The hook runs at the first `begin(ReadCommitted)`, so it sees the calls made before it.
        move |_store| *seen.lock() = cipher.calls()
    });
    use_case_with(context, &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    assert_eq!(*seen.lock(), vec![CipherCall::Seal(mid(2))]);
    assert_eq!(
        fixture.store.journal().first(),
        Some(&JournalEntry::Begin(Isolation::ReadOnly))
    );
}

#[tokio::test]
async fn a_failed_seal_gives_task_failed_token_before_the_command_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let cipher = MemoryCredentialCipher::default();
    cipher.fail_seal(CipherFault::Random);
    let error = use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Token
        }
    ));
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.store.state().connections.is_empty());
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

#[tokio::test]
async fn a_first_save_targets_the_row_id_with_the_machine_as_aggregate() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    use_case(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            save("M1", 0, s7_fields("100")),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    let target = entry.target();
    let row_id = stored_id(&fixture, mid(1)).await;
    assert_eq!(target.entity_type(), "plc_connection");
    assert_eq!(target.entity_id(), &row_id);
    assert_ne!(target.entity_id(), mid(1).as_uuid());
    assert_eq!(target.aggregate_type().as_sql(), "machine");
    assert_eq!(target.aggregate_id(), mid(1).as_uuid());
    let image_id = entry.after().and_then(|after| after.get("id"));
    assert_eq!(image_id, Some(&json!(row_id.to_string())));
}

#[tokio::test]
async fn an_update_targets_the_row_id_with_the_machine_as_aggregate() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);
    let row_id = stored_id(&fixture, mid(1)).await;
    use_case(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            save("M1", 1, s7_fields("100")),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    let target = entry.target();
    assert_eq!(target.entity_type(), "plc_connection");
    assert_eq!(target.entity_id(), &row_id);
    assert_ne!(target.entity_id(), mid(1).as_uuid());
    assert_eq!(target.aggregate_id(), mid(1).as_uuid());
    let image_id = entry.after().and_then(|after| after.get("id"));
    assert_eq!(image_id, Some(&json!(row_id.to_string())));
}

#[tokio::test]
async fn a_first_save_with_a_password_holds_the_full_row_and_both_flags_true() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    assert!(entry.before().is_none());
    let after = entry.after().unwrap();
    assert_eq!(after.get("host"), Some(&json!("plc.local")));
    assert_eq!(after.get("row_version"), Some(&json!(1)));
    assert_eq!(after.get("password_set"), Some(&json!(true)));
    assert_eq!(after.get("password_changed"), Some(&json!(true)));
    assert_only_allowed_keys(Some(after));
    assert!(!format!("{after:?}").contains(NEW_PASSWORD));
}

#[tokio::test]
async fn a_first_save_without_a_password_holds_both_flags_false() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    use_case(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            save("M1", 0, s7_fields("100")),
        )
        .await
        .unwrap();
    let after = only_entry(&fixture).after().cloned().unwrap();
    assert_eq!(after.get("password_set"), Some(&json!(false)));
    assert_eq!(after.get("password_changed"), Some(&json!(false)));
    assert_only_allowed_keys(Some(&after));
}

#[tokio::test]
async fn keep_holds_password_set_true_and_password_changed_false() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), "stored-secret")),
    );
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, opcua_fields(None)),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    let (before, after) = (entry.before().unwrap(), entry.after().unwrap());
    assert_eq!(before.get("password_set"), Some(&json!(true)));
    assert_eq!(after.get("password_set"), Some(&json!(true)));
    assert_eq!(after.get("password_changed"), Some(&json!(false)));
    assert_only_allowed_keys(Some(before));
    assert_only_allowed_keys(Some(after));
}

#[tokio::test]
async fn replace_holds_password_set_true_and_password_changed_true() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), "stored-secret")),
    );
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    let after = entry.after().unwrap();
    assert_eq!(after.get("password_set"), Some(&json!(true)));
    assert_eq!(after.get("password_changed"), Some(&json!(true)));
    let text = format!("{:?}", (entry.before(), entry.after()));
    assert!(!text.contains(NEW_PASSWORD) && !text.contains("stored-secret"));
}

#[tokio::test]
async fn clear_of_a_stored_password_holds_password_set_false_and_password_changed_true() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), "stored-secret")),
    );
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, modbus_fields()),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    assert_eq!(
        entry.before().and_then(|before| before.get("password_set")),
        Some(&json!(true))
    );
    let after = entry.after().unwrap();
    assert_eq!(after.get("password_set"), Some(&json!(false)));
    assert_eq!(after.get("password_changed"), Some(&json!(true)));
}

#[tokio::test]
async fn clear_without_a_stored_password_holds_password_changed_false() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, modbus_fields()),
        )
        .await
        .unwrap();
    let after = only_entry(&fixture).after().cloned().unwrap();
    assert_eq!(after.get("password_set"), Some(&json!(false)));
    assert_eq!(after.get("password_changed"), Some(&json!(false)));
}
