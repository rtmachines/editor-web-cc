//! Tests of the saved form that `SetPlcConnection` returns (follow-up of PR #52).
//!
//! The output holds the saved record, so the web layer needs no second read after the commit.

use std::sync::Arc;

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    CommMode, ConnectionFields, DataBlockNumber, MachineCode, MachineId, PlcAddress, PlcConnection,
    PlcHost, PlcLogin, PlcPassword, PlcPlatform, Rack, Slot, TcpPort,
};

use super::{SetPlcConnection, SetPlcConnectionInput, SetPlcConnectionOutput};
use crate::memory::MemoryCredentialCipher;
use crate::memory::fixtures::{Fixture, actor, meta, mid};
use crate::ports::{CredentialCipher as _, SealedSecret};

const NEW_PASSWORD: &str = "new-secret-1";
const STORED_PASSWORD: &str = "stored-secret";

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

fn modbus_fields() -> ConnectionFields {
    ConnectionFields {
        host: Some(PlcHost::parse("plc.local").unwrap()),
        port: Some(TcpPort::parse("502").unwrap()),
        address: Some(PlcAddress::parse("plc.local:502").unwrap()),
        mode: Some(CommMode::ModbusTcp),
        ..ConnectionFields::default()
    }
}

fn s7_fields() -> ConnectionFields {
    ConnectionFields {
        host: Some(PlcHost::parse("10.0.0.5").unwrap()),
        rack: Some(Rack::parse("0").unwrap()),
        slot: Some(Slot::parse("2").unwrap()),
        db_number: Some(DataBlockNumber::parse("100").unwrap()),
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

fn sealed_before(machine: MachineId, text: &str) -> SealedSecret {
    MemoryCredentialCipher::default()
        .seal(machine, &PlcPassword::parse(text).unwrap())
        .unwrap()
}

/// Run one save as an Administrator.
async fn run(
    fixture: &Fixture,
    code: &str,
    expected_row_version: i64,
    fields: ConnectionFields,
) -> SetPlcConnectionOutput {
    let command = SetPlcConnection::new(
        fixture.context(),
        Arc::new(MemoryCredentialCipher::default()),
    );
    let input = SetPlcConnectionInput {
        code: MachineCode::parse(code).unwrap(),
        expected_row_version,
        fields,
    };
    command
        .execute(&actor(Role::Administrator), &meta(), input)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_first_save_with_a_password_returns_the_saved_form_with_password_set() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let output = run(&fixture, "M2", 0, opcua_fields(Some(NEW_PASSWORD))).await;
    let form = &output.form;
    assert_eq!(form.machine, mid(2));
    assert_eq!(form.code, MachineCode::parse("M2").unwrap());
    assert_eq!(form.platform, PlcPlatform::TwinCat3);
    assert_eq!(form.row_version, 1);
    assert!(form.password_set);
    assert!(matches!(
        form.connection,
        Some(PlcConnection::Iec {
            mode: CommMode::OpcUa,
            ..
        })
    ));
    assert!(!format!("{output:?}").contains(NEW_PASSWORD));
}

#[tokio::test]
async fn a_save_with_an_empty_password_field_keeps_the_stored_password_set() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), STORED_PASSWORD)),
    );
    let output = run(&fixture, "M2", 1, opcua_fields(None)).await;
    assert_eq!(output.form.row_version, 2);
    assert!(output.form.password_set);
    assert!(!format!("{output:?}").contains(STORED_PASSWORD));
}

#[tokio::test]
async fn a_save_on_a_mode_without_credentials_returns_password_set_false() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), STORED_PASSWORD)),
    );
    let output = run(&fixture, "M2", 1, modbus_fields()).await;
    assert_eq!(output.form.row_version, 2);
    assert!(!output.form.password_set);
    assert!(matches!(
        output.form.connection,
        Some(PlcConnection::Iec {
            mode: CommMode::ModbusTcp,
            ..
        })
    ));
}

#[tokio::test]
async fn a_save_on_an_s7_platform_returns_the_s7_record_without_a_password() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let output = run(&fixture, "M1", 0, s7_fields()).await;
    assert_eq!(output.form.machine, mid(1));
    assert_eq!(output.form.platform, PlcPlatform::S7_1500);
    assert_eq!(output.form.row_version, 1);
    assert!(!output.form.password_set);
    assert!(matches!(
        output.form.connection,
        Some(PlcConnection::S7 { .. })
    ));
}
