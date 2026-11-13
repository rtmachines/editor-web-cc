//! Value tests of the table `plc_connection`: ranges, text limits, the host format, and the AMS
//! Net ID (design D1, task 4.2).

use super::*;

// ---------------------------------------------------------------------------------------------
// Task 4.2: the range checks, the text limits, the host format, the control characters, and the
// AMS Net ID of the address. Each case changes one field of a valid row.
// ---------------------------------------------------------------------------------------------

/// Longest label of a host name (RFC 1123).
const LABEL_LENGTH: usize = 63;

/// Column values of an S7 record. A test changes one field of `S7::valid()`.
struct S7 {
    host: String,
    rack: i16,
    slot: i16,
    db_number: i32,
}

impl S7 {
    fn valid() -> Self {
        Self {
            host: "192.168.1.20".to_owned(),
            rack: 0,
            slot: 2,
            db_number: 10,
        }
    }
}

/// Column values of an IEC record. A test changes one field of a constructor result.
struct Iec {
    host: String,
    port: i32,
    address: String,
    mode: &'static str,
    login: Option<String>,
}

impl Iec {
    /// A valid `Ads` row. Use it on `TwinCat3`. It has no login.
    fn ads() -> Self {
        Self {
            host: "plc-line1.plant.local".to_owned(),
            port: 4840,
            address: "5.12.34.56.1.1".to_owned(),
            mode: "Ads",
            login: None,
        }
    }

    /// A valid `OpcUa` row with a login. The address of this mode is free text.
    fn opc_ua() -> Self {
        Self {
            host: "plc-line1.plant.local".to_owned(),
            port: 4840,
            address: "opc.tcp://plc-line1:4840".to_owned(),
            mode: "OpcUa",
            login: Some("operator".to_owned()),
        }
    }

    /// A row of `mode` that has the pair of login and password that the mode needs. An unknown mode
    /// gets no pair, so that only the mode check can fail.
    fn for_mode(mode: &'static str) -> Self {
        match mode {
            "Ads" => Self::ads(),
            "ModbusTcp" | "Unknown" => Self {
                mode,
                address: "1".to_owned(),
                login: None,
                ..Self::opc_ua()
            },
            _ => Self {
                mode,
                ..Self::opc_ua()
            },
        }
    }
}

/// Insert an S7 record for a new machine of `platform`.
async fn s7_case(conn: &mut PgConnection, platform: &str, row: &S7) -> Result<Uuid, sqlx::Error> {
    let machine = fixtures::machine(conn, platform).await;
    sqlx::query_scalar(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, rack, slot, db_number) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(&row.host)
    .bind(row.rack)
    .bind(row.slot)
    .bind(row.db_number)
    .fetch_one(conn)
    .await
}

/// Insert an IEC record for a new machine of `platform`. A login comes with a sealed password.
async fn iec_case(conn: &mut PgConnection, platform: &str, row: &Iec) -> Result<Uuid, sqlx::Error> {
    let machine = fixtures::machine(conn, platform).await;
    let sealed = row.login.as_ref().map(|_| vec![1_u8]);
    sqlx::query_scalar(
        "INSERT INTO plc_connection \
             (machine_id, plc_platform, host, port, address, comm_mode, login, password_sealed) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(&row.host)
    .bind(row.port)
    .bind(&row.address)
    .bind(row.mode)
    .bind(&row.login)
    .bind(sealed)
    .fetch_one(conn)
    .await
}

/// Return a host name of exactly `total` characters with labels of at most 63 characters.
fn host_of_length(total: usize) -> String {
    let mut host = String::new();
    while host.len() < total {
        if !host.is_empty() {
            host.push('.');
        }
        let remaining = total.saturating_sub(host.len());
        host.push_str(&"a".repeat(remaining.min(LABEL_LENGTH)));
    }
    host
}

#[tokio::test]
async fn plc_connection_rack_outside_0_to_7_violates_the_rack_range() {
    let mut conn = connection().await;

    for (rack, is_valid) in [(-1, false), (0, true), (7, true), (8, false)] {
        let row = S7 {
            rack,
            ..S7::valid()
        };
        let result = s7_case(&mut conn, S7_1500, &row).await;
        assert_verdict(
            &format!("rack {rack}"),
            result,
            is_valid,
            "plc_connection_rack_range",
        );
    }
}

#[tokio::test]
async fn plc_connection_slot_outside_0_to_31_violates_the_slot_range() {
    let mut conn = connection().await;

    for (slot, is_valid) in [(-1, false), (0, true), (31, true), (32, false)] {
        let row = S7 {
            slot,
            ..S7::valid()
        };
        let result = s7_case(&mut conn, S7_1500, &row).await;
        assert_verdict(
            &format!("slot {slot}"),
            result,
            is_valid,
            "plc_connection_slot_range",
        );
    }
}

#[tokio::test]
async fn plc_connection_db_number_outside_the_limit_of_the_platform_violates_the_range() {
    let mut conn = connection().await;
    let cases = [
        (S7_300, 0, false),
        (S7_300, 1, true),
        (S7_300, 16_000, true),
        (S7_300, 16_001, false),
        (S7_1200, 0, false),
        (S7_1200, 1, true),
        (S7_1200, 59_999, true),
        (S7_1200, 60_000, false),
        (S7_1500, 0, false),
        (S7_1500, 1, true),
        (S7_1500, 59_999, true),
        (S7_1500, 60_000, false),
    ];

    for (platform, db_number, is_valid) in cases {
        let row = S7 {
            db_number,
            ..S7::valid()
        };
        let result = s7_case(&mut conn, platform, &row).await;
        assert_verdict(
            &format!("{platform} db_number {db_number}"),
            result,
            is_valid,
            "plc_connection_db_number_range",
        );
    }
}

#[tokio::test]
async fn plc_connection_port_outside_1_to_65535_violates_the_port_range() {
    let mut conn = connection().await;

    for (port, is_valid) in [(0, false), (1, true), (65_535, true), (65_536, false)] {
        let row = Iec { port, ..Iec::ads() };
        let result = iec_case(&mut conn, TWIN_CAT, &row).await;
        assert_verdict(
            &format!("port {port}"),
            result,
            is_valid,
            "plc_connection_port_range",
        );
    }
}

#[tokio::test]
async fn plc_connection_mode_of_another_platform_violates_the_mode_value() {
    let mut conn = connection().await;
    let cases = [
        (CODESYS, "Ads", false),
        (CODESYS, "OpcUa", true),
        (CODESYS, "CodesysV3", true),
        (CODESYS, "ModbusTcp", true),
        (CODESYS, "Unknown", false),
        (TWIN_CAT, "Ads", true),
        (TWIN_CAT, "OpcUa", true),
        (TWIN_CAT, "CodesysV3", false),
        (TWIN_CAT, "ModbusTcp", true),
    ];

    for (platform, mode, is_valid) in cases {
        let result = iec_case(&mut conn, platform, &Iec::for_mode(mode)).await;
        assert_verdict(
            &format!("{platform} mode {mode}"),
            result,
            is_valid,
            "plc_connection_mode_value",
        );
    }
}

#[tokio::test]
async fn plc_connection_host_longer_than_253_characters_violates_the_host_length() {
    let mut conn = connection().await;

    for (length, is_valid) in [(253, true), (254, false)] {
        let row = Iec {
            host: host_of_length(length),
            ..Iec::opc_ua()
        };
        let result = iec_case(&mut conn, CODESYS, &row).await;
        assert_verdict(
            &format!("host of {length} characters"),
            result,
            is_valid,
            "plc_connection_host_length",
        );
    }
}

#[tokio::test]
async fn plc_connection_empty_host_violates_the_host_format() {
    let mut conn = connection().await;
    let row = S7 {
        host: String::new(),
        ..S7::valid()
    };

    // The empty host also breaks plc_connection_host_length. PostgreSQL reports the first
    // violated check in the order of the check names, and "format" sorts before "length".
    let result = s7_case(&mut conn, S7_1500, &row).await;

    assert_check("empty host", result, "plc_connection_host_format");
}

#[tokio::test]
async fn plc_connection_host_that_is_not_an_ipv4_address_or_a_host_name_violates_the_host_format() {
    let mut conn = connection().await;
    let valid = [
        "192.168.1.20",
        "0.0.0.0",
        "255.255.255.255",
        "plc-line1.plant.local",
        "PLC1",
        "1abc",
        "123.abc",
    ];
    let too_long_label = "a".repeat(LABEL_LENGTH + 1);
    let invalid = [
        "256.1.1.1",
        "01.2.3.4",
        "1.2.3",
        "1.2.3.4.5",
        "123",
        "10.0.0.1.",
        "plc_1",
        "-plc",
        "plc-",
        "plc.",
        ".plc",
        "a..b",
        "plc line",
        "plc\n",
        "plc.plant.\n",
        too_long_label.as_str(),
    ];

    for host in valid {
        let row = S7 {
            host: host.to_owned(),
            ..S7::valid()
        };
        let result = s7_case(&mut conn, S7_1500, &row).await;
        assert_accepted(&format!("host {host:?}"), result);
    }
    for host in invalid {
        let row = S7 {
            host: host.to_owned(),
            ..S7::valid()
        };
        let result = s7_case(&mut conn, S7_1500, &row).await;
        assert_check(
            &format!("host {host:?}"),
            result,
            "plc_connection_host_format",
        );
    }
}

#[tokio::test]
async fn plc_connection_address_outside_1_to_200_characters_violates_the_address_length() {
    let mut conn = connection().await;
    // Each letter has two bytes. The limit counts characters, not bytes.
    let cases = [
        (String::new(), false),
        ("a".to_owned(), true),
        ("a".repeat(200), true),
        ("a".repeat(201), false),
        ("\u{142}".repeat(200), true),
        ("\u{142}".repeat(201), false),
    ];

    for (address, is_valid) in cases {
        let row = Iec {
            address: address.clone(),
            ..Iec::opc_ua()
        };
        let result = iec_case(&mut conn, CODESYS, &row).await;
        assert_verdict(
            &format!("address of {} characters", address.chars().count()),
            result,
            is_valid,
            "plc_connection_address_length",
        );
    }
}

#[tokio::test]
async fn plc_connection_login_outside_1_to_64_characters_violates_the_login_length() {
    let mut conn = connection().await;
    let cases = [
        (String::new(), false),
        ("a".to_owned(), true),
        ("a".repeat(64), true),
        ("a".repeat(65), false),
        ("\u{142}".repeat(64), true),
        ("\u{142}".repeat(65), false),
    ];

    for (login, is_valid) in cases {
        let row = Iec {
            login: Some(login.clone()),
            ..Iec::opc_ua()
        };
        let result = iec_case(&mut conn, CODESYS, &row).await;
        assert_verdict(
            &format!("login of {} characters", login.chars().count()),
            result,
            is_valid,
            "plc_connection_login_length",
        );
    }
}

/// Text with one control character each: C0, DEL, and C1. The value objects refuse all of them.
const WITH_CONTROL: [&str; 6] = ["a\nb", "a\tb", "a\rb", "a\u{1b}b", "a\u{7f}b", "a\u{85}b"];

#[tokio::test]
async fn plc_connection_address_with_a_control_character_violates_the_text_check() {
    let mut conn = connection().await;

    for address in WITH_CONTROL {
        let row = Iec {
            address: address.to_owned(),
            ..Iec::opc_ua()
        };
        let result = iec_case(&mut conn, CODESYS, &row).await;
        assert_check(
            &format!("address {address:?}"),
            result,
            "plc_connection_text_no_control",
        );
    }
}

#[tokio::test]
async fn plc_connection_login_with_a_control_character_violates_the_text_check() {
    let mut conn = connection().await;

    for login in WITH_CONTROL {
        let row = Iec {
            login: Some(login.to_owned()),
            ..Iec::opc_ua()
        };
        let result = iec_case(&mut conn, CODESYS, &row).await;
        assert_check(
            &format!("login {login:?}"),
            result,
            "plc_connection_text_no_control",
        );
    }
}

#[tokio::test]
async fn plc_connection_ads_address_that_is_not_an_ams_net_id_violates_the_ams_check() {
    let mut conn = connection().await;
    let cases = [
        ("5.12.34.56.1.1", true),
        ("0.0.0.0.0.0", true),
        ("255.255.255.255.255.255", true),
        ("5.12.34.56.1", false),
        ("5.12.34.56.1.1.1", false),
        ("256.12.34.56.1.1", false),
        ("05.12.34.56.1.1", false),
        ("5.12.34.56.1.1.", false),
        ("5.12.34.56.1.1\n", false),
        ("a.b.c.d.e.f", false),
        ("plc1", false),
    ];

    for (address, is_valid) in cases {
        let row = Iec {
            address: address.to_owned(),
            ..Iec::ads()
        };
        let result = iec_case(&mut conn, TWIN_CAT, &row).await;
        assert_verdict(
            &format!("Ads address {address:?}"),
            result,
            is_valid,
            "plc_connection_ams_net_id",
        );
    }
}

#[tokio::test]
async fn plc_connection_address_of_another_mode_is_free_text() {
    let mut conn = connection().await;

    for mode in ["OpcUa", "CodesysV3", "ModbusTcp"] {
        let row = Iec {
            address: "not an ams net id".to_owned(),
            ..Iec::for_mode(mode)
        };
        let result = iec_case(&mut conn, CODESYS, &row).await;
        assert_accepted(&format!("mode {mode}"), result);
    }
}
