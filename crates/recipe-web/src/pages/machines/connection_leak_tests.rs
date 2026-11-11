//! Leak tests of the PLC password (task 7.5 of `plc-connection-config`, ADR-0154 (h), design D6
//! and D9). The text `S3cret-PLC` may appear in the answer of the transfer read only. It never
//! appears in the answer of a save, a refusal, or a page, in a log line, or in an audit entry.

use axum::http::StatusCode;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use recipe_domain::identity::{Role, UserId};
use recipe_domain::plant::{MachineId, PlcPlatform};
use recipe_domain::recipe::{Recipe, RecipeId, RecipeVersionId, ReferenceNumber};
use recipe_domain::text::Text;
use uuid::Uuid;

use super::connection_tests::seed;
use crate::test_support::plant::{World, world};
use crate::test_support::{LogBuffer, Reply, capture_logs, get_with};

/// The text of the password that every test sends. It is the probe of the leak search.
const PLC_PASSWORD: &str = "S3cret-PLC";
/// The number of the `CodesysIec` machine `M102` that the tests use.
const MACHINE_NUMBER: u32 = 102;
/// The path of the connection route of `M102`.
const SAVE_PATH: &str = "/machines/M102/connection";
/// The number of the `ExportClient` account of the transfer read.
const CLIENT_NUMBER: u8 = 4;
/// The password of that account. It differs from `PLC_PASSWORD`.
const CLIENT_PASSWORD: &str = "tr4nsfer-s3cret";
/// The reference number of the recipe that the transfer read names.
const RECIPE_REFERENCE: &str = "R-400";
/// The route of the transfer read of `M102`.
const TRANSFER_PATH: &str = "/export/v1/machines/M102/connection?recipe=R-400";

/// A valid save of `M102` in the mode `OpcUa` with a login and the probe password.
const VALID_BODY: &str = "row_version=0&host=10.0.0.7&port=4840&address=opc.tcp%3A%2F%2Fplc&comm_mode=OpcUa&login=adapter&password=S3cret-PLC";
/// A save with a host that does not parse. The domain never sees it.
const BAD_HOST_BODY: &str = "row_version=0&host=bad%20host&port=4840&address=opc.tcp%3A%2F%2Fplc&comm_mode=OpcUa&login=adapter&password=S3cret-PLC";
/// A save that parses but breaks a rule of the domain: the mode `OpcUa` without a port.
const NO_PORT_BODY: &str = "row_version=0&host=10.0.0.7&port=&address=opc.tcp%3A%2F%2Fplc&comm_mode=OpcUa&login=adapter&password=S3cret-PLC";
/// A save with an unknown key. The form body has changed, so the answer is one `E-WEB-008` line.
const UNKNOWN_KEY_BODY: &str = "row_version=0&host=10.0.0.7&port=4840&address=opc.tcp%3A%2F%2Fplc&comm_mode=OpcUa&login=adapter&password=S3cret-PLC&extra=1";
/// A save that repeats the password key.
const REPEATED_KEY_BODY: &str = "row_version=0&host=10.0.0.7&port=4840&address=opc.tcp%3A%2F%2Fplc&comm_mode=OpcUa&login=adapter&password=S3cret-PLC&password=S3cret-PLC";

/// Build the world with the machine `M102` and the account of the transfer client.
fn leak_world() -> (World, MachineId) {
    let world = world();
    let machine = seed(&world, MACHINE_NUMBER, PlcPlatform::CodesysIec);
    world
        .fixture
        .seed_account(CLIENT_NUMBER, Role::ExportClient, CLIENT_PASSWORD, false);
    (world, machine)
}

/// Store a recipe on `M102`, so that the transfer read finds it.
fn seed_recipe(world: &World, machine: MachineId) {
    let recipe = Recipe::create(
        RecipeId::from(Uuid::from_u128(0x7357_0e12_0000_0000_0000_0000_0000_0066)),
        machine,
        ReferenceNumber::parse(RECIPE_REFERENCE).unwrap(),
        Text::parse("Connection recipe").unwrap(),
        RecipeVersionId::from(Uuid::from_u128(0x7357_0e13_0000_0000_0000_0000_0000_0066)),
    );
    let platform = world
        .fixture
        .store
        .state()
        .machine(machine)
        .unwrap()
        .platform();
    let client = UserId::from(Uuid::from_u128(u128::from(CLIENT_NUMBER)));
    world.fixture.store.seed_recipe(&recipe, platform, client);
}

/// Send the transfer read of `M102` with the Basic header of the client account.
async fn transfer_read(world: &World) -> Reply {
    let header = format!(
        "Basic {}",
        STANDARD.encode(format!("user{CLIENT_NUMBER}:{CLIENT_PASSWORD}"))
    );
    get_with(
        &world.fixture.router(),
        TRANSFER_PATH,
        &[("authorization", &header)],
    )
    .await
}

/// Assert that no audit entry of the store holds the probe, in any field.
fn assert_audit_clean(world: &World, what: &str) {
    let entries = format!("{:?}", world.fixture.store.audit_entries());
    assert!(!entries.contains(PLC_PASSWORD), "{what}: {entries}");
}

/// Assert that the answer and the log hold no probe, and that no audit entry holds it.
fn assert_no_leak(world: &World, reply: &Reply, logs: &LogBuffer, what: &str) {
    assert!(
        !reply.body.contains(PLC_PASSWORD),
        "{what}: body: {}",
        reply.body
    );
    for (name, value) in &reply.headers {
        let text = value.to_str().unwrap_or_default();
        assert!(!text.contains(PLC_PASSWORD), "{what}: header {name}");
    }
    let text = logs.text();
    assert!(!text.contains(PLC_PASSWORD), "{what}: log: {text}");
    assert_audit_clean(world, what);
}

#[tokio::test]
async fn refused_saves_never_echo_the_password_and_write_nothing() {
    let (logs, _guard) = capture_logs();
    let (world, _machine) = leak_world();
    let before = world.fixture.snapshot();
    let cases = [
        ("bad host", BAD_HOST_BODY, StatusCode::UNPROCESSABLE_ENTITY),
        ("no port", NO_PORT_BODY, StatusCode::UNPROCESSABLE_ENTITY),
        (
            "unknown key",
            UNKNOWN_KEY_BODY,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "repeated key",
            REPEATED_KEY_BODY,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ];

    for (what, body, status) in cases {
        let reply = world.post(&world.engineer, SAVE_PATH, body, true).await;
        assert_eq!(reply.status, status, "{what}: {}", reply.body);
        assert_no_leak(&world, &reply, &logs, what);
    }

    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn a_refused_save_gives_the_form_with_no_value_in_the_password_input() {
    let (world, _machine) = leak_world();

    let reply = world
        .post(&world.engineer, SAVE_PATH, BAD_HOST_BODY, true)
        .await;

    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let (_before, rest) = reply.body.split_once("<input id=\"password\"").unwrap();
    let tag = rest.split('>').next().unwrap();
    assert!(!tag.contains("value="), "{tag}");
    assert!(
        reply.body.contains("bad host"),
        "the other texts are echoed"
    );
}

#[tokio::test]
async fn a_stale_save_never_echoes_the_password() {
    let (logs, _guard) = capture_logs();
    let (world, _machine) = leak_world();
    let first = world
        .post(&world.engineer, SAVE_PATH, VALID_BODY, true)
        .await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);

    // The second save sends the old row version, so it must lose.
    let stale = world
        .post(&world.engineer, SAVE_PATH, VALID_BODY, true)
        .await;

    assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
    assert_no_leak(&world, &stale, &logs, "stale save");
}

#[tokio::test]
async fn a_successful_save_never_shows_the_password_in_the_answer_the_log_or_the_audit() {
    let (logs, _guard) = capture_logs();
    let (world, _machine) = leak_world();
    let before = world.audit_count();

    let reply = world
        .post(&world.engineer, SAVE_PATH, VALID_BODY, true)
        .await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(world.audit_count(), before + 1, "one entry for the save");
    assert_no_leak(&world, &reply, &logs, "save with HTMX");
    let plain = world
        .post(
            &world.admin,
            SAVE_PATH,
            &VALID_BODY.replace("row_version=0", "row_version=1"),
            false,
        )
        .await;
    assert_eq!(plain.status, StatusCode::SEE_OTHER, "{}", plain.body);
    assert_no_leak(&world, &plain, &logs, "save without HTMX");
}

#[tokio::test]
async fn the_pages_after_a_save_never_show_the_password() {
    let (logs, _guard) = capture_logs();
    let (world, _machine) = leak_world();
    let saved = world
        .post(&world.engineer, SAVE_PATH, VALID_BODY, true)
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);

    for path in [
        "/machines/M102",
        "/machines/M102/connection/edit",
        "/machines",
        "/audit",
    ] {
        let reply = world.get_en(&world.admin, path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
        assert_no_leak(&world, &reply, &logs, path);
    }
}

#[tokio::test]
async fn the_transfer_read_is_the_only_answer_with_the_password() {
    let (logs, _guard) = capture_logs();
    let (world, machine) = leak_world();
    let saved = world
        .post(&world.engineer, SAVE_PATH, VALID_BODY, true)
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    seed_recipe(&world, machine);

    let reply = transfer_read(&world).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains(&format!("\"password\": \"{PLC_PASSWORD}\"")),
        "{}",
        reply.body
    );
    let text = logs.text();
    assert!(!text.contains(PLC_PASSWORD), "log: {text}");
    assert_audit_clean(&world, "transfer read");
}

#[tokio::test]
async fn a_refused_transfer_read_never_shows_the_password() {
    let (logs, _guard) = capture_logs();
    let (world, machine) = leak_world();
    let saved = world
        .post(&world.engineer, SAVE_PATH, VALID_BODY, true)
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    seed_recipe(&world, machine);
    let wrong = format!("Basic {}", STANDARD.encode("user4:wrong-password-1"));

    let unknown_recipe = get_with(
        &world.fixture.router(),
        "/export/v1/machines/M102/connection?recipe=R-999",
        &[(
            "authorization",
            &format!(
                "Basic {}",
                STANDARD.encode(format!("user{CLIENT_NUMBER}:{CLIENT_PASSWORD}"))
            ),
        )],
    )
    .await;
    let bad_login = get_with(
        &world.fixture.router(),
        TRANSFER_PATH,
        &[("authorization", &wrong)],
    )
    .await;
    let no_header = get_with(&world.fixture.router(), TRANSFER_PATH, &[]).await;

    for (what, reply) in [
        ("unknown recipe", unknown_recipe),
        ("wrong account password", bad_login),
        ("no header", no_header),
    ] {
        assert_ne!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
        assert_no_leak(&world, &reply, &logs, what);
    }
}
