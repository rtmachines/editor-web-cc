//! Tests of `GetPlcConnectionForm` on `MemoryStore` (task 5.7, ADR-0154, design D8).

use recipe_domain::audit::SecurityEventKind;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    CommMode, DataBlockNumber, MachineCode, PlcAddress, PlcConnection, PlcHost, PlcLogin,
    PlcPlatform, Rack, Slot, TcpPort,
};

use super::plc_connection::{GetPlcConnectionForm, PlcConnectionForm, PlcConnectionFormInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::JournalEntry;
use crate::memory::fixtures::{Fixture, actor, meta, mid, timestamp};
use crate::ports::{Isolation, SealedSecret};

/// A ciphertext that no output, `Debug` text, or audit entry may show.
const SEALED_BYTES: &[u8] = b"SEALED-BYTES-OF-THE-TEST";

fn input(code: &str) -> PlcConnectionFormInput {
    PlcConnectionFormInput {
        code: MachineCode::parse(code).unwrap(),
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

fn opcua_record() -> PlcConnection {
    PlcConnection::Iec {
        host: PlcHost::parse("plc.local").unwrap(),
        port: TcpPort::parse("4840").unwrap(),
        address: PlcAddress::parse("plc.local:4840").unwrap(),
        mode: CommMode::OpcUa,
        login: Some(PlcLogin::parse("adapter").unwrap()),
    }
}

fn sealed() -> SealedSecret {
    SealedSecret::from_bytes(SEALED_BYTES.to_vec())
}

async fn run(
    fixture: &Fixture,
    role: Role,
    request: PlcConnectionFormInput,
) -> Result<PlcConnectionForm, AppError> {
    GetPlcConnectionForm::new(fixture.context())
        .execute(&actor(role), &meta(), request)
        .await
}

#[test]
fn the_query_declares_its_operation() {
    assert_eq!(
        GetPlcConnectionForm::OPERATION,
        Operation::GetPlcConnectionForm
    );
}

#[tokio::test]
async fn an_operator_gets_forbidden_before_any_read_and_one_security_event() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");

    let error = run(&fixture, Role::Operator, input("M1"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(matches!(
        error,
        AppError::Forbidden {
            operation: Operation::GetPlcConnectionForm
        }
    ));
    assert!(fixture.store.journal().is_empty());
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

    let error = run(&fixture, Role::ExportClient, input("M1"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(fixture.store.journal().is_empty());
}

#[tokio::test]
async fn a_process_engineer_and_an_administrator_may_open_the_form() {
    for role in [Role::ProcessEngineer, Role::Administrator] {
        let fixture = Fixture::new();
        fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
        assert!(run(&fixture, role, input("M1")).await.is_ok());
    }
}

#[tokio::test]
async fn an_unknown_machine_gives_web_005() {
    let fixture = Fixture::new();

    let error = run(&fixture, Role::ProcessEngineer, input("M9"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound { entity: Missing::Machine(code) }
            if code == MachineCode::parse("M9").unwrap()
    ));
}

#[tokio::test]
async fn an_archived_machine_gives_web_005() {
    let fixture = Fixture::new();
    let mut machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    machine.archive(timestamp(), None).unwrap();
    fixture.store.seed_machine(&machine);

    let error = run(&fixture, Role::ProcessEngineer, input("M1"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
}

#[tokio::test]
async fn a_machine_without_a_record_gives_row_version_0_and_no_password() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");

    let form = run(&fixture, Role::ProcessEngineer, input("M1"))
        .await
        .unwrap();

    assert_eq!(form.machine, mid(1));
    assert_eq!(form.code, MachineCode::parse("M1").unwrap());
    assert_eq!(form.platform, PlcPlatform::S7_1500);
    assert!(form.connection.is_none());
    assert_eq!(form.row_version, 0);
    assert!(!form.password_set);
}

#[tokio::test]
async fn a_record_without_a_password_gives_password_set_false() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);

    let form = run(&fixture, Role::ProcessEngineer, input("M1"))
        .await
        .unwrap();

    assert_eq!(form.row_version, 1);
    assert!(!form.password_set);
    assert!(matches!(
        form.connection,
        Some(PlcConnection::S7 { ref host, .. }) if host.as_str() == "10.0.0.9"
    ));
}

#[tokio::test]
async fn a_record_with_a_password_gives_password_set_true_and_no_password_text() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture
        .store
        .seed_connection(mid(1), &opcua_record(), Some(sealed()));

    let form = run(&fixture, Role::Administrator, input("M1"))
        .await
        .unwrap();

    assert!(form.password_set);
    assert_eq!(form.row_version, 1);
    assert!(matches!(
        form.connection,
        Some(PlcConnection::Iec { mode: CommMode::OpcUa, ref login, .. })
            if login.as_ref().is_some_and(|login| login.as_str() == "adapter")
    ));
    let text = format!("{form:?}");
    assert!(!text.contains("SEALED"));
    assert!(!text.contains("password_sealed"));
}

#[tokio::test]
async fn the_query_reads_in_one_read_only_transaction_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);

    run(&fixture, Role::ProcessEngineer, input("M1"))
        .await
        .unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}
