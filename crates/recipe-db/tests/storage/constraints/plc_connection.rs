//! Tests of `plc_connection` (`data-model.md` section 3.18, task 4.8).
//!
//! The range checks and the text checks of the table mirror the value objects of
//! `domain-model.md` section 2. Each test here sends the boundary values of one field through
//! both sides. The table gets an `INSERT`. The value objects get the same values as text, and
//! `PlcConnection::for_platform` applies the rules of the platform. The test passes when both
//! sides give the same verdict, and when this verdict equals the verdict that the case names.
//! A drift between a check and a value object is a failure of the check, so the fix goes into
//! the migration.
//!
//! The other tests of the table are in `tests/storage/plc_connection.rs` (tasks 4.1 to 4.4).
//! `COVERED` lists every constraint of the table. The tests of `plc_connection.rs` violate all
//! of them except `plc_connection_pkey` and `plc_connection_row_version_positive`. Those two
//! tests are at the end of this file.

use recipe_domain::plant::{
    CommMode, ConnectionFields, DataBlockNumber, MachineCode, PasswordState, PlcAddress,
    PlcConnection, PlcHost, PlcLogin, PlcPassword, PlcPlatform, Rack, Slot, TcpPort,
};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{CHECK_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION, assert_constraint};

/// The constraints of `plc_connection` that the tests of the table violate.
pub const COVERED: &[&str] = &[
    "plc_connection_pkey",
    "plc_connection_machine_unique",
    "plc_connection_machine_platform",
    "plc_connection_shape",
    "plc_connection_credentials",
    "plc_connection_rack_range",
    "plc_connection_slot_range",
    "plc_connection_db_number_range",
    "plc_connection_port_range",
    "plc_connection_mode_value",
    "plc_connection_host_length",
    "plc_connection_host_format",
    "plc_connection_address_length",
    "plc_connection_login_length",
    "plc_connection_text_no_control",
    "plc_connection_ams_net_id",
    "plc_connection_row_version_positive",
];

const S7_300: &str = "S7_300";
const S7_1200: &str = "S7_1200";
const S7_1500: &str = "S7_1500";
const CODESYS: &str = "CodesysIec";
/// Longest label of a host name (RFC 1123).
const LABEL_LENGTH: usize = 63;
/// The login of a valid IEC record that has credentials.
const LOGIN: &str = "operator";
/// The password of a valid IEC record that has credentials.
const PASSWORD: &str = "secret";
/// Any machine code. `for_platform` only copies it into an error.
const MACHINE_CODE: &str = "M1";

const HOST_CHECKS: &[&str] = &["plc_connection_host_format", "plc_connection_host_length"];
const ADDRESS_CHECKS: &[&str] = &[
    "plc_connection_address_length",
    "plc_connection_text_no_control",
];
const LOGIN_CHECKS: &[&str] = &[
    "plc_connection_login_length",
    "plc_connection_text_no_control",
];
const AMS_CHECKS: &[&str] = &[
    "plc_connection_ams_net_id",
    "plc_connection_address_length",
    "plc_connection_text_no_control",
];

/// The verdict that a case must get from both sides.
#[derive(Clone, Copy)]
enum Expect {
    /// Both sides accept the values.
    Accept,
    /// Both sides refuse the values. The table names one of these constraints.
    Reject(&'static [&'static str]),
}

/// The text values of one connection record. A test changes one field of a valid record.
#[derive(Clone, Default)]
struct Case {
    platform: &'static str,
    host: String,
    rack: Option<i16>,
    slot: Option<i16>,
    db_number: Option<i32>,
    port: Option<i32>,
    address: Option<String>,
    mode: Option<String>,
    login: Option<String>,
    password: Option<String>,
}

impl Case {
    /// A valid S7 record.
    fn s7(platform: &'static str) -> Self {
        Self {
            platform,
            host: "192.168.1.20".to_owned(),
            rack: Some(0),
            slot: Some(2),
            db_number: Some(10),
            ..Self::default()
        }
    }

    /// An IEC record of `mode`. It has the login and the password pair if the mode needs it.
    fn iec(platform: &'static str, mode: &str) -> Self {
        let has_pair = matches!(mode, "OpcUa" | "CodesysV3");
        Self {
            platform,
            host: "plc-line1.plant.local".to_owned(),
            port: Some(4840),
            address: Some("5.12.34.56.1.1".to_owned()),
            mode: Some(mode.to_owned()),
            login: has_pair.then(|| LOGIN.to_owned()),
            password: has_pair.then(|| PASSWORD.to_owned()),
            ..Self::default()
        }
    }

    /// A valid `OpcUa` record. Its address is free text.
    fn opc_ua() -> Self {
        Self::iec(CODESYS, "OpcUa")
    }

    /// A valid `Ads` record without a login.
    fn ads() -> Self {
        Self::iec(TWIN_CAT, "Ads")
    }

    fn with_host(self, host: &str) -> Self {
        Self {
            host: host.to_owned(),
            ..self
        }
    }

    fn with_address(self, address: &str) -> Self {
        Self {
            address: Some(address.to_owned()),
            ..self
        }
    }

    fn with_login(self, login: &str) -> Self {
        Self {
            login: Some(login.to_owned()),
            ..self
        }
    }
}

/// Insert the record for a new machine. Return the name of the violated check, or `Ok`.
async fn table_verdict(conn: &mut PgConnection, case: &Case) -> Result<(), String> {
    let machine = fixtures::machine(conn, case.platform).await;
    // The sealed password is a stand-in. The table only checks that it exists.
    let sealed = case.password.as_ref().map(|_| vec![1_u8]);
    let result = sqlx::query(
        "INSERT INTO plc_connection \
             (machine_id, plc_platform, host, rack, slot, db_number, port, address, comm_mode, \
              login, password_sealed) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(machine)
    .bind(case.platform)
    .bind(&case.host)
    .bind(case.rack)
    .bind(case.slot)
    .bind(case.db_number)
    .bind(case.port)
    .bind(&case.address)
    .bind(&case.mode)
    .bind(&case.login)
    .bind(sealed)
    .execute(conn)
    .await;
    let Err(error) = result else {
        return Ok(());
    };
    let database = error
        .as_database_error()
        .expect("the failure is a database error");
    assert_eq!(database.code().as_deref(), Some(CHECK_VIOLATION), "{error}");
    Err(database.constraint().unwrap_or_default().to_owned())
}

/// Parses the fields of a case with the value objects and remembers a refusal.
struct Parsed {
    is_valid: bool,
}

impl Parsed {
    /// Parse a field. A missing field stays missing. A bad text marks the case as refused.
    fn field<T, E>(&mut self, text: Option<&str>, parse: fn(&str) -> Result<T, E>) -> Option<T> {
        let text = text?;
        match parse(text) {
            Ok(value) => Some(value),
            Err(_) => {
                self.is_valid = false;
                None
            }
        }
    }
}

/// Tell whether the value objects and the rules of the platform accept the case.
fn domain_accepts(case: &Case) -> bool {
    let platform = PlcPlatform::parse(case.platform).expect("the case names a platform");
    let code = MachineCode::parse(MACHINE_CODE).expect("a valid machine code");
    let mut parsed = Parsed { is_valid: true };
    let rack = case.rack.map(|n| n.to_string());
    let slot = case.slot.map(|n| n.to_string());
    let db_number = case.db_number.map(|n| n.to_string());
    let port = case.port.map(|n| n.to_string());
    let fields = ConnectionFields {
        host: parsed.field(Some(&case.host), PlcHost::parse),
        rack: parsed.field(rack.as_deref(), Rack::parse),
        slot: parsed.field(slot.as_deref(), Slot::parse),
        db_number: parsed.field(db_number.as_deref(), DataBlockNumber::parse),
        port: parsed.field(port.as_deref(), TcpPort::parse),
        address: parsed.field(case.address.as_deref(), PlcAddress::parse),
        mode: parsed.field(case.mode.as_deref(), CommMode::parse),
        login: parsed.field(case.login.as_deref(), PlcLogin::parse),
        password: parsed.field(case.password.as_deref(), PlcPassword::parse),
    };
    parsed.is_valid
        && PlcConnection::for_platform(code, platform, fields, PasswordState::Absent).is_ok()
}

/// Run every case through the table and the value objects. Return one line for each drift or
/// wrong verdict.
async fn problems(conn: &mut PgConnection, cases: Vec<(String, Case, Expect)>) -> Vec<String> {
    let mut problems = Vec::new();
    for (label, case, expect) in cases {
        let table = table_verdict(conn, &case).await;
        let domain = domain_accepts(&case);
        let table_accepts = table.is_ok();
        if table_accepts != domain {
            problems.push(format!(
                "DRIFT {label}: the table says {table:?}, the value objects say accept={domain}"
            ));
            continue;
        }
        match (expect, table) {
            (Expect::Accept, Ok(())) => {}
            (Expect::Reject(names), Err(name)) if names.contains(&name.as_str()) => {}
            (_, table) => problems.push(format!(
                "VERDICT {label}: both sides gave {table:?}, the case expects another result"
            )),
        }
    }
    problems
}

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await
}

/// Run the cases on a new database and fail with the list of problems.
async fn assert_same(cases: Vec<(String, Case, Expect)>) {
    let mut conn = connection().await;
    let problems = problems(&mut conn, cases).await;
    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// Return a host name of 63-character labels and a last label of `tail` characters.
fn name_of(tail: usize) -> String {
    let label = "a".repeat(LABEL_LENGTH);
    format!("{label}.{label}.{label}.{}", "a".repeat(tail))
}

/// Make one case for each text. `build` turns a text into a record.
fn cases_of(
    field: &str,
    expect: Expect,
    texts: &[String],
    build: impl Fn(&str) -> Case,
) -> Vec<(String, Case, Expect)> {
    texts
        .iter()
        .map(|text| (format!("{field} {text:?}"), build(text), expect))
        .collect()
}

fn texts(values: &[&str]) -> Vec<String> {
    values.iter().map(|text| (*text).to_owned()).collect()
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

/// The same machine and the same id cannot hold two rows. The key `id` stops the second row.
#[tokio::test]
async fn plc_connection_duplicate_id_violates_the_primary_key() {
    let mut conn = connection().await;
    let first = fixtures::machine(&mut conn, S7_1500).await;
    let second = fixtures::machine(&mut conn, S7_1500).await;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, rack, slot, db_number) \
         VALUES ($1, 'S7_1500', '192.168.1.20', 0, 2, 10) RETURNING id",
    )
    .bind(first)
    .fetch_one(&mut conn)
    .await
    .unwrap();

    let result = sqlx::query(
        "INSERT INTO plc_connection (id, machine_id, plc_platform, host, rack, slot, db_number) \
         VALUES ($1, $2, 'S7_1500', '192.168.1.21', 0, 2, 10)",
    )
    .bind(id)
    .bind(second)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_connection_pkey");
}

/// The trigger `plc_connection_a_bump` sets `row_version` on every update, so an insert is the
/// only statement that can break the check.
#[tokio::test]
async fn plc_connection_row_version_below_1_violates_the_row_version_check() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    for row_version in [0_i64, -1] {
        let result = sqlx::query(
            "INSERT INTO plc_connection \
                 (machine_id, plc_platform, host, rack, slot, db_number, row_version) \
             VALUES ($1, 'S7_1500', '192.168.1.20', 0, 2, 10, $2)",
        )
        .bind(machine)
        .bind(row_version)
        .execute(&mut conn)
        .await;

        assert_constraint(
            result,
            CHECK_VIOLATION,
            "plc_connection_row_version_positive",
        );
    }
}
