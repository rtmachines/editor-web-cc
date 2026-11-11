//! Tests of the connection route `GET /export/v1/machines/{code}/connection?recipe={ref}`
//! (task 8.1 of `plc-connection-config`, ADR-0154 (j), design D9, `api.md` sections 2.7 and 6.5).
//!
//! The data is the machine `M8` (`TwinCat3`) with the recipe `R-300`, and the machine `M9` (`S7_1500`)
//! with the recipe `R-310`. The `ExportClient` account `user1` sends every request with HTTP Basic.

use axum::http::StatusCode;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use recipe_app::blocking::{AppSettings, Blocking, ExportSettings};
use recipe_app::memory::Fault;
use recipe_app::ports::CredentialCipher;
use recipe_domain::audit::AuditAction;
use recipe_domain::identity::{Role, UserId};
use recipe_domain::plant::{
    CommMode, DataBlockNumber, Machine, MachineCode, MachineId, PlcAddress, PlcConnection, PlcHost,
    PlcLogin, PlcPassword, PlcPlatform, Rack, Slot, TcpPort, UnitId,
};
use recipe_domain::recipe::{Recipe, RecipeId, RecipeVersionId, ReferenceNumber};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::tests::{audit_count, client, client_get};
use crate::test_support::{Fixture, Reply, capture_logs, get_with};

/// The text of the PLC password of `M8`. It may appear in the answer of this route only.
pub(super) const PLC_PASSWORD: &str = "S3cret-PLC";
/// The route of the connection of `M8` for the recipe `R-300`.
pub(super) const M8_ROUTE: &str = "/export/v1/machines/M8/connection?recipe=R-300";
/// The route of the connection of `M9` for the recipe `R-310`.
pub(super) const M9_ROUTE: &str = "/export/v1/machines/M9/connection?recipe=R-310";
/// The password of the Basic accounts of the tests.
const ACCOUNT_PASSWORD: &str = "tr4nsfer-s3cret";

/// The expected body for `M8`, with the field order of `api.md` section 6.5.
const M8_BODY: &str = r#"{
  "schema_version": "1",
  "machine": {
    "code": "M8",
    "plc_platform": "TwinCat3"
  },
  "recipe": {
    "reference_number": "R-300"
  },
  "row_version": 1,
  "connection": {
    "host": "192.168.1.10",
    "port": 48898,
    "address": "192.168.1.10.1.1",
    "comm_mode": "Ads",
    "login": "adapter",
    "password": "S3cret-PLC"
  }
}"#;

/// The expected body for `M9`. An S7 record has no port, address, mode, login, or password.
const M9_BODY: &str = r#"{
  "schema_version": "1",
  "machine": {
    "code": "M9",
    "plc_platform": "S7_1500"
  },
  "recipe": {
    "reference_number": "R-310"
  },
  "row_version": 1,
  "connection": {
    "host": "plc-1.plant.local",
    "rack": 0,
    "slot": 2,
    "db_number": 100
  }
}"#;

/// Return an identifier with the first field `tag` and the last eight bytes of `number`.
fn id_of(tag: u32, number: u32) -> Uuid {
    Uuid::from_fields(tag, 0, 0, &u64::from(number).to_be_bytes())
}

/// Build a live machine `M<number>` with one unit, and keep it in the store.
fn seed_machine(fixture: &Fixture, number: u32, platform: PlcPlatform) -> MachineId {
    let id = MachineId::from(id_of(0x7357_0e10, number));
    let unit = UnitId::from(id_of(0x7357_0e11, number));
    let machine = Machine::create(
        id,
        MachineCode::parse(&format!("M{number}")).unwrap(),
        Text::parse("Spare line").unwrap(),
        platform,
        (unit, Text::parse("Infeed").unwrap()),
    );
    fixture.store.seed_machine(&machine);
    id
}

/// Store the recipe `reference` on `machine`. Its creator is the account `user1`.
fn seed_recipe(fixture: &Fixture, machine: MachineId, number: u32, reference: &str) {
    let recipe = Recipe::create(
        RecipeId::from(id_of(0x7357_0e12, number)),
        machine,
        ReferenceNumber::parse(reference).unwrap(),
        Text::parse("Connection recipe").unwrap(),
        RecipeVersionId::from(id_of(0x7357_0e13, number)),
    );
    let platform = fixture.store.state().machine(machine).unwrap().platform();
    fixture.store.seed_recipe(&recipe, platform, client());
}

/// Return the record of `M8`: a `TwinCat3` record with the mode `Ads` and a login.
fn ads_record() -> PlcConnection {
    PlcConnection::Iec {
        host: PlcHost::parse("192.168.1.10").unwrap(),
        port: TcpPort::parse("48898").unwrap(),
        address: PlcAddress::parse("192.168.1.10.1.1").unwrap(),
        mode: CommMode::Ads,
        login: Some(PlcLogin::parse("adapter").unwrap()),
    }
}

/// Return the record of `M9`: an S7 record.
fn s7_record() -> PlcConnection {
    PlcConnection::S7 {
        host: PlcHost::parse("plc-1.plant.local").unwrap(),
        rack: Rack::parse("0").unwrap(),
        slot: Slot::parse("2").unwrap(),
        db_number: DataBlockNumber::parse("100").unwrap(),
    }
}

/// Build a fixture with the accounts, `M8` with its record and password, and `M9` with its record.
pub(super) fn seeded() -> Fixture {
    seeded_in(Fixture::new())
}

/// Fill `fixture` with the data of [`seeded`].
fn seeded_in(fixture: Fixture) -> Fixture {
    fixture.seed_account(1, Role::ExportClient, ACCOUNT_PASSWORD, false);
    fixture.seed_account(2, Role::Operator, ACCOUNT_PASSWORD, false);
    let m8 = seed_machine(&fixture, 8, PlcPlatform::TwinCat3);
    seed_recipe(&fixture, m8, 8, "R-300");
    let sealed = fixture
        .cipher
        .seal(m8, &PlcPassword::parse(PLC_PASSWORD).unwrap())
        .unwrap();
    fixture
        .store
        .seed_connection(m8, &ads_record(), Some(sealed));
    let m9 = seed_machine(&fixture, 9, PlcPlatform::S7_1500);
    seed_recipe(&fixture, m9, 9, "R-310");
    fixture.store.seed_connection(m9, &s7_record(), None);
    fixture
}

/// Return the actor of each `PlcConnectionExported` entry, the oldest first.
pub(super) fn connection_entries(fixture: &Fixture) -> Vec<Option<UserId>> {
    fixture
        .store
        .audit_entries()
        .iter()
        .filter(|entry| entry.action() == AuditAction::PlcConnectionExported)
        .map(|entry| entry.actor())
        .collect()
}

#[tokio::test]
async fn connection_route_gives_the_document_of_an_iec_record_with_the_password() {
    let fixture = seeded();

    let reply = client_get(&fixture, M8_ROUTE).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(reply.header("cache-control"), "no-store");
    assert_eq!(reply.body, M8_BODY);
}

#[tokio::test]
async fn connection_route_gives_the_document_of_an_s7_record_without_the_iec_and_credential_fields()
{
    let fixture = seeded();

    let reply = client_get(&fixture, M9_ROUTE).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("cache-control"), "no-store");
    assert_eq!(reply.body, M9_BODY);
    for absent in ["port", "address", "comm_mode", "login", "password"] {
        assert!(
            !reply.body.contains(&format!("\"{absent}\"")),
            "{absent}: {}",
            reply.body
        );
    }
}

#[tokio::test]
async fn connection_route_of_an_iec_record_without_credentials_omits_login_and_password() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::ExportClient, ACCOUNT_PASSWORD, false);
    let machine = seed_machine(&fixture, 8, PlcPlatform::TwinCat3);
    seed_recipe(&fixture, machine, 8, "R-300");
    let record = PlcConnection::Iec {
        host: PlcHost::parse("192.168.1.10").unwrap(),
        port: TcpPort::parse("4840").unwrap(),
        address: PlcAddress::parse("192.168.1.10.1.1").unwrap(),
        mode: CommMode::Ads,
        login: None,
    };
    fixture.store.seed_connection(machine, &record, None);

    let reply = client_get(&fixture, M8_ROUTE).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("\"comm_mode\": \"Ads\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("\"login\""), "{}", reply.body);
    assert!(!reply.body.contains("\"password\""), "{}", reply.body);
}

#[tokio::test]
async fn connection_route_writes_one_entry_of_the_account_for_each_read() {
    let fixture = seeded();
    let before = connection_entries(&fixture).len();
    let entries = audit_count(&fixture);

    let first = client_get(&fixture, M8_ROUTE).await;
    let second = client_get(&fixture, M8_ROUTE).await;

    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    assert_eq!(second.status, StatusCode::OK, "{}", second.body);
    let after = connection_entries(&fixture);
    assert_eq!(after.len(), before + 2);
    assert_eq!(after.last(), Some(&Some(client())));
    assert_eq!(audit_count(&fixture), entries + 2);
}

#[tokio::test]
async fn connection_route_gives_a_person_account_401_and_no_document() {
    let fixture = seeded();
    let header = format!(
        "Basic {}",
        STANDARD.encode(format!("user2:{ACCOUNT_PASSWORD}"))
    );

    let reply = get_with(&fixture.router(), M8_ROUTE, &[("authorization", &header)]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
    assert!(reply.body.contains("E-IDN-003"), "{}", reply.body);
    assert!(!reply.body.contains(PLC_PASSWORD), "{}", reply.body);
    assert!(connection_entries(&fixture).is_empty());
}

#[tokio::test]
async fn connection_route_needs_the_basic_header() {
    let fixture = seeded();

    let reply = get_with(&fixture.router(), M8_ROUTE, &[]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-012"), "{}", reply.body);
    assert!(connection_entries(&fixture).is_empty());
}

#[tokio::test]
async fn connection_route_accepts_only_get() {
    use axum::body::Body;
    use axum::http::Method;

    let fixture = seeded();
    let header = format!(
        "Basic {}",
        STANDARD.encode(format!("user1:{ACCOUNT_PASSWORD}"))
    );
    for method in [Method::HEAD] {
        let builder =
            crate::test_support::request(method.clone(), M8_ROUTE).header("authorization", &header);

        let reply =
            crate::test_support::call(&fixture.router(), builder.body(Body::empty()).unwrap())
                .await;

        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{method}: {}",
            reply.body
        );
    }
    assert!(connection_entries(&fixture).is_empty());
}

#[tokio::test]
async fn connection_route_refuses_an_htmx_request_before_the_basic_check() {
    let fixture = seeded();
    let header = format!(
        "Basic {}",
        STANDARD.encode(format!("user1:{ACCOUNT_PASSWORD}"))
    );

    let reply = get_with(
        &fixture.router(),
        M8_ROUTE,
        &[("authorization", &header), ("hx-request", "true")],
    )
    .await;

    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"), "{}", reply.body);
    assert!(!reply.body.contains(PLC_PASSWORD), "{}", reply.body);
    assert_eq!(fixture.store.state().login_attempt_count(), 0);
    assert!(connection_entries(&fixture).is_empty());
}

#[tokio::test]
async fn connection_route_refuses_a_cross_site_request_before_the_basic_check() {
    let fixture = seeded();
    let header = format!(
        "Basic {}",
        STANDARD.encode(format!("user1:{ACCOUNT_PASSWORD}"))
    );

    let reply = get_with(
        &fixture.router(),
        M8_ROUTE,
        &[("authorization", &header), ("sec-fetch-site", "cross-site")],
    )
    .await;

    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"), "{}", reply.body);
    assert!(!reply.body.contains(PLC_PASSWORD), "{}", reply.body);
    assert!(connection_entries(&fixture).is_empty());
}

#[tokio::test]
async fn connection_route_of_a_machine_without_a_record_gives_not_found() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::ExportClient, ACCOUNT_PASSWORD, false);
    let machine = seed_machine(&fixture, 8, PlcPlatform::TwinCat3);
    seed_recipe(&fixture, machine, 8, "R-300");

    let reply = client_get(&fixture, M8_ROUTE).await;

    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert!(reply.body.contains("E-PLT-027"), "{}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json");
    assert!(connection_entries(&fixture).is_empty());
}

/// Archive the stored machine `machine` at a fixed instant.
fn archive(fixture: &Fixture, machine: MachineId) {
    let mut stored = fixture.store.state().machine(machine).unwrap();
    let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap());
    stored.archive(at, None).unwrap();
    fixture.store.seed_machine(&stored);
}

#[tokio::test]
async fn connection_route_resolves_the_live_machine_of_a_reused_code_and_refuses_an_old_recipe() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::ExportClient, ACCOUNT_PASSWORD, false);
    let old = seed_machine(&fixture, 8, PlcPlatform::TwinCat3);
    seed_recipe(&fixture, old, 8, "R-300");
    fixture.store.seed_connection(old, &ads_record(), None);
    archive(&fixture, old);
    let reused = MachineId::from(id_of(0x7357_0e10, 18));
    let unit = UnitId::from(id_of(0x7357_0e11, 18));
    let new = Machine::create(
        reused,
        MachineCode::parse("M8").unwrap(),
        Text::parse("Reused code").unwrap(),
        PlcPlatform::TwinCat3,
        (unit, Text::parse("Infeed").unwrap()),
    );
    fixture.store.seed_machine(&new);
    seed_recipe(&fixture, reused, 18, "R-301");
    fixture.store.seed_connection(reused, &ads_record(), None);

    let old_recipe = client_get(&fixture, M8_ROUTE).await;
    let new_recipe = client_get(&fixture, "/export/v1/machines/M8/connection?recipe=R-301").await;

    assert_eq!(
        old_recipe.status,
        StatusCode::NOT_FOUND,
        "{}",
        old_recipe.body
    );
    assert!(old_recipe.body.contains("E-PLT-029"), "{}", old_recipe.body);
    assert_eq!(old_recipe.header("content-type"), "application/json");
    assert_eq!(new_recipe.status, StatusCode::OK, "{}", new_recipe.body);
    assert_eq!(connection_entries(&fixture).len(), 1);
}

#[tokio::test]
async fn connection_route_without_a_usable_recipe_value_gives_not_found() {
    let fixture = seeded();
    for path in [
        "/export/v1/machines/M8/connection",
        "/export/v1/machines/M8/connection?recipe=",
        "/export/v1/machines/M8/connection?recipe=%00bad",
        "/export/v1/machines/M8/connection?recipe=R-300&extra=1",
        "/export/v1/machines/M8/connection?other=R-300",
    ] {
        let reply = client_get(&fixture, path).await;

        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-005"), "{path}: {}", reply.body);
        assert!(!reply.body.contains(PLC_PASSWORD), "{path}: {}", reply.body);
    }
    assert!(connection_entries(&fixture).is_empty());
}

/// Assert that the reply is a JSON refusal with `status` and `code`, and that no text of the reply
/// and no log line holds the PLC password.
fn assert_refusal_without_password(reply: &Reply, logs: &str, status: StatusCode, code: &str) {
    assert_eq!(reply.status, status, "{}", reply.body);
    assert!(reply.body.contains(code), "{}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json");
    assert!(!reply.body.contains(PLC_PASSWORD), "{}", reply.body);
    assert!(!logs.contains(PLC_PASSWORD), "{logs}");
    assert!(
        reply
            .headers
            .iter()
            .all(|(_name, value)| !String::from_utf8_lossy(value.as_bytes()).contains(PLC_PASSWORD)),
        "{:?}",
        reply.headers
    );
}

#[tokio::test]
async fn connection_route_gives_500_aud_001_and_no_password_when_the_audit_insert_fails() {
    let fixture = seeded();
    fixture.store.fail_append_audit(Fault::Unavailable);
    let (logs, _guard) = capture_logs();

    let reply = client_get(&fixture, M8_ROUTE).await;

    assert_refusal_without_password(
        &reply,
        &logs.text(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "E-AUD-001",
    );
    assert!(connection_entries(&fixture).is_empty());
}

#[tokio::test]
async fn connection_route_gives_no_password_and_no_entry_when_the_commit_fails() {
    let fixture = seeded();
    fixture.store.fail_finish(Fault::Unavailable);
    let (logs, _guard) = capture_logs();

    let reply = client_get(&fixture, M8_ROUTE).await;

    assert_refusal_without_password(
        &reply,
        &logs.text(),
        StatusCode::SERVICE_UNAVAILABLE,
        "E-DB-005",
    );
    assert!(connection_entries(&fixture).is_empty());
}

#[tokio::test]
async fn connection_route_gives_503_web_009_and_no_password_when_the_export_limit_ends() {
    // No export permit exists, so the wait for the permit ends at `timeout.export`.
    let settings = AppSettings {
        blocking: Blocking::new(Duration::from_secs(10), Duration::from_millis(50)),
        export: ExportSettings { permits: 0 },
        ..AppSettings::default()
    };
    let fixture = seeded_in(Fixture::with_app_settings(settings));
    let (logs, _guard) = capture_logs();

    let reply = client_get(&fixture, M8_ROUTE).await;

    assert_refusal_without_password(
        &reply,
        &logs.text(),
        StatusCode::SERVICE_UNAVAILABLE,
        "E-WEB-009",
    );
    assert!(connection_entries(&fixture).is_empty());
}
