//! Tests of the connection error codes, texts, and registry rows.

use super::*;
use crate::error_codes::REGISTRY;

type ConnectionCase = (PlantError, ErrorCode, &'static str, &'static str);

/// Field, mode, and range cases of the connection errors, E-PLT-022 to E-PLT-026.
fn connection_cases_settings(code: MachineCode) -> Vec<ConnectionCase> {
    let address = PlcAddress::parse("plc-1").unwrap();
    vec![
        (
            PlantError::ConnectionFieldNotOnPlatform {
                code,
                field: "rack",
                platform: PlcPlatform::TwinCat3,
            },
            ErrorCode::PLT_022,
            "E-PLT-022",
            "The connection of machine M7 has no field rack on platform TwinCat3. Delete the value of rack.",
        ),
        (
            PlantError::ConnectionFieldMissing {
                code,
                field: "slot",
            },
            ErrorCode::PLT_023,
            "E-PLT-023",
            "The connection of machine M7 needs a value in field slot. Enter a value.",
        ),
        (
            PlantError::DataBlockOutOfRange {
                code,
                number: 16001,
                max: 16000,
            },
            ErrorCode::PLT_024,
            "E-PLT-024",
            "Data block 16001 of machine M7 is above 16000. The controller of the machine has no such data block.",
        ),
        (
            PlantError::CommModeNotOnPlatform {
                code,
                mode: CommMode::Ads,
                platform: PlcPlatform::CodesysIec,
            },
            ErrorCode::PLT_025,
            "E-PLT-025",
            "Platform CodesysIec of machine M7 does not support the mode Ads. Select a mode of this platform.",
        ),
        (
            PlantError::AddressNotAmsNetId { code, address },
            ErrorCode::PLT_026,
            "E-PLT-026",
            "Address plc-1 of machine M7 is not an AMS Net ID. The mode Ads needs an AMS Net ID.",
        ),
    ]
}

/// Machine, recipe, and data block cases, E-PLT-027 to E-PLT-051.
fn connection_cases_machine(code: MachineCode) -> Vec<ConnectionCase> {
    let reference = ReferenceNumber::parse("r-100").unwrap();
    vec![
        (
            PlantError::ConnectionNotConfigured { code },
            ErrorCode::PLT_027,
            "E-PLT-027",
            "Machine M7 has no connection settings. Enter the settings on the page of the machine first.",
        ),
        (
            PlantError::ConnectionFieldNotInMode {
                code,
                field: "login",
                mode: CommMode::ModbusTcp,
            },
            ErrorCode::PLT_028,
            "E-PLT-028",
            "The mode ModbusTcp of machine M7 does not use field login. Delete the value of login.",
        ),
        (
            PlantError::ConnectionRecipeMismatch { code, reference },
            ErrorCode::PLT_029,
            "E-PLT-029",
            "Recipe R-100 does not belong to machine M7. Use a recipe of this machine.",
        ),
        (
            PlantError::DataBlockMissing {
                code,
                platform: PlcPlatform::S7_1500,
            },
            ErrorCode::PLT_050,
            "E-PLT-050",
            "Machine M7 on platform S7_1500 needs a data block number. Enter a number.",
        ),
        (
            PlantError::DataBlockNotOnPlatform {
                code,
                platform: PlcPlatform::TwinCat3,
            },
            ErrorCode::PLT_051,
            "E-PLT-051",
            "Platform TwinCat3 of machine M7 has no data block number. Delete the number.",
        ),
    ]
}

#[test]
fn connection_errors_have_their_code_registry_row_and_text() {
    let code = MachineCode::parse("M7").unwrap();
    let cases = connection_cases_settings(code)
        .into_iter()
        .chain(connection_cases_machine(code));
    for (error, expected, text, message) in cases {
        assert_eq!(error.code(), expected);
        assert_eq!(error.code().as_str(), text);
        assert!(REGISTRY.contains(&expected), "{text}");
        assert_eq!(error.to_string(), message);
    }
}
