//! Tests of the text fields, the numbers, and the host names of `plc_connection`.

use super::*;

/// Return a host name of 63-character labels and a last label of `tail` characters.
fn name_of(tail: usize) -> String {
    let label = "a".repeat(LABEL_LENGTH);
    format!("{label}.{label}.{label}.{}", "a".repeat(tail))
}
#[tokio::test]
async fn host_gets_the_same_verdict_from_the_check_and_from_plc_host() {
    let accepted = texts(&[
        "192.168.1.20",
        "0.0.0.0",
        "255.255.255.255",
        "10.0.0.5",
        "plc-line1.plant.local",
        "plc",
        "a",
        "PLC-1",
        "a1.b2",
        "1plc",
        "plc.1",
        "123.abc",
        "1.2.3.4a",
        "999.abc",
    ]);
    let refused = texts(&[
        "",
        "192.168.1.300",
        "256.1.1.1",
        "010.0.0.1",
        "01.2.3.4",
        "1.2.3.04",
        "1.2.3",
        "123",
        "1.2.3.4.5",
        "1.2.3.4.",
        "....",
        "1..2.3",
        "10.0.0.1.",
        "plc_01:102",
        "plc_1",
        "fe80::1",
        "plc.local.",
        "plc..local",
        ".plc",
        "-plc.local",
        "plc.-local",
        "plc-.local",
        "plc.local-",
        "plc line",
        " plc",
        "plc ",
        "plc\n",
        "plc.plant.\n",
        "1.2.3.4\n",
        "pl\u{107}.local",
        "\u{663}.\u{663}.\u{663}.\u{663}",
        "\u{ff11}.\u{ff12}.\u{ff13}.\u{ff14}",
    ]);
    let mut cases = cases_of("host", Expect::Accept, &accepted, |host| {
        Case::s7(S7_1500).with_host(host)
    });
    cases.extend(cases_of(
        "host",
        Expect::Reject(HOST_CHECKS),
        &refused,
        |host| Case::s7(S7_1500).with_host(host),
    ));
    let long_label = "a".repeat(LABEL_LENGTH);
    let too_long_label = "a".repeat(LABEL_LENGTH + 1);
    let longest = name_of(61);
    let too_long = name_of(62);
    assert_eq!(longest.len(), 253);
    assert_eq!(too_long.len(), 254);
    for (label, host, expect) in [
        ("label of 63", long_label, Expect::Accept),
        ("label of 64", too_long_label, Expect::Reject(HOST_CHECKS)),
        ("name of 253", longest, Expect::Accept),
        ("name of 254", too_long, Expect::Reject(HOST_CHECKS)),
    ] {
        cases.push((
            format!("host {label}"),
            Case::s7(S7_1500).with_host(&host),
            expect,
        ));
    }

    assert_same(cases).await;
}

#[tokio::test]
async fn host_gets_the_same_verdict_on_an_iec_record() {
    let cases = vec![
        (
            "ip".to_owned(),
            Case::opc_ua().with_host("10.1.2.3"),
            Expect::Accept,
        ),
        (
            "bad ip".to_owned(),
            Case::opc_ua().with_host("10.1.2.256"),
            Expect::Reject(HOST_CHECKS),
        ),
        (
            "underscore".to_owned(),
            Case::ads().with_host("plc_1"),
            Expect::Reject(HOST_CHECKS),
        ),
    ];

    assert_same(cases).await;
}

/// A case for each number. `build` puts the number into a record.
fn number_cases<T: std::fmt::Display + Copy>(
    field: &str,
    accepted: &[T],
    refused: &[T],
    check: &'static [&'static str],
    build: impl Fn(T) -> Case,
) -> Vec<(String, Case, Expect)> {
    let mut cases = Vec::new();
    for number in accepted {
        cases.push((format!("{field} {number}"), build(*number), Expect::Accept));
    }
    for number in refused {
        cases.push((
            format!("{field} {number}"),
            build(*number),
            Expect::Reject(check),
        ));
    }
    cases
}
#[tokio::test]
async fn rack_gets_the_same_verdict_from_the_check_and_from_rack() {
    let cases = number_cases(
        "rack",
        &[0, 1, 7],
        &[-1, 8, 255, i16::MAX, i16::MIN],
        &["plc_connection_rack_range"],
        |rack| Case {
            rack: Some(rack),
            ..Case::s7(S7_1500)
        },
    );

    assert_same(cases).await;
}

#[tokio::test]
async fn slot_gets_the_same_verdict_from_the_check_and_from_slot() {
    let cases = number_cases(
        "slot",
        &[0, 1, 31],
        &[-1, 32, 255, i16::MAX, i16::MIN],
        &["plc_connection_slot_range"],
        |slot| Case {
            slot: Some(slot),
            ..Case::s7(S7_1500)
        },
    );

    assert_same(cases).await;
}

#[tokio::test]
async fn port_gets_the_same_verdict_from_the_check_and_from_tcp_port() {
    let cases = number_cases(
        "port",
        &[1, 102, 4840, 65_535],
        &[0, -1, 65_536, i32::MAX, i32::MIN],
        &["plc_connection_port_range"],
        |port| Case {
            port: Some(port),
            ..Case::opc_ua()
        },
    );

    assert_same(cases).await;
}

#[tokio::test]
async fn db_number_gets_the_same_verdict_from_the_check_and_from_the_platform_limit() {
    let mut cases = Vec::new();
    for (platform, accepted, refused) in [
        (
            S7_300,
            &[1, 2, 15_999, 16_000][..],
            &[0, -1, 16_001, 59_999, 60_000, 65_535, 65_536, i32::MAX][..],
        ),
        (
            S7_1200,
            &[1, 2, 59_998, 59_999][..],
            &[0, -1, 60_000, 65_535, 65_536, i32::MAX][..],
        ),
        (
            S7_1500,
            &[1, 2, 59_998, 59_999][..],
            &[0, -1, 60_000, 65_535, 65_536, i32::MAX][..],
        ),
    ] {
        cases.extend(number_cases(
            &format!("{platform} db_number"),
            accepted,
            refused,
            &["plc_connection_db_number_range"],
            |db_number| Case {
                db_number: Some(db_number),
                ..Case::s7(platform)
            },
        ));
    }

    assert_same(cases).await;
}
/// The address and login texts: each length boundary, in characters and in bytes.
fn length_texts(max: usize) -> (Vec<String>, Vec<String>) {
    let accepted = vec![
        "a".to_owned(),
        "a".repeat(max),
        "\u{142}".repeat(max),
        "\u{1f600}".repeat(max),
    ];
    let refused = vec![
        String::new(),
        "a".repeat(max.saturating_add(1)),
        "\u{142}".repeat(max.saturating_add(1)),
        "\u{1f600}".repeat(max.saturating_add(1)),
    ];
    (accepted, refused)
}

/// Texts with one control character each (Unicode category Cc). NUL is not here, because
/// PostgreSQL cannot store it.
const CONTROL: &[&str] = &[
    "a\u{1}b", "a\tb", "a\nb", "a\rb", "a\u{1b}b", "a\u{1f}b", "a\u{7f}b", "a\u{80}b", "a\u{85}b",
    "a\u{9f}b", "\n", "a\n",
];

/// Texts with a character that is no control character, although it looks like a separator.
const NOT_CONTROL: &[&str] = &[
    "a b",
    " a",
    "a\u{a0}b",
    "a\u{a1}b",
    "a\u{ad}b",
    "a\u{2028}b",
    "a\u{2029}b",
    "a\u{200b}b",
    "a\u{feff}b",
    "a\u{e000}b",
    "\u{20ac}",
];

#[tokio::test]
async fn address_gets_the_same_verdict_from_the_check_and_from_plc_address() {
    let (accepted, refused) = length_texts(200);
    let mut cases = cases_of("address", Expect::Accept, &accepted, |text| {
        Case::opc_ua().with_address(text)
    });
    cases.extend(cases_of(
        "address",
        Expect::Reject(ADDRESS_CHECKS),
        &refused,
        |text| Case::opc_ua().with_address(text),
    ));
    cases.extend(cases_of(
        "address",
        Expect::Reject(ADDRESS_CHECKS),
        &texts(CONTROL),
        |text| Case::opc_ua().with_address(text),
    ));
    cases.extend(cases_of(
        "address",
        Expect::Accept,
        &texts(NOT_CONTROL),
        |text| Case::opc_ua().with_address(text),
    ));

    assert_same(cases).await;
}

#[tokio::test]
async fn login_gets_the_same_verdict_from_the_check_and_from_plc_login() {
    let (accepted, refused) = length_texts(64);
    let mut cases = cases_of("login", Expect::Accept, &accepted, |text| {
        Case::opc_ua().with_login(text)
    });
    cases.extend(cases_of(
        "login",
        Expect::Reject(LOGIN_CHECKS),
        &refused,
        |text| Case::opc_ua().with_login(text),
    ));
    cases.extend(cases_of(
        "login",
        Expect::Reject(LOGIN_CHECKS),
        &texts(CONTROL),
        |text| Case::opc_ua().with_login(text),
    ));
    cases.extend(cases_of(
        "login",
        Expect::Accept,
        &texts(NOT_CONTROL),
        |text| Case::opc_ua().with_login(text),
    ));

    assert_same(cases).await;
}

#[tokio::test]
async fn ads_address_gets_the_same_verdict_from_the_check_and_from_ams_net_id() {
    let accepted = texts(&[
        "5.12.34.56.1.1",
        "0.0.0.0.0.0",
        "255.255.255.255.255.255",
        "192.168.1.10.1.1",
        "10.0.0.5.1.1",
    ]);
    let refused = texts(&[
        "",
        "5.12.34.56.1",
        "5.12.34.56.1.1.1",
        "1",
        "256.12.34.56.1.1",
        "192.168.1.10.1.256",
        "1.2.3.4.5.1000",
        "1.2.3.4.5.99999999999",
        "05.12.34.56.1.1",
        "00.0.0.0.0.0",
        "1.2.3.4.5.007",
        "5.12.34.56.1.1.",
        ".5.12.34.56.1.1",
        "5..34.56.1.1",
        ".....",
        "5.12.34.56.1.1\n",
        "5.12.34.56.1.\n1",
        " 5.12.34.56.1.1",
        "5.12.34.56.1.1 ",
        "a.b.c.d.e.f",
        "5.12.34.56.1.x",
        "5.12.34.56.1.-1",
        "5.12.34.56.1.+1",
        "5.12.34.56.1.\u{663}",
        "5.12.34.56.1.\u{ff11}",
        "plc1",
    ]);
    let mut cases = cases_of("Ads address", Expect::Accept, &accepted, |text| {
        Case::ads().with_address(text)
    });
    cases.extend(cases_of(
        "Ads address",
        Expect::Reject(AMS_CHECKS),
        &refused,
        |text| Case::ads().with_address(text),
    ));

    assert_same(cases).await;
}

#[tokio::test]
async fn address_of_another_mode_is_free_text_for_the_check_and_for_the_domain() {
    let mut cases = Vec::new();
    for (platform, mode) in [
        (CODESYS, "OpcUa"),
        (CODESYS, "CodesysV3"),
        (CODESYS, "ModbusTcp"),
        (TWIN_CAT, "OpcUa"),
        (TWIN_CAT, "ModbusTcp"),
    ] {
        for address in [
            "not an ams net id",
            "1",
            "opc.tcp://plc-line1:4840",
            "5.12.34.56.1",
        ] {
            cases.push((
                format!("{platform} {mode} {address:?}"),
                Case::iec(platform, mode).with_address(address),
                Expect::Accept,
            ));
        }
    }

    assert_same(cases).await;
}
