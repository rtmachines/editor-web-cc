use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::PasswordState;
use uuid::Uuid;

use super::{ConnectionRecord, build_connection};

fn record(platform: &str, mode: &str) -> ConnectionRecord {
    ConnectionRecord {
        id: Uuid::from_u128(1),
        plc_platform: platform.to_owned(),
        machine_code: "M7".to_owned(),
        host: "plc.local".to_owned(),
        rack: None,
        slot: None,
        port: Some(4840),
        address: Some("opc.tcp://plc.local:4840".to_owned()),
        comm_mode: Some(mode.to_owned()),
        login: Some("operator".to_owned()),
        row_version: 1,
    }
}

#[test]
fn a_stored_row_with_a_mode_that_the_platform_refuses_is_corrupt() {
    let row = record("TwinCat3", "CodesysV3");

    let error = build_connection(&row, PasswordState::Stored).unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008);
}

#[test]
fn a_stored_row_with_a_mode_that_the_platform_permits_builds() {
    let row = record("TwinCat3", "OpcUa");

    let built = build_connection(&row, PasswordState::Stored);

    assert!(built.is_ok());
}
