//! Tests of the use case `ExportPlcConnection` (tasks 5.8 and 5.9 of the change `plc-connection-config`).
//!
//! The specs are "Transfer of the connection" and "Secrecy of the connection password".

use std::sync::Arc;

use recipe_domain::audit::{AuditAction, RequestId};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{Permission, Role};
use recipe_domain::plant::{
    CommMode, DataBlockNumber, Machine, MachineCode, PlantError, PlcAddress, PlcConnection,
    PlcHost, PlcLogin, PlcPassword, PlcPlatform, Rack, Slot, TcpPort,
};

use super::{ExportPlcConnection, ExportPlcConnectionInput, ExportedPlcConnection};
use crate::auth::RequestMeta;
use crate::auth::{Operation, Requirement};
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{
    Fixture, actor, lock_line, machine_on, meta, mid, reference, timestamp,
};
use crate::memory::{CipherCall, Fault, JournalEntry, LockMode, MemoryCredentialCipher};
use crate::ports::{CipherFault, CredentialCipher as _, Isolation};
use serde_json::json;

pub(super) const PASSWORD: &str = "s3cret-pw-9";

pub(super) fn use_case(fixture: &Fixture, cipher: &MemoryCredentialCipher) -> ExportPlcConnection {
    ExportPlcConnection::new(fixture.context(), Arc::new(cipher.clone()))
}

pub(super) fn input(code: &str, recipe: &str) -> ExportPlcConnectionInput {
    ExportPlcConnectionInput {
        code: MachineCode::parse(code).unwrap(),
        recipe: reference(recipe),
    }
}

fn s7_record() -> PlcConnection {
    PlcConnection::S7 {
        host: PlcHost::parse("10.0.0.9").unwrap(),
        rack: Rack::parse("0").unwrap(),
        slot: Slot::parse("1").unwrap(),
    }
}

fn opc_ua_record() -> PlcConnection {
    PlcConnection::Iec {
        host: PlcHost::parse("plc.local").unwrap(),
        port: TcpPort::parse("4840").unwrap(),
        address: PlcAddress::parse("opc.tcp://plc.local:4840").unwrap(),
        mode: CommMode::OpcUa,
        login: Some(PlcLogin::parse("transfer").unwrap()),
    }
}

/// Store machine `M1` on `platform` with the draft recipe `R-1`.
fn machine_with_recipe(fixture: &Fixture, platform: PlcPlatform) -> Machine {
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    machine
}

/// Store an IEC machine `M1`, the recipe `R-1`, and an OPC UA record with a sealed password.
pub(super) fn iec_fixture(cipher: &MemoryCredentialCipher) -> Fixture {
    let fixture = Fixture::new();
    let machine = machine_with_recipe(&fixture, PlcPlatform::TwinCat3);
    let password = PlcPassword::parse(PASSWORD).unwrap();
    let sealed = cipher.seal(machine.id(), &password).unwrap();
    fixture
        .store
        .seed_connection(machine.id(), &opc_ua_record(), Some(sealed));
    fixture
}

async fn export(
    fixture: &Fixture,
    cipher: &MemoryCredentialCipher,
    role: Role,
    request: ExportPlcConnectionInput,
) -> Result<ExportedPlcConnection, AppError> {
    use_case(fixture, cipher)
        .execute(&actor(role), &meta(), request)
        .await
}

#[test]
fn the_use_case_declares_its_operation_and_the_permission_version_export() {
    assert_eq!(
        ExportPlcConnection::OPERATION,
        Operation::ExportPlcConnection
    );
    assert_eq!(
        Operation::ExportPlcConnection.requirement(),
        Requirement::Permission(Permission::VersionExport)
    );
}

#[test]
fn the_use_case_continues_in_read_only_mode() {
    // A24 and ADR-0154 (l): the gate of `E-WEB-006` in `recipe-web` reads this flag.
    assert!(ExportPlcConnection::OPERATION.runs_in_read_only());
}

#[tokio::test]
async fn every_role_passes_the_permission_check_before_any_read() {
    for role in Role::ALL {
        let fixture = Fixture::new();
        let cipher = MemoryCredentialCipher::default();
        let error = export(&fixture, &cipher, role, input("M9", "R-1"))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::WEB_005, "{role:?}");
        assert!(fixture.sink.events().is_empty(), "{role:?}");
    }
}

#[tokio::test]
async fn an_unknown_machine_gives_web_005() {
    let fixture = Fixture::new();
    let cipher = MemoryCredentialCipher::default();
    let error = export(&fixture, &cipher, Role::ExportClient, input("M9", "R-1"))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::NotFound { entity: Missing::Machine(code) }
            if code == MachineCode::parse("M9").unwrap()
    ));
}

#[tokio::test]
async fn an_unknown_recipe_gives_web_005() {
    let fixture = Fixture::new();
    machine_with_recipe(&fixture, PlcPlatform::S7_1500);
    let cipher = MemoryCredentialCipher::default();
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-77"))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::NotFound { entity: Missing::Recipe(ref number) }
            if *number == reference("R-77")
    ));
}

#[tokio::test]
async fn a_machine_without_a_record_gives_plt_027() {
    let fixture = Fixture::new();
    machine_with_recipe(&fixture, PlcPlatform::S7_1500);
    let cipher = MemoryCredentialCipher::default();
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_027);
    assert!(matches!(
        error,
        AppError::Plant(PlantError::ConnectionNotConfigured { code })
            if code == MachineCode::parse("M1").unwrap()
    ));
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn a_recipe_of_another_machine_gives_plt_029() {
    let fixture = Fixture::new();
    let first = machine_with_recipe(&fixture, PlcPlatform::S7_1500);
    fixture
        .store
        .seed_connection(first.id(), &s7_record(), None);
    let second = fixture.seed_machine_on(PlcPlatform::S7_1500, 2, "M2");
    fixture.seed_draft(&second, 2, "R-2");
    let cipher = MemoryCredentialCipher::default();
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_029);
    assert!(matches!(
        error,
        AppError::Plant(PlantError::ConnectionRecipeMismatch { code, reference: ref number })
            if code == MachineCode::parse("M1").unwrap() && *number == reference("R-2")
    ));
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn a_recipe_of_another_machine_wins_over_a_missing_record() {
    let fixture = Fixture::new();
    machine_with_recipe(&fixture, PlcPlatform::S7_1500);
    let second = fixture.seed_machine_on(PlcPlatform::S7_1500, 2, "M2");
    fixture.seed_draft(&second, 2, "R-2");
    let cipher = MemoryCredentialCipher::default();
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_029);
}

/// Store the archived machine `M1`, its recipe `R-1`, and optionally an S7 record.
fn archived_fixture(with_record: bool) -> Fixture {
    let fixture = Fixture::new();
    let machine = machine_with_recipe(&fixture, PlcPlatform::S7_1500);
    if with_record {
        fixture
            .store
            .seed_connection(machine.id(), &s7_record(), None);
    }
    let mut archived = machine_on(PlcPlatform::S7_1500, 1, "M1");
    archived.archive(timestamp(), None).unwrap();
    fixture.store.seed_machine(&archived);
    fixture
}

#[tokio::test]
async fn an_archived_machine_gives_plt_010() {
    let fixture = archived_fixture(true);
    let cipher = MemoryCredentialCipher::default();
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(matches!(
        error,
        AppError::Plant(PlantError::MachineArchived { code })
            if code == MachineCode::parse("M1").unwrap()
    ));
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn an_archived_machine_wins_over_a_missing_record() {
    let fixture = archived_fixture(false);
    let cipher = MemoryCredentialCipher::default();
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
}

#[tokio::test]
async fn an_archived_machine_wins_over_a_recipe_of_another_machine() {
    let fixture = archived_fixture(true);
    let other = fixture.seed_machine_on(PlcPlatform::S7_1500, 2, "M2");
    fixture.seed_draft(&other, 2, "R-2");
    let cipher = MemoryCredentialCipher::default();
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
}

#[tokio::test]
async fn a_live_machine_with_a_reused_code_wins_over_the_archived_machine() {
    let fixture = archived_fixture(true);
    let live = fixture.seed_machine_on(PlcPlatform::S7_1500, 2, "M1");
    fixture.seed_draft(&live, 2, "R-2");
    fixture.store.seed_connection(live.id(), &s7_record(), None);
    let cipher = MemoryCredentialCipher::default();
    let exported = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-2"))
        .await
        .unwrap();
    assert_eq!(exported.machine, mid(2));
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_029);
}

#[tokio::test]
async fn an_s7_record_returns_without_a_password_and_without_an_open() {
    let fixture = Fixture::new();
    let machine = machine_with_recipe(&fixture, PlcPlatform::S7_1500);
    fixture
        .store
        .seed_connection(machine.id(), &s7_record(), None);
    let cipher = MemoryCredentialCipher::default();
    let exported = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    assert_eq!(exported.machine, machine.id());
    assert_eq!(exported.code, machine.code());
    assert_eq!(exported.platform, PlcPlatform::S7_1500);
    assert_eq!(exported.recipe, reference("R-1"));
    assert_eq!(exported.row_version, 1);
    assert!(matches!(exported.connection, PlcConnection::S7 { .. }));
    assert!(exported.password.is_none());
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn an_s7_export_gives_the_number_of_the_machine_and_not_the_copy_of_the_record() {
    let fixture = Fixture::new();
    let machine = machine_with_recipe(&fixture, PlcPlatform::S7_1500);
    fixture
        .store
        .seed_connection(machine.id(), &s7_record(), None);
    // Simulate drift: the machine has 200 and the copy in the record has 100 (ADR-0156 (g)).
    fixture.store.update_state(|state, now| {
        let row = state.machines.get_mut(&machine.id()).unwrap();
        row.db_number = Some(DataBlockNumber::parse("200").unwrap());
        let record = state.connections.get_mut(&machine.id()).unwrap();
        record
            .copy_data_block(DataBlockNumber::parse("100").unwrap(), now)
            .unwrap();
    });
    let cipher = MemoryCredentialCipher::default();
    let exported = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    assert_eq!(
        exported.db_number,
        Some(DataBlockNumber::parse("200").unwrap())
    );
}

#[tokio::test]
async fn a_live_s7_machine_without_a_number_gives_db_008_and_no_entry() {
    let fixture = Fixture::new();
    let machine = machine_with_recipe(&fixture, PlcPlatform::S7_1500);
    fixture
        .store
        .seed_connection(machine.id(), &s7_record(), None);
    // Simulate a stored row that breaks the rule of ADR-0156 (a).
    fixture.store.update_state(|state, _now| {
        let row = state.machines.get_mut(&machine.id()).unwrap();
        row.db_number = None;
    });
    let cipher = MemoryCredentialCipher::default();
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_iec_export_has_no_data_block_number() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    let exported = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    assert_eq!(exported.db_number, None);
}

#[tokio::test]
async fn a_stored_password_opens_for_the_machine() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    let exported = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    assert_eq!(
        exported.password.as_ref().map(PlcPassword::expose),
        Some(PASSWORD)
    );
    assert_eq!(
        cipher.calls(),
        vec![CipherCall::Seal(mid(1)), CipherCall::Open(mid(1))]
    );
}

#[tokio::test]
async fn an_iec_record_without_a_password_opens_nothing() {
    let fixture = Fixture::new();
    let machine = machine_with_recipe(&fixture, PlcPlatform::TwinCat3);
    let record = PlcConnection::Iec {
        host: PlcHost::parse("plc.local").unwrap(),
        port: TcpPort::parse("502").unwrap(),
        address: PlcAddress::parse("plc.local:502").unwrap(),
        mode: CommMode::ModbusTcp,
        login: None,
    };
    fixture.store.seed_connection(machine.id(), &record, None);
    let cipher = MemoryCredentialCipher::default();
    let exported = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    assert!(exported.password.is_none());
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn the_reads_and_locks_follow_the_order_of_the_data_model() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForShare, mid(1).to_string()),
            lock_line("plc_connection", LockMode::ForShare, mid(1).to_string()),
        ]
    );
}

#[tokio::test]
async fn a_failed_open_gives_web_014_with_the_machine_code_and_no_secret() {
    for fault in [CipherFault::Format, CipherFault::Authentication] {
        let cipher = MemoryCredentialCipher::default();
        let fixture = iec_fixture(&cipher);
        cipher.fail_open(fault);
        let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::WEB_014, "{fault:?}");
        assert!(matches!(
            error,
            AppError::SecretUnreadable { code } if code == MachineCode::parse("M1").unwrap()
        ));
        let texts = [error.to_string(), format!("{error:?}")];
        assert!(texts.iter().all(|text| !text.contains(PASSWORD)));
        assert!(fixture.store.audit_entries().is_empty());
    }
}

#[tokio::test]
async fn a_sealed_secret_of_another_machine_gives_web_014() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = Fixture::new();
    let machine = machine_with_recipe(&fixture, PlcPlatform::TwinCat3);
    let password = PlcPassword::parse(PASSWORD).unwrap();
    let foreign = cipher.seal(mid(2), &password).unwrap();
    fixture
        .store
        .seed_connection(machine.id(), &opc_ua_record(), Some(foreign));
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_014);
}

#[tokio::test]
async fn the_output_hides_the_password_in_debug_text() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    let exported = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    assert!(!format!("{exported:?}").contains(PASSWORD));
}

/// The request of an `ExportClient` account with HTTP Basic: it has no session.
fn basic_meta() -> RequestMeta {
    RequestMeta::new(
        RequestId::from(uuid::Uuid::from_u128(0x77)),
        None,
        None,
        None,
    )
}

/// Read the id of the connection row of the machine `M1`.
async fn connection_id(fixture: &Fixture) -> uuid::Uuid {
    let mut tx = fixture
        .context()
        .uow
        .begin(Isolation::ReadOnly)
        .await
        .unwrap();
    let stored = tx
        .plc_connections()
        .connection_by_machine(mid(1))
        .await
        .unwrap()
        .unwrap();
    *stored.id.as_uuid()
}

#[tokio::test]
async fn a_success_commits_the_entry_before_it_returns_the_record() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    let exported = use_case(&fixture, &cipher)
        .execute(
            &actor(Role::ExportClient),
            &basic_meta(),
            input("M1", "R-1"),
        )
        .await
        .unwrap();
    assert!(exported.password.is_some());
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::PlcConnectionExported);
    assert_eq!(entry.target().entity_type(), "plc_connection");
    assert_eq!(*entry.target().entity_id(), connection_id(&fixture).await);
    assert_eq!(entry.target().aggregate_type().as_sql(), "machine");
    assert_eq!(*entry.target().aggregate_id(), *mid(1).as_uuid());
    assert_eq!(entry.before(), None);
    assert_eq!(
        entry.after(),
        Some(&json!({
            "machine_id": mid(1).to_string(),
            "row_version": 1,
            "schema_version": "1",
            "reference_number": "R-1",
        }))
    );
    assert_eq!(entry.actor(), Some(actor(Role::ExportClient).user()));
    assert_eq!(entry.request_id(), basic_meta().request_id());
    assert_eq!(entry.session_ref(), None);
}

#[tokio::test]
async fn an_s7_record_without_a_password_also_commits_the_entry() {
    let fixture = Fixture::new();
    let machine = machine_with_recipe(&fixture, PlcPlatform::S7_1500);
    fixture
        .store
        .seed_connection(machine.id(), &s7_record(), None);
    let cipher = MemoryCredentialCipher::default();
    export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries.first().unwrap().action(),
        AuditAction::PlcConnectionExported
    );
}

#[tokio::test]
async fn the_entry_holds_no_password() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    let text = format!("{entries:?}");
    assert!(!text.contains(PASSWORD));
    assert!(!text.contains("password"));
}

#[tokio::test]
async fn the_entry_reports_the_row_version_of_the_read_record() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    let exported = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    let after = entries.first().unwrap().after().unwrap();
    assert_eq!(after.get("row_version"), Some(&json!(exported.row_version)));
}

#[tokio::test]
async fn a_failed_audit_insert_gives_aud_001_without_the_password() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    fixture.store.fail_append_audit(Fault::Unavailable);
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::AUD_001);
    assert!(matches!(error, AppError::AuditWriteFailed { .. }));
    let texts = [error.to_string(), format!("{error:?}")];
    assert!(texts.iter().all(|text| !text.contains(PASSWORD)));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_failed_commit_gives_an_error_without_the_password_and_no_entry() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    fixture.store.fail_finish(Fault::Unavailable);
    let error = export(&fixture, &cipher, Role::ExportClient, input("M1", "R-1"))
        .await
        .unwrap_err();
    let texts = [error.to_string(), format!("{error:?}")];
    assert!(texts.iter().all(|text| !text.contains(PASSWORD)));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_refused_request_writes_no_entry() {
    let fixture = archived_fixture(true);
    let cipher = MemoryCredentialCipher::default();
    for (code, recipe) in [("M1", "R-1"), ("M9", "R-1"), ("M1", "R-77")] {
        export(&fixture, &cipher, Role::ExportClient, input(code, recipe))
            .await
            .unwrap_err();
    }
    assert!(fixture.store.audit_entries().is_empty());
}
