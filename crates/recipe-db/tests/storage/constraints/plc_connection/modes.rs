//! Tests of the mode, the credentials, and the record shape of `plc_connection`.

use super::*;

#[tokio::test]
async fn mode_gets_the_same_verdict_from_the_check_and_from_comm_mode_and_the_platform() {
    let mut cases = Vec::new();
    for (platform, mode, expect) in [
        (
            CODESYS,
            "Ads",
            Expect::Reject(&["plc_connection_mode_value"][..]),
        ),
        (CODESYS, "OpcUa", Expect::Accept),
        (CODESYS, "CodesysV3", Expect::Accept),
        (CODESYS, "ModbusTcp", Expect::Accept),
        (TWIN_CAT, "Ads", Expect::Accept),
        (TWIN_CAT, "OpcUa", Expect::Accept),
        (
            TWIN_CAT,
            "CodesysV3",
            Expect::Reject(&["plc_connection_mode_value"][..]),
        ),
        (TWIN_CAT, "ModbusTcp", Expect::Accept),
    ] {
        cases.push((
            format!("{platform} {mode}"),
            Case::iec(platform, mode),
            expect,
        ));
    }
    // A name with another case, or with a space, is no mode on any platform.
    for mode in [
        "ads", "ADS", "opcua", "OPCUA", "Unknown", "", "Ads ", " Ads", "Modbus",
    ] {
        for platform in [CODESYS, TWIN_CAT] {
            cases.push((
                format!("{platform} {mode:?}"),
                Case::iec(platform, mode),
                Expect::Reject(&["plc_connection_mode_value"]),
            ));
        }
    }

    assert_same(cases).await;
}

#[tokio::test]
async fn login_and_password_get_the_same_verdict_for_each_mode() {
    const NEITHER: (bool, bool) = (false, false);
    const LOGIN_ONLY: (bool, bool) = (true, false);
    const PASSWORD_ONLY: (bool, bool) = (false, true);
    const BOTH: (bool, bool) = (true, true);
    let combinations = [NEITHER, LOGIN_ONLY, PASSWORD_ONLY, BOTH];
    let mut cases = Vec::new();
    // Each mode names the combinations that it accepts (ADR-0154 (ea)).
    for (platform, mode, accepted) in [
        (CODESYS, "OpcUa", &[BOTH][..]),
        (CODESYS, "CodesysV3", &[BOTH][..]),
        (CODESYS, "ModbusTcp", &[NEITHER][..]),
        (TWIN_CAT, "Ads", &[NEITHER, BOTH][..]),
        (TWIN_CAT, "OpcUa", &[BOTH][..]),
        (TWIN_CAT, "ModbusTcp", &[NEITHER][..]),
    ] {
        for (has_login, has_password) in combinations {
            let case = Case {
                login: has_login.then(|| LOGIN.to_owned()),
                password: has_password.then(|| PASSWORD.to_owned()),
                ..Case::iec(platform, mode)
            };
            let expect = if accepted.contains(&(has_login, has_password)) {
                Expect::Accept
            } else {
                Expect::Reject(&["plc_connection_credentials"])
            };
            cases.push((
                format!("{platform} {mode} login={has_login} password={has_password}"),
                case,
                expect,
            ));
        }
    }
    // An S7 record has no login and no password.
    for platform in [S7_300, S7_1200, S7_1500] {
        for (has_login, has_password) in combinations {
            let case = Case {
                login: has_login.then(|| LOGIN.to_owned()),
                password: has_password.then(|| PASSWORD.to_owned()),
                ..Case::s7(platform)
            };
            let expect = if has_login || has_password {
                Expect::Reject(&["plc_connection_credentials"])
            } else {
                Expect::Accept
            };
            cases.push((
                format!("{platform} login={has_login} password={has_password}"),
                case,
                expect,
            ));
        }
    }

    assert_same(cases).await;
}

#[tokio::test]
async fn shape_gets_the_same_verdict_from_the_check_and_from_the_platform_shape() {
    let shape = &["plc_connection_shape"][..];
    // PostgreSQL reports the first violated check in the order of the check names. A column that
    // the platform does not use can break a second check before the shape check.
    let shape_or_mode = &["plc_connection_mode_value", "plc_connection_shape"][..];
    let shape_or_block = &["plc_connection_db_number_range", "plc_connection_shape"][..];
    let cases = vec![
        (
            "S7 with a port".to_owned(),
            Case {
                port: Some(102),
                ..Case::s7(S7_1500)
            },
            Expect::Reject(shape),
        ),
        (
            "S7 with an address".to_owned(),
            Case::s7(S7_1500).with_address("5.12.34.56.1.1"),
            Expect::Reject(shape),
        ),
        (
            "S7 with a mode".to_owned(),
            Case {
                mode: Some("Ads".to_owned()),
                ..Case::s7(S7_1500)
            },
            Expect::Reject(shape_or_mode),
        ),
        (
            "S7 without a rack".to_owned(),
            Case {
                rack: None,
                ..Case::s7(S7_1500)
            },
            Expect::Reject(shape),
        ),
        (
            "S7 without a slot".to_owned(),
            Case {
                slot: None,
                ..Case::s7(S7_1500)
            },
            Expect::Reject(shape),
        ),
        (
            "S7 without a data block".to_owned(),
            Case {
                db_number: None,
                ..Case::s7(S7_1500)
            },
            Expect::Reject(shape),
        ),
        (
            "IEC with a rack".to_owned(),
            Case {
                rack: Some(0),
                ..Case::opc_ua()
            },
            Expect::Reject(shape),
        ),
        (
            "IEC with a slot".to_owned(),
            Case {
                slot: Some(0),
                ..Case::opc_ua()
            },
            Expect::Reject(shape),
        ),
        (
            "IEC with a data block".to_owned(),
            Case {
                db_number: Some(1),
                ..Case::opc_ua()
            },
            Expect::Reject(shape_or_block),
        ),
        (
            "IEC without a port".to_owned(),
            Case {
                port: None,
                ..Case::opc_ua()
            },
            Expect::Reject(shape),
        ),
        (
            "IEC without an address".to_owned(),
            Case {
                address: None,
                ..Case::opc_ua()
            },
            Expect::Reject(shape),
        ),
        (
            "IEC without a mode".to_owned(),
            Case {
                mode: None,
                login: None,
                password: None,
                ..Case::opc_ua()
            },
            Expect::Reject(shape),
        ),
    ];

    assert_same(cases).await;
}
