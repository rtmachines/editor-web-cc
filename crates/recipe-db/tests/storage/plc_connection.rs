//! Tests of the table `plc_connection` of migration `0012_plc_connection.sql` (ADR-0154 (b),
//! design D1, task 4.1).
//!
//! Each violating row changes one column of a valid row, so the named constraint is the only one
//! that fails. The tests run as `recipe_migrator`, the owner of the tables.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{
    CHECK_VIOLATION, FOREIGN_KEY_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION,
    assert_constraint, assert_privilege_denied,
};

const S7_1500: &str = "S7_1500";

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await
}

/// Insert an S7 record for a machine. `port` is the only column that a test changes on this row.
async fn insert_s7(
    conn: &mut PgConnection,
    machine: Uuid,
    platform: &str,
    port: Option<i32>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, rack, slot, db_number, port) \
         VALUES ($1, $2, '192.168.1.20', 0, 2, 10, $3) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(port)
    .fetch_one(conn)
    .await
}

/// Insert an IEC record for a machine. `rack` is the only column that a test changes on this row.
async fn insert_iec(
    conn: &mut PgConnection,
    machine: Uuid,
    platform: &str,
    rack: Option<i16>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO plc_connection \
             (machine_id, plc_platform, host, rack, port, address, comm_mode) \
         VALUES ($1, $2, 'plc-line1.plant.local', $3, 4840, '5.12.34.56.1.1', 'Ads') RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(rack)
    .fetch_one(conn)
    .await
}

#[tokio::test]
async fn plc_connection_s7_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    let result = insert_s7(&mut conn, machine, S7_1500, None).await;

    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn plc_connection_iec_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = insert_iec(&mut conn, machine, TWIN_CAT, None).await;

    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn plc_connection_row_starts_at_row_version_1() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;
    insert_s7(&mut conn, machine, S7_1500, None).await.unwrap();

    let version: i64 =
        sqlx::query_scalar("SELECT row_version FROM plc_connection WHERE machine_id = $1")
            .bind(machine)
            .fetch_one(&mut conn)
            .await
            .unwrap();

    assert_eq!(version, 1);
}

#[tokio::test]
async fn plc_connection_row_of_another_platform_violates_the_machine_platform_key() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    let result = insert_iec(&mut conn, machine, TWIN_CAT, None).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "plc_connection_machine_platform",
    );
}

#[tokio::test]
async fn plc_connection_second_row_for_one_machine_violates_the_machine_unique_key() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;
    insert_s7(&mut conn, machine, S7_1500, None).await.unwrap();

    let result = insert_s7(&mut conn, machine, S7_1500, None).await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_connection_machine_unique");
}

#[tokio::test]
async fn plc_connection_s7_row_with_a_port_violates_the_shape_check() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    let result = insert_s7(&mut conn, machine, S7_1500, Some(102)).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_connection_shape");
}

#[tokio::test]
async fn plc_connection_iec_row_with_a_rack_violates_the_shape_check() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = insert_iec(&mut conn, machine, TWIN_CAT, Some(0)).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_connection_shape");
}

#[tokio::test]
async fn plc_connection_s7_row_without_a_rack_violates_the_shape_check() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    let result = sqlx::query(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, slot, db_number) \
         VALUES ($1, $2, '192.168.1.20', 2, 10)",
    )
    .bind(machine)
    .bind(S7_1500)
    .execute(&mut conn)
    .await;

    assert_constraint(result, CHECK_VIOLATION, "plc_connection_shape");
}

// ---------------------------------------------------------------------------------------------
// Task 4.2: the range checks, the text limits, the host format, the control characters, and the
// AMS Net ID of the address. Each case changes one field of a valid row.
// ---------------------------------------------------------------------------------------------

const S7_300: &str = "S7_300";
const S7_1200: &str = "S7_1200";
const CODESYS: &str = "CodesysIec";
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

/// Assert that the row was accepted.
fn assert_accepted(case: &str, result: Result<Uuid, sqlx::Error>) {
    assert!(result.is_ok(), "{case}: {result:?}");
}

/// Assert that the row violates the check `constraint`. `case` names the tested value.
fn assert_check(case: &str, result: Result<Uuid, sqlx::Error>, constraint: &str) {
    let error = result.expect_err(&format!("{case}: the check rejects the row"));
    let database = error
        .as_database_error()
        .expect("the failure is a database error");
    assert_eq!(
        database.code().as_deref(),
        Some(CHECK_VIOLATION),
        "{case}: {error}"
    );
    assert_eq!(database.constraint(), Some(constraint), "{case}: {error}");
}

/// Assert the verdict of one case: accepted, or a violation of `constraint`.
fn assert_verdict(case: &str, result: Result<Uuid, sqlx::Error>, is_valid: bool, constraint: &str) {
    if is_valid {
        assert_accepted(case, result);
    } else {
        assert_check(case, result, constraint);
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

// ---------------------------------------------------------------------------------------------
// Task 4.3: the check `plc_connection_credentials`. Login and sealed password come as a pair.
// The mode decides whether the pair is needed, permitted, or refused.
// ---------------------------------------------------------------------------------------------

/// Insert an IEC record for a new machine of `platform` with an explicit login and sealed
/// password, so that a test can break the pair.
async fn credential_case(
    conn: &mut PgConnection,
    platform: &str,
    mode: &str,
    login: Option<&str>,
    sealed: Option<&[u8]>,
) -> Result<Uuid, sqlx::Error> {
    let machine = fixtures::machine(conn, platform).await;
    sqlx::query_scalar(
        "INSERT INTO plc_connection \
             (machine_id, plc_platform, host, port, address, comm_mode, login, password_sealed) \
         VALUES ($1, $2, '192.168.1.20', 4840, '5.12.34.56.1.1', $3, $4, $5) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(mode)
    .bind(login)
    .bind(sealed)
    .fetch_one(conn)
    .await
}

/// Insert an S7 record for a new machine of `platform` with an explicit login and sealed password.
async fn s7_credential_case(
    conn: &mut PgConnection,
    platform: &str,
    login: Option<&str>,
    sealed: Option<&[u8]>,
) -> Result<Uuid, sqlx::Error> {
    let machine = fixtures::machine(conn, platform).await;
    sqlx::query_scalar(
        "INSERT INTO plc_connection \
             (machine_id, plc_platform, host, rack, slot, db_number, login, password_sealed) \
         VALUES ($1, $2, '192.168.1.20', 0, 2, 10, $3, $4) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(login)
    .bind(sealed)
    .fetch_one(conn)
    .await
}

const CREDENTIALS: &str = "plc_connection_credentials";
const SEALED: &[u8] = &[1_u8];

#[tokio::test]
async fn plc_connection_login_without_a_ciphertext_violates_the_credentials_check() {
    let mut conn = connection().await;

    for (platform, mode) in [
        (CODESYS, "OpcUa"),
        (CODESYS, "CodesysV3"),
        (TWIN_CAT, "Ads"),
    ] {
        let result = credential_case(&mut conn, platform, mode, Some("operator"), None).await;
        assert_check(&format!("login only, {mode}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_ciphertext_without_a_login_violates_the_credentials_check() {
    let mut conn = connection().await;

    for (platform, mode) in [
        (CODESYS, "OpcUa"),
        (CODESYS, "CodesysV3"),
        (TWIN_CAT, "Ads"),
    ] {
        let result = credential_case(&mut conn, platform, mode, None, Some(SEALED)).await;
        assert_check(&format!("ciphertext only, {mode}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_mode_that_needs_credentials_without_a_login_violates_the_credentials_check()
{
    let mut conn = connection().await;

    for mode in ["OpcUa", "CodesysV3"] {
        let result = credential_case(&mut conn, CODESYS, mode, None, None).await;
        assert_check(&format!("no pair, {mode}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_modbus_row_with_a_login_violates_the_credentials_check() {
    let mut conn = connection().await;

    for platform in [CODESYS, TWIN_CAT] {
        let result = credential_case(
            &mut conn,
            platform,
            "ModbusTcp",
            Some("operator"),
            Some(SEALED),
        )
        .await;
        assert_check(&format!("ModbusTcp on {platform}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_s7_row_with_a_login_violates_the_credentials_check() {
    let mut conn = connection().await;

    for platform in [S7_300, S7_1200, S7_1500] {
        let result = s7_credential_case(&mut conn, platform, Some("operator"), Some(SEALED)).await;
        assert_check(&format!("S7 login on {platform}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_row_with_the_pair_that_its_mode_permits_inserts() {
    let mut conn = connection().await;
    let pair = (Some("operator"), Some(SEALED));
    let cases = [
        (CODESYS, "OpcUa", pair),
        (CODESYS, "CodesysV3", pair),
        (CODESYS, "ModbusTcp", (None, None)),
        (TWIN_CAT, "OpcUa", pair),
        (TWIN_CAT, "Ads", pair),
        (TWIN_CAT, "Ads", (None, None)),
        (TWIN_CAT, "ModbusTcp", (None, None)),
    ];

    for (platform, mode, (login, sealed)) in cases {
        let result = credential_case(&mut conn, platform, mode, login, sealed).await;
        assert_accepted(&format!("{platform} {mode} login {login:?}"), result);
    }
    for platform in [S7_300, S7_1200, S7_1500] {
        let result = s7_credential_case(&mut conn, platform, None, None).await;
        assert_accepted(&format!("S7 without a login on {platform}"), result);
    }
}

// ---------------------------------------------------------------------------------------------
// Task 4.4: the trigger `plc_connection_a_bump` sets `row_version` and `updated_at`. The role
// `recipe_app` has no `UPDATE` grant on `row_version`.
// ---------------------------------------------------------------------------------------------

/// Create a database with one S7 record. Return the handle, the machine, and a session of the
/// owner of the tables.
async fn database_with_record() -> (TestDatabase, Uuid, PgConnection) {
    let database = TestDatabase::create().await;
    let mut owner = database.connect(Role::Migrator).await;
    let machine = fixtures::machine(&mut owner, S7_1500).await;
    insert_s7(&mut owner, machine, S7_1500, None).await.unwrap();
    (database, machine, owner)
}

/// Return `row_version` of the record of a machine.
async fn row_version_of(conn: &mut PgConnection, machine: Uuid) -> i64 {
    sqlx::query_scalar("SELECT row_version FROM plc_connection WHERE machine_id = $1")
        .bind(machine)
        .fetch_one(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn plc_connection_trigger_plc_connection_a_bump_is_a_before_update_row_trigger() {
    let (_database, _machine, mut owner) = database_with_record().await;

    let (function, is_row, is_before, is_update): (String, bool, bool, bool) = sqlx::query_as(
        "SELECT p.proname::text, \
                (t.tgtype & 1) = 1, (t.tgtype & 2) = 2, (t.tgtype & 16) = 16 \
         FROM pg_trigger t JOIN pg_proc p ON p.oid = t.tgfoid \
         WHERE t.tgrelid = 'plc_connection'::regclass AND t.tgname = 'plc_connection_a_bump'",
    )
    .fetch_one(&mut owner)
    .await
    .expect("the trigger plc_connection_a_bump exists");

    assert_eq!(function, "bump_row_version");
    assert!(is_row && is_before && is_update);
}

#[tokio::test]
async fn plc_connection_update_as_recipe_app_adds_one_to_row_version_and_sets_updated_at() {
    let (database, machine, mut owner) = database_with_record().await;
    let mut app = database.connect(Role::App).await;

    let changed =
        sqlx::query("UPDATE plc_connection SET host = '192.168.1.21' WHERE machine_id = $1")
            .bind(machine)
            .execute(&mut app)
            .await
            .unwrap();

    assert_eq!(changed.rows_affected(), 1);
    assert_eq!(row_version_of(&mut owner, machine).await, 2);
    let is_touched: bool = sqlx::query_scalar(
        "SELECT updated_at > created_at FROM plc_connection WHERE machine_id = $1",
    )
    .bind(machine)
    .fetch_one(&mut owner)
    .await
    .unwrap();
    assert!(is_touched, "the trigger sets updated_at to now()");
}

#[tokio::test]
async fn plc_connection_update_that_sets_row_version_as_recipe_app_gives_42501() {
    let (database, machine, mut owner) = database_with_record().await;
    let mut app = database.connect(Role::App).await;

    let result = sqlx::query("UPDATE plc_connection SET row_version = 99 WHERE machine_id = $1")
        .bind(machine)
        .execute(&mut app)
        .await;

    assert_privilege_denied(result);
    assert_eq!(row_version_of(&mut owner, machine).await, 1);
}

#[tokio::test]
async fn plc_connection_update_that_sets_row_version_as_the_owner_still_adds_one() {
    let (_database, machine, mut owner) = database_with_record().await;

    sqlx::query("UPDATE plc_connection SET row_version = 99 WHERE machine_id = $1")
        .bind(machine)
        .execute(&mut owner)
        .await
        .unwrap();

    assert_eq!(row_version_of(&mut owner, machine).await, 2);
}
