//! Tests of the connection part of `GetMachine` on `MemoryStore` (fix G7, ADR-0154, design D8).
//!
//! The specs are "Connection of a machine" and "Secrecy of the connection password".

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    CommMode, MachineCode, PlcAddress, PlcConnection, PlcHost, PlcLogin, PlcPlatform, Rack, Slot,
    TcpPort,
};

use super::machines::{GetMachine, GetMachineInput, MachineConnection, MachineRef, MachineView};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, meta, mid};
use crate::ports::SealedSecret;

/// The probe password. No output and no `Debug` text may show it.
const PROBE_PASSWORD: &str = "S3cret-PLC";

fn opcua_record() -> PlcConnection {
    PlcConnection::Iec {
        host: PlcHost::parse("plc.local").unwrap(),
        port: TcpPort::parse("4840").unwrap(),
        address: PlcAddress::parse("plc.local:4840").unwrap(),
        mode: CommMode::OpcUa,
        login: Some(PlcLogin::parse("adapter").unwrap()),
    }
}

fn s7_record() -> PlcConnection {
    PlcConnection::S7 {
        host: PlcHost::parse("10.0.0.9").unwrap(),
        rack: Rack::parse("0").unwrap(),
        slot: Slot::parse("1").unwrap(),
    }
}

fn sealed() -> SealedSecret {
    SealedSecret::from_bytes(PROBE_PASSWORD.as_bytes().to_vec())
}

async fn get_as(fixture: &Fixture, role: Role) -> Result<MachineView, AppError> {
    let input = GetMachineInput {
        machine: MachineRef::Code(MachineCode::parse("M1").unwrap()),
        archived: None,
    };
    GetMachine::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
}

#[tokio::test]
async fn a_machine_with_an_iec_record_and_a_password_gives_the_record_and_password_set() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture
        .store
        .seed_connection(mid(1), &opcua_record(), Some(sealed()));

    let view = get_as(&fixture, Role::ProcessEngineer).await.unwrap();

    let connection = view.connection.unwrap();
    assert!(connection.password_set);
    assert!(matches!(
        connection.connection,
        PlcConnection::Iec { mode: CommMode::OpcUa, ref login, .. }
            if login.as_ref().is_some_and(|login| login.as_str() == "adapter")
    ));
}

#[tokio::test]
async fn a_record_without_a_password_gives_password_set_false() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);

    let view = get_as(&fixture, Role::ProcessEngineer).await.unwrap();

    let connection = view.connection.unwrap();
    assert!(!connection.password_set);
    assert!(matches!(
        connection.connection,
        PlcConnection::S7 { ref host, .. } if host.as_str() == "10.0.0.9"
    ));
}

#[tokio::test]
async fn a_machine_without_a_record_gives_no_connection_part() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");

    let view = get_as(&fixture, Role::ProcessEngineer).await.unwrap();

    assert!(view.connection.is_none());
}

#[tokio::test]
async fn an_operator_with_only_recipe_view_gets_the_connection_part() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture
        .store
        .seed_connection(mid(1), &opcua_record(), Some(sealed()));

    let view = get_as(&fixture, Role::Operator).await.unwrap();

    assert!(view.connection.is_some_and(|part| part.password_set));
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn the_view_holds_no_password_and_no_ciphertext() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture
        .store
        .seed_connection(mid(1), &opcua_record(), Some(sealed()));

    let view = get_as(&fixture, Role::Operator).await.unwrap();

    // The destructure names every field. A field of type `PlcPassword` or `SealedSecret`
    // would break this test at compile time, so the type shows the absence of a secret.
    let MachineConnection {
        connection: _,
        password_set: _,
    } = view.connection.as_ref().unwrap();
    let text = format!("{view:?}");
    assert!(!text.contains(PROBE_PASSWORD));
    assert!(!text.contains("SealedSecret"));
    assert!(!text.contains("password_sealed"));
}
