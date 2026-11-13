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
//! tests are in the module `keys`.

use recipe_domain::plant::{
    CommMode, ConnectionFields, DataBlockNumber, MachineCode, PasswordState, PlcAddress,
    PlcConnection, PlcHost, PlcLogin, PlcPassword, PlcPlatform, Rack, Slot, TcpPort,
};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{CHECK_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION, assert_constraint};

mod fields;
mod keys;
mod modes;

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
        port: parsed.field(port.as_deref(), TcpPort::parse),
        address: parsed.field(case.address.as_deref(), PlcAddress::parse),
        mode: parsed.field(case.mode.as_deref(), CommMode::parse),
        login: parsed.field(case.login.as_deref(), PlcLogin::parse),
        password: parsed.field(case.password.as_deref(), PlcPassword::parse),
    };
    parsed.is_valid
        && db_number_accepted(&mut parsed, platform, db_number.as_deref())
        && parsed.is_valid
        && PlcConnection::for_platform(code, platform, fields, PasswordState::Absent).is_ok()
}

/// Tell whether the copy `db_number` of a row suits its platform. The domain connection no longer
/// holds the number (ADR-0156), so the case checks the number through `DataBlockNumber` and the
/// limit of the platform. An S7 row needs a number. An IEC row has none.
fn db_number_accepted(parsed: &mut Parsed, platform: PlcPlatform, text: Option<&str>) -> bool {
    let number = parsed.field(text, DataBlockNumber::parse);
    match (platform.max_data_block(), text) {
        (Some(max), Some(_)) => number.is_some_and(|number| number.value() <= max),
        (Some(_), None) | (None, Some(_)) => false,
        (None, None) => true,
    }
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
