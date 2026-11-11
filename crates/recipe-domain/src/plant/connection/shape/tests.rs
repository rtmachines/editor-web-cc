use super::*;
use crate::error_codes::ErrorCode;
use crate::plant::PlcPassword;

const S7_PLATFORMS: [PlcPlatform; 3] = [
    PlcPlatform::S7_300,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];

fn machine() -> MachineCode {
    MachineCode::parse("M3").unwrap()
}

fn s7_fields() -> ConnectionFields {
    ConnectionFields {
        host: Some(PlcHost::parse("192.168.1.20").unwrap()),
        rack: Some(Rack::parse("0").unwrap()),
        slot: Some(Slot::parse("2").unwrap()),
        db_number: Some(DataBlockNumber::parse("10").unwrap()),
        ..ConnectionFields::default()
    }
}

fn run(platform: PlcPlatform, fields: ConnectionFields) -> Result<ConnectionPlan, PlantErrors> {
    PlcConnection::for_platform(machine(), platform, fields, PasswordState::Absent)
}

/// The code and the field name of each error of a refused request.
fn faults(result: Result<ConnectionPlan, PlantErrors>) -> Vec<(ErrorCode, &'static str)> {
    let errors = result.unwrap_err();
    errors
        .iter()
        .map(|error| {
            let field = match error {
                PlantError::ConnectionFieldNotOnPlatform { field, .. }
                | PlantError::ConnectionFieldMissing { field, .. }
                | PlantError::ConnectionFieldNotInMode { field, .. } => *field,
                PlantError::CommModeNotOnPlatform { .. } => "comm_mode",
                PlantError::AddressNotAmsNetId { .. } => "address",
                _ => "other error",
            };
            (error.code(), field)
        })
        .collect()
}

#[test]
fn s7_request_with_all_fields_passes_on_every_s7_platform() {
    for platform in S7_PLATFORMS {
        let plan = run(platform, s7_fields()).unwrap();
        assert!(
            matches!(
                &plan.connection,
                PlcConnection::S7 { rack, slot, db_number, host }
                    if rack.value() == 0
                        && slot.value() == 2
                        && db_number.value() == 10
                        && host.as_str() == "192.168.1.20"
            ),
            "{platform}: {plan:?}"
        );
        assert!(plan.password.is_none(), "{platform}");
    }
}

#[test]
fn s7_request_with_a_port_gives_e_plt_022() {
    for platform in S7_PLATFORMS {
        let fields = ConnectionFields {
            port: Some(TcpPort::parse("102").unwrap()),
            ..s7_fields()
        };
        assert_eq!(
            faults(run(platform, fields)),
            vec![(ErrorCode::PLT_022, "port")],
            "{platform}"
        );
    }
}

#[test]
fn s7_request_with_each_iec_field_gives_e_plt_022_for_that_field() {
    let cases: [(&str, ConnectionFields); 4] = [
        (
            "address",
            ConnectionFields {
                address: Some(PlcAddress::parse("PLC1").unwrap()),
                ..s7_fields()
            },
        ),
        (
            "comm_mode",
            ConnectionFields {
                mode: Some(CommMode::OpcUa),
                ..s7_fields()
            },
        ),
        (
            "login",
            ConnectionFields {
                login: Some(PlcLogin::parse("adapter").unwrap()),
                ..s7_fields()
            },
        ),
        (
            "password",
            ConnectionFields {
                password: Some(PlcPassword::parse("secret").unwrap()),
                ..s7_fields()
            },
        ),
    ];
    for (name, fields) in cases {
        assert_eq!(
            faults(run(PlcPlatform::S7_1500, fields)),
            vec![(ErrorCode::PLT_022, name)]
        );
    }
}

#[test]
fn s7_password_error_does_not_show_the_password() {
    let fields = ConnectionFields {
        password: Some(PlcPassword::parse("hunter2-secret").unwrap()),
        ..s7_fields()
    };
    let errors = run(PlcPlatform::S7_300, fields).unwrap_err();
    let shown = format!("{errors} {errors:?}");
    assert!(!shown.contains("hunter2-secret"));
}

#[test]
fn s7_request_without_a_slot_gives_e_plt_023() {
    for platform in S7_PLATFORMS {
        let fields = ConnectionFields {
            slot: None,
            ..s7_fields()
        };
        assert_eq!(
            faults(run(platform, fields)),
            vec![(ErrorCode::PLT_023, "slot")],
            "{platform}"
        );
    }
}

#[test]
fn s7_request_without_any_field_lists_every_missing_field_in_order() {
    assert_eq!(
        faults(run(PlcPlatform::S7_1200, ConnectionFields::default())),
        vec![
            (ErrorCode::PLT_023, "host"),
            (ErrorCode::PLT_023, "rack"),
            (ErrorCode::PLT_023, "slot"),
            (ErrorCode::PLT_023, "db_number"),
        ]
    );
}

#[test]
fn s7_request_lists_a_foreign_field_before_a_missing_field() {
    let fields = ConnectionFields {
        port: Some(TcpPort::parse("102").unwrap()),
        rack: None,
        ..s7_fields()
    };
    assert_eq!(
        faults(run(PlcPlatform::S7_1500, fields)),
        vec![(ErrorCode::PLT_022, "port"), (ErrorCode::PLT_023, "rack")]
    );
}

#[test]
fn stored_password_state_does_not_change_the_s7_result() {
    let plan = PlcConnection::for_platform(
        machine(),
        PlcPlatform::S7_300,
        s7_fields(),
        PasswordState::Stored,
    )
    .unwrap();
    assert!(plan.password.is_none());
}

fn with_data_block(text: &str) -> ConnectionFields {
    ConnectionFields {
        db_number: Some(DataBlockNumber::parse(text).unwrap()),
        ..s7_fields()
    }
}

#[test]
fn data_block_at_the_limit_passes_on_each_s7_platform() {
    let cases = [
        (PlcPlatform::S7_300, "16000"),
        (PlcPlatform::S7_1200, "59999"),
        (PlcPlatform::S7_1500, "59999"),
    ];
    for (platform, number) in cases {
        let plan = run(platform, with_data_block(number)).unwrap();
        assert!(
            matches!(&plan.connection, PlcConnection::S7 { db_number, .. }
                if db_number.value().to_string() == number),
            "{platform}: {plan:?}"
        );
    }
}

#[test]
fn data_block_above_the_limit_gives_e_plt_024_with_the_limit() {
    let cases = [
        (PlcPlatform::S7_300, "16001", 16_001_u16, 16_000_u16),
        (PlcPlatform::S7_1200, "60000", 60_000, 59_999),
        (PlcPlatform::S7_1500, "60000", 60_000, 59_999),
    ];
    for (platform, text, expected_number, expected_max) in cases {
        let errors = run(platform, with_data_block(text)).unwrap_err();
        assert_eq!(errors.iter().count(), 1, "{platform}");
        assert!(
            errors.iter().all(|error| matches!(
                error,
                PlantError::ConnectionDataBlockOutOfRange { code, number, max }
                    if *code == machine()
                        && *number == expected_number
                        && *max == expected_max
            )),
            "{platform}: {errors:?}"
        );
        assert!(
            errors
                .iter()
                .all(|error| error.code() == ErrorCode::PLT_024),
            "{platform}"
        );
    }
}

#[test]
fn data_block_limit_is_listed_after_a_missing_field() {
    let fields = ConnectionFields {
        rack: None,
        ..with_data_block("16001")
    };
    let codes: Vec<ErrorCode> = run(PlcPlatform::S7_300, fields)
        .unwrap_err()
        .iter()
        .map(PlantError::code)
        .collect();
    assert_eq!(codes, vec![ErrorCode::PLT_023, ErrorCode::PLT_024]);
}

const IEC_PLATFORMS: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];

/// A request of the mode `ModbusTcp`, which both IEC platforms permit and which needs no login.
fn iec_fields() -> ConnectionFields {
    ConnectionFields {
        host: Some(PlcHost::parse("192.168.1.30").unwrap()),
        port: Some(TcpPort::parse("502").unwrap()),
        address: Some(PlcAddress::parse("PLC1").unwrap()),
        mode: Some(CommMode::ModbusTcp),
        ..ConnectionFields::default()
    }
}

fn ads_fields(address: &str) -> ConnectionFields {
    ConnectionFields {
        port: Some(TcpPort::parse("48898").unwrap()),
        address: Some(PlcAddress::parse(address).unwrap()),
        mode: Some(CommMode::Ads),
        ..iec_fields()
    }
}

fn login() -> PlcLogin {
    PlcLogin::parse("adapter").unwrap()
}

fn password() -> PlcPassword {
    PlcPassword::parse("s3cret-word").unwrap()
}

/// A request of the given mode with a login and a new password.
fn with_credentials(mode: CommMode) -> ConnectionFields {
    ConnectionFields {
        mode: Some(mode),
        login: Some(login()),
        password: Some(password()),
        ..iec_fields()
    }
}

/// A request of the given mode with the credentials that the mode needs and no others.
fn credentials_of(mode: CommMode) -> ConnectionFields {
    match mode {
        CommMode::OpcUa | CommMode::CodesysV3 => with_credentials(mode),
        CommMode::Ads | CommMode::ModbusTcp => ConnectionFields {
            mode: Some(mode),
            ..iec_fields()
        },
    }
}

fn run_stored(
    platform: PlcPlatform,
    fields: ConnectionFields,
) -> Result<ConnectionPlan, PlantErrors> {
    PlcConnection::for_platform(machine(), platform, fields, PasswordState::Stored)
}

#[test]
fn iec_request_with_all_fields_passes_on_both_iec_platforms() {
    for platform in IEC_PLATFORMS {
        let plan = run(platform, iec_fields()).unwrap();
        assert!(
            matches!(
                &plan.connection,
                PlcConnection::Iec { host, port, address, mode, login }
                    if host.as_str() == "192.168.1.30"
                        && port.value() == 502
                        && address.as_str() == "PLC1"
                        && *mode == CommMode::ModbusTcp
                        && login.is_none()
            ),
            "{platform}: {plan:?}"
        );
    }
}

#[test]
fn iec_request_with_each_s7_field_gives_e_plt_022_for_that_field() {
    let cases: [(&str, ConnectionFields); 3] = [
        (
            "rack",
            ConnectionFields {
                rack: Some(Rack::parse("0").unwrap()),
                ..iec_fields()
            },
        ),
        (
            "slot",
            ConnectionFields {
                slot: Some(Slot::parse("2").unwrap()),
                ..iec_fields()
            },
        ),
        (
            "db_number",
            ConnectionFields {
                db_number: Some(DataBlockNumber::parse("10").unwrap()),
                ..iec_fields()
            },
        ),
    ];
    for (name, fields) in cases {
        assert_eq!(
            faults(run(PlcPlatform::CodesysIec, fields)),
            vec![(ErrorCode::PLT_022, name)]
        );
    }
}

#[test]
fn iec_request_with_a_rack_gives_e_plt_022_on_both_iec_platforms() {
    for platform in IEC_PLATFORMS {
        let fields = ConnectionFields {
            rack: Some(Rack::parse("0").unwrap()),
            ..iec_fields()
        };
        assert_eq!(
            faults(run(platform, fields)),
            vec![(ErrorCode::PLT_022, "rack")],
            "{platform}"
        );
    }
}

#[test]
fn iec_request_without_any_field_lists_every_missing_field_in_order() {
    for platform in IEC_PLATFORMS {
        assert_eq!(
            faults(run(platform, ConnectionFields::default())),
            vec![
                (ErrorCode::PLT_023, "host"),
                (ErrorCode::PLT_023, "port"),
                (ErrorCode::PLT_023, "address"),
                (ErrorCode::PLT_023, "comm_mode"),
            ],
            "{platform}"
        );
    }
}

#[test]
fn iec_request_lists_a_foreign_field_before_a_missing_field() {
    let fields = ConnectionFields {
        rack: Some(Rack::parse("0").unwrap()),
        port: None,
        ..iec_fields()
    };
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, fields)),
        vec![(ErrorCode::PLT_022, "rack"), (ErrorCode::PLT_023, "port")]
    );
}

#[test]
fn ads_on_codesys_iec_gives_e_plt_025_with_mode_and_platform() {
    let errors = run(PlcPlatform::CodesysIec, ads_fields("192.168.1.10.1.1")).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(
        matches!(
            errors.first(),
            PlantError::CommModeNotOnPlatform { code, mode, platform }
                if *code == machine()
                    && *mode == CommMode::Ads
                    && *platform == PlcPlatform::CodesysIec
        ),
        "{errors:?}"
    );
    assert_eq!(errors.first().code(), ErrorCode::PLT_025);
}

#[test]
fn codesys_v3_on_twincat_3_gives_e_plt_025() {
    let fields = with_credentials(CommMode::CodesysV3);
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, fields)),
        vec![(ErrorCode::PLT_025, "comm_mode")]
    );
}

#[test]
fn modes_of_each_iec_platform_are_permitted() {
    let cases = [
        (PlcPlatform::CodesysIec, CommMode::OpcUa),
        (PlcPlatform::CodesysIec, CommMode::CodesysV3),
        (PlcPlatform::CodesysIec, CommMode::ModbusTcp),
        (PlcPlatform::TwinCat3, CommMode::Ads),
        (PlcPlatform::TwinCat3, CommMode::OpcUa),
        (PlcPlatform::TwinCat3, CommMode::ModbusTcp),
    ];
    for (platform, mode) in cases {
        let fields = ConnectionFields {
            mode: Some(mode),
            address: Some(PlcAddress::parse("192.168.1.10.1.1").unwrap()),
            ..credentials_of(mode)
        };
        let plan = run(platform, fields).unwrap();
        assert!(
            matches!(&plan.connection, PlcConnection::Iec { mode: kept, .. } if *kept == mode),
            "{platform} {mode}: {plan:?}"
        );
    }
}

#[test]
fn ads_with_a_text_address_gives_e_plt_026_with_the_address() {
    let errors = run(PlcPlatform::TwinCat3, ads_fields("PLC1")).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(
        matches!(
            errors.first(),
            PlantError::AddressNotAmsNetId { code, address }
                if *code == machine() && address.as_str() == "PLC1"
        ),
        "{errors:?}"
    );
    assert_eq!(errors.first().code(), ErrorCode::PLT_026);
}

#[test]
fn ads_with_a_short_ams_net_id_gives_e_plt_026() {
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, ads_fields("192.168.1.10.1"))),
        vec![(ErrorCode::PLT_026, "address")]
    );
}

#[test]
fn ads_with_an_ams_net_id_passes_on_twincat_3() {
    let plan = run(PlcPlatform::TwinCat3, ads_fields("192.168.1.10.1.1")).unwrap();
    assert!(
        matches!(
            &plan.connection,
            PlcConnection::Iec { address, mode, .. }
                if address.as_str() == "192.168.1.10.1.1" && *mode == CommMode::Ads
        ),
        "{plan:?}"
    );
}

#[test]
fn text_address_passes_with_a_mode_other_than_ads() {
    let fields = ConnectionFields {
        mode: Some(CommMode::OpcUa),
        address: Some(PlcAddress::parse("opc.tcp://10.0.0.9:4840").unwrap()),
        ..with_credentials(CommMode::OpcUa)
    };
    assert!(run(PlcPlatform::CodesysIec, fields).is_ok());
}

#[test]
fn iec_mode_and_address_faults_follow_the_missing_field_faults() {
    let fields = ConnectionFields {
        host: None,
        ..ads_fields("PLC1")
    };
    let codes: Vec<ErrorCode> = run(PlcPlatform::CodesysIec, fields)
        .unwrap_err()
        .iter()
        .map(PlantError::code)
        .collect();
    assert_eq!(
        codes,
        vec![ErrorCode::PLT_023, ErrorCode::PLT_025, ErrorCode::PLT_026]
    );
}

#[test]
fn opc_ua_without_a_login_gives_e_plt_023_for_the_login() {
    let fields = ConnectionFields {
        login: None,
        ..with_credentials(CommMode::OpcUa)
    };
    assert_eq!(
        faults(run(PlcPlatform::CodesysIec, fields)),
        vec![(ErrorCode::PLT_023, "login")]
    );
}

#[test]
fn codesys_v3_without_a_login_gives_e_plt_023_for_the_login() {
    let fields = ConnectionFields {
        login: None,
        ..with_credentials(CommMode::CodesysV3)
    };
    assert_eq!(
        faults(run(PlcPlatform::CodesysIec, fields)),
        vec![(ErrorCode::PLT_023, "login")]
    );
}

#[test]
fn opc_ua_with_a_login_and_no_password_gives_e_plt_023_for_the_password() {
    let fields = ConnectionFields {
        password: None,
        ..with_credentials(CommMode::OpcUa)
    };
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, fields)),
        vec![(ErrorCode::PLT_023, "password")]
    );
}

#[test]
fn opc_ua_with_a_login_and_a_stored_password_passes() {
    let fields = ConnectionFields {
        password: None,
        ..with_credentials(CommMode::OpcUa)
    };
    assert!(run_stored(PlcPlatform::TwinCat3, fields).is_ok());
}

#[test]
fn opc_ua_without_a_login_and_a_stored_password_still_needs_the_login() {
    let fields = ConnectionFields {
        login: None,
        password: None,
        ..with_credentials(CommMode::OpcUa)
    };
    assert_eq!(
        faults(run_stored(PlcPlatform::CodesysIec, fields)),
        vec![(ErrorCode::PLT_023, "login")]
    );
}

#[test]
fn opc_ua_with_a_login_and_a_new_password_passes() {
    let plan = run(PlcPlatform::CodesysIec, with_credentials(CommMode::OpcUa)).unwrap();
    assert!(
        matches!(
            &plan.connection,
            PlcConnection::Iec { login: Some(kept), .. } if kept.as_str() == "adapter"
        ),
        "{plan:?}"
    );
}

#[test]
fn opc_ua_without_credentials_lists_the_login_before_the_password() {
    let fields = ConnectionFields {
        login: None,
        password: None,
        ..with_credentials(CommMode::OpcUa)
    };
    assert_eq!(
        faults(run(PlcPlatform::CodesysIec, fields)),
        vec![
            (ErrorCode::PLT_023, "login"),
            (ErrorCode::PLT_023, "password")
        ]
    );
}

#[test]
fn ads_without_a_login_and_without_a_password_passes() {
    assert!(run(PlcPlatform::TwinCat3, ads_fields("192.168.1.10.1.1")).is_ok());
}

#[test]
fn ads_with_a_password_and_no_login_gives_e_plt_023_for_the_login() {
    let fields = ConnectionFields {
        password: Some(password()),
        ..ads_fields("192.168.1.10.1.1")
    };
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, fields)),
        vec![(ErrorCode::PLT_023, "login")]
    );
}

#[test]
fn ads_with_a_login_and_no_password_gives_e_plt_023_for_the_password() {
    let fields = ConnectionFields {
        login: Some(login()),
        ..ads_fields("192.168.1.10.1.1")
    };
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, fields)),
        vec![(ErrorCode::PLT_023, "password")]
    );
}

#[test]
fn ads_with_a_login_and_a_stored_password_passes() {
    let fields = ConnectionFields {
        login: Some(login()),
        ..ads_fields("192.168.1.10.1.1")
    };
    assert!(run_stored(PlcPlatform::TwinCat3, fields).is_ok());
}

#[test]
fn ads_with_a_login_and_a_password_passes() {
    let fields = ConnectionFields {
        login: Some(login()),
        password: Some(password()),
        ..ads_fields("192.168.1.10.1.1")
    };
    assert!(run(PlcPlatform::TwinCat3, fields).is_ok());
}

#[test]
fn modbus_tcp_with_a_login_gives_e_plt_028_with_the_mode() {
    let fields = ConnectionFields {
        login: Some(login()),
        ..iec_fields()
    };
    let errors = run(PlcPlatform::CodesysIec, fields).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(
        matches!(
            errors.first(),
            PlantError::ConnectionFieldNotInMode { code, field, mode }
                if *code == machine() && *field == "login" && *mode == CommMode::ModbusTcp
        ),
        "{errors:?}"
    );
    assert_eq!(errors.first().code(), ErrorCode::PLT_028);
}

#[test]
fn modbus_tcp_with_a_password_gives_e_plt_028_for_the_password() {
    let fields = ConnectionFields {
        password: Some(password()),
        ..iec_fields()
    };
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, fields)),
        vec![(ErrorCode::PLT_028, "password")]
    );
}

#[test]
fn modbus_tcp_with_a_login_and_a_password_lists_both_fields() {
    assert_eq!(
        faults(run(
            PlcPlatform::CodesysIec,
            with_credentials(CommMode::ModbusTcp)
        )),
        vec![
            (ErrorCode::PLT_028, "login"),
            (ErrorCode::PLT_028, "password")
        ]
    );
}

#[test]
fn credential_faults_follow_the_address_fault() {
    let fields = ConnectionFields {
        password: Some(password()),
        ..ads_fields("PLC1")
    };
    let codes: Vec<ErrorCode> = run(PlcPlatform::TwinCat3, fields)
        .unwrap_err()
        .iter()
        .map(PlantError::code)
        .collect();
    assert_eq!(codes, vec![ErrorCode::PLT_026, ErrorCode::PLT_023]);
}

#[test]
fn credentials_are_not_checked_without_a_mode() {
    let fields = ConnectionFields {
        mode: None,
        login: Some(login()),
        ..iec_fields()
    };
    assert_eq!(
        faults(run(PlcPlatform::CodesysIec, fields)),
        vec![(ErrorCode::PLT_023, "comm_mode")]
    );
}

fn password_plan(
    platform: PlcPlatform,
    fields: ConnectionFields,
    stored: PasswordState,
) -> Option<PasswordPlan> {
    PlcConnection::for_platform(machine(), platform, fields, stored)
        .unwrap()
        .password
}

fn with_stored_state(stored: bool) -> PasswordState {
    if stored {
        PasswordState::Stored
    } else {
        PasswordState::Absent
    }
}

#[test]
fn empty_password_field_with_a_stored_password_gives_keep() {
    for mode in [CommMode::OpcUa, CommMode::CodesysV3] {
        let fields = ConnectionFields {
            password: None,
            ..with_credentials(mode)
        };
        let platform = PlcPlatform::CodesysIec;
        let plan = password_plan(platform, fields, PasswordState::Stored);
        assert!(
            matches!(plan, Some(PasswordPlan::Keep)),
            "{mode:?}: {plan:?}"
        );
    }
}

#[test]
fn ads_with_a_login_and_a_stored_password_gives_keep() {
    let fields = ConnectionFields {
        login: Some(login()),
        ..ads_fields("192.168.1.10.1.1")
    };
    let plan = password_plan(PlcPlatform::TwinCat3, fields, PasswordState::Stored);
    assert!(matches!(plan, Some(PasswordPlan::Keep)), "{plan:?}");
}

#[test]
fn empty_password_field_without_a_stored_password_gives_e_plt_023_on_opc_ua() {
    let fields = ConnectionFields {
        password: None,
        ..with_credentials(CommMode::OpcUa)
    };
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, fields)),
        vec![(ErrorCode::PLT_023, "password")]
    );
}

#[test]
fn a_new_password_gives_replace_with_that_password() {
    for stored in [PasswordState::Absent, PasswordState::Stored] {
        let plan = password_plan(
            PlcPlatform::CodesysIec,
            with_credentials(CommMode::OpcUa),
            stored,
        );
        assert!(
            matches!(&plan, Some(PasswordPlan::Replace(new)) if new.expose() == "s3cret-word"),
            "{stored:?}: {plan:?}"
        );
    }
}

#[test]
fn ads_with_a_login_and_a_new_password_gives_replace() {
    let fields = ConnectionFields {
        login: Some(login()),
        password: Some(password()),
        ..ads_fields("192.168.1.10.1.1")
    };
    let plan = password_plan(PlcPlatform::TwinCat3, fields, PasswordState::Absent);
    assert!(matches!(plan, Some(PasswordPlan::Replace(_))), "{plan:?}");
}

#[test]
fn modbus_tcp_gives_clear_with_and_without_a_stored_password() {
    for stored in [PasswordState::Absent, PasswordState::Stored] {
        let plan = password_plan(PlcPlatform::CodesysIec, iec_fields(), stored);
        assert!(
            matches!(plan, Some(PasswordPlan::Clear)),
            "{stored:?}: {plan:?}"
        );
    }
}

#[test]
fn ads_without_a_login_gives_clear_with_and_without_a_stored_password() {
    for stored in [PasswordState::Absent, PasswordState::Stored] {
        let plan = password_plan(
            PlcPlatform::TwinCat3,
            ads_fields("192.168.1.10.1.1"),
            stored,
        );
        assert!(
            matches!(plan, Some(PasswordPlan::Clear)),
            "{stored:?}: {plan:?}"
        );
    }
}

#[test]
fn s7_record_gives_no_password_plan_with_and_without_a_stored_password() {
    for platform in S7_PLATFORMS {
        for is_stored in [false, true] {
            let plan = password_plan(platform, s7_fields(), with_stored_state(is_stored));
            assert!(plan.is_none(), "{platform} {is_stored}: {plan:?}");
        }
    }
}

#[test]
fn opc_ua_with_a_rack_and_no_login_on_twincat_3_gives_both_faults_in_one_error_list() {
    let fields = ConnectionFields {
        rack: Some(Rack::parse("0").unwrap()),
        login: None,
        ..with_credentials(CommMode::OpcUa)
    };
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, fields)),
        vec![(ErrorCode::PLT_022, "rack"), (ErrorCode::PLT_023, "login")]
    );
}

#[test]
fn a_request_with_faults_of_every_kind_lists_all_of_them_in_check_order() {
    let fields = ConnectionFields {
        host: None,
        rack: Some(Rack::parse("0").unwrap()),
        mode: Some(CommMode::Ads),
        address: Some(PlcAddress::parse("PLC1").unwrap()),
        password: Some(password()),
        ..iec_fields()
    };
    assert_eq!(
        faults(run(PlcPlatform::CodesysIec, fields)),
        vec![
            (ErrorCode::PLT_022, "rack"),
            (ErrorCode::PLT_023, "host"),
            (ErrorCode::PLT_025, "comm_mode"),
            (ErrorCode::PLT_026, "address"),
            (ErrorCode::PLT_023, "login"),
        ]
    );
}
