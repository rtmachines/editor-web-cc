use super::*;

#[test]
fn s7_request_without_any_field_lists_every_missing_field_in_order() {
    assert_eq!(
        faults(run(PlcPlatform::S7_1200, ConnectionFields::default())),
        vec![
            (ErrorCode::PLT_023, "host"),
            (ErrorCode::PLT_023, "rack"),
            (ErrorCode::PLT_023, "slot"),
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
fn a_missing_login_comes_before_the_address_fault() {
    let fields = ConnectionFields {
        password: Some(password()),
        ..ads_fields("PLC1")
    };
    let codes: Vec<ErrorCode> = run(PlcPlatform::TwinCat3, fields)
        .unwrap_err()
        .iter()
        .map(PlantError::code)
        .collect();
    assert_eq!(codes, vec![ErrorCode::PLT_023, ErrorCode::PLT_026]);
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
            (ErrorCode::PLT_023, "login"),
            (ErrorCode::PLT_025, "comm_mode"),
            (ErrorCode::PLT_026, "address"),
        ]
    );
}

#[test]
fn a_missing_login_of_the_mode_comes_before_the_mode_fault() {
    let fields = ConnectionFields {
        mode: Some(CommMode::CodesysV3),
        login: None,
        password: None,
        ..iec_fields()
    };
    assert_eq!(
        faults(run(PlcPlatform::TwinCat3, fields)),
        vec![
            (ErrorCode::PLT_023, "login"),
            (ErrorCode::PLT_023, "password"),
            (ErrorCode::PLT_025, "comm_mode"),
        ]
    );
}
