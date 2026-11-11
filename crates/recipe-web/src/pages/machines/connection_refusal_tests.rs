//! Refusal tests of `POST /machines/{code}/connection` (task 7.6 of `plc-connection-config`,
//! ADR-0154, design D8, `api.md` section 5.1). The Operator gets `403 E-WEB-002`. Read-only mode
//! gives `503 E-WEB-006`. A stale `row_version` gives `409 E-WEB-013` with `error.html` in
//! `#error-panel`. A save on an archived machine gives `404 E-WEB-005`, because `api.md` section 2.2
//! resolves the route to the live machine only. No refusal writes a row or an audit entry.

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::connection_tests::{ads_record, seed};
use crate::i18n::Lang;
use crate::pages::test_helpers::{assert_error, assert_htmx_error, assert_refusal_headers};
use crate::test_support::plant::{HTMX, World, cookie_of, world};
use crate::test_support::{Fixture, post_fields};

/// The path of the connection route of the S7 machine `M7`.
const SAVE_PATH: &str = "/machines/M7/connection";
/// A valid save of `M7`.
const S7_BODY: &str = "row_version=0&host=10.0.0.5&rack=0&slot=2&db_number=1";
/// The stale-save sentence in English (`reliability.md` section 1.2, `E-WEB-013`).
const STALE_SENTENCE: &str = "Another user changed the connection settings of machine M7.";

/// Return the security events as text, one entry for each event.
fn event_texts(fixture: &Fixture) -> Vec<String> {
    fixture
        .sink
        .events()
        .iter()
        .map(|event| {
            format!(
                "{:?} {:?}",
                event.kind(),
                event.detail().map(ToString::to_string)
            )
        })
        .collect()
}

/// Return the text of the `forbidden` event that names the command `SetPlcConnection`.
fn forbidden_text() -> String {
    let detail = "{\"operation\":\"SetPlcConnection\"}".to_owned();
    format!("{:?} {:?}", SecurityEventKind::Forbidden, Some(detail))
}

/// Save `M7` once as the Engineer, so that the record has `row_version` 1.
async fn save_once(world: &World) {
    let first = world.post(&world.engineer, SAVE_PATH, S7_BODY, true).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
}

/// The path of the connection route of the archived machine `M3`.
const ARCHIVED_PATH: &str = "/machines/M3/connection";
/// A valid save of the `TwinCat3` machine `M3` that holds the record at `row_version` 1.
const ADS_BODY: &str = "row_version=1&host=10.0.0.6&port=48898&address=192.168.1.10.1.1&comm_mode=Ads&login=&password=";
/// The instant of the archive of `M3`, in Unix seconds.
const ARCHIVED_AT_SECONDS: i64 = 1_700_000_000;

/// Make the archived machine `M3` with a stored record. No live machine `M3` exists.
fn seed_archived_m3(world: &World) {
    let id = seed(world, 3, PlcPlatform::TwinCat3);
    world.fixture.store.seed_connection(id, &ads_record(), None);
    let mut machine = world.fixture.store.state().machine(id).unwrap();
    let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(ARCHIVED_AT_SECONDS).unwrap());
    machine.archive(at, None).unwrap();
    world.fixture.store.seed_machine(&machine);
}

// --- 403 E-WEB-002: the Operator -----------------------------------------------------------------

#[tokio::test]
async fn connection_save_by_an_operator_gives_403_with_one_forbidden_event_and_no_write() {
    for htmx in [true, false] {
        let world = world();
        let before = world.fixture.snapshot();
        let events = event_texts(&world.fixture);
        let reply = world.post(&world.operator, SAVE_PATH, S7_BODY, htmx).await;
        let what = format!("operator, htmx {htmx}");
        assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", &what);
        assert_refusal_headers(&reply, htmx, &what);
        let mut expected = events;
        expected.push(forbidden_text());
        assert_eq!(event_texts(&world.fixture), expected, "{what}");
        assert!(!reply.body.contains("<form"), "{what}: {}", reply.body);
        assert_eq!(world.fixture.snapshot(), before, "{what}");
    }
}

#[tokio::test]
async fn connection_save_by_an_operator_is_refused_before_the_body_is_read() {
    let world = world();
    let before = world.fixture.snapshot();
    // A body with an unknown key gives E-WEB-008 for a role with the permission.
    let body = format!("{S7_BODY}&extra=1");
    let reply = world.post(&world.operator, SAVE_PATH, &body, true).await;
    assert_htmx_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002");
    assert_eq!(world.fixture.snapshot(), before);
}

// --- 503 E-WEB-006: read-only mode ---------------------------------------------------------------

#[tokio::test]
async fn connection_save_in_read_only_mode_gives_503_and_writes_nothing() {
    for htmx in [true, false] {
        let world = world();
        world.fixture.state.read_only().set(true);
        let before = world.fixture.snapshot();
        for session in [&world.admin, &world.engineer] {
            let reply = world.post(session, SAVE_PATH, S7_BODY, htmx).await;
            let what = format!("read-only, htmx {htmx}");
            assert_error(&reply, StatusCode::SERVICE_UNAVAILABLE, "E-WEB-006", &what);
            assert_refusal_headers(&reply, htmx, &what);
            assert!(!reply.body.contains("<form"), "{what}: {}", reply.body);
            assert_eq!(world.fixture.snapshot(), before, "{what}");
        }
    }
}

#[tokio::test]
async fn connection_save_in_read_only_mode_never_reaches_the_use_case() {
    let world = world();
    world.fixture.state.read_only().set(true);
    // A stale token gives 409 when the use case runs.
    let body = S7_BODY.replace("row_version=0", "row_version=9");
    let reply = world.post(&world.engineer, SAVE_PATH, &body, true).await;
    assert_htmx_error(&reply, StatusCode::SERVICE_UNAVAILABLE, "E-WEB-006");
}

#[tokio::test]
async fn connection_form_page_stays_available_in_read_only_mode() {
    let world = world();
    world.fixture.state.read_only().set(true);
    let reply = world
        .get_en(&world.engineer, "/machines/M7/connection/edit")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

// --- 409 E-WEB-013: a stale row_version ----------------------------------------------------------

#[tokio::test]
async fn a_stale_save_gives_409_e_web_013_with_error_html_in_the_error_panel() {
    let world = world();
    save_once(&world).await;
    let before = world.fixture.snapshot();
    let reply = world.post(&world.admin, SAVE_PATH, S7_BODY, true).await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-WEB-013");
    assert_eq!(reply.header("hx-redirect"), "");
    assert_eq!(
        world.fixture.snapshot(),
        before,
        "a stale save writes nothing"
    );
    // error.html: the code, one sentence, and the request id. Never conflict.html.
    let code = "<span class=\"alert-code\">E-WEB-013</span>";
    assert!(reply.body.contains(code), "{}", reply.body);
    assert!(reply.body.contains(STALE_SENTENCE), "{}", reply.body);
    assert!(reply.body.contains("alert-ref"), "{}", reply.body);
    assert!(!reply.body.contains("field-lines"), "{}", reply.body);
    assert!(!reply.body.contains("target-link"), "{}", reply.body);
    assert!(!reply.body.contains("E-WEB-001"), "{}", reply.body);
}

#[tokio::test]
async fn a_stale_save_answers_no_form_and_no_row_version_so_the_input_of_the_user_stays() {
    let world = world();
    save_once(&world).await;
    let reply = world.post(&world.admin, SAVE_PATH, S7_BODY, true).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(!reply.body.contains("<form"), "{}", reply.body);
    assert!(
        !reply.body.contains("hx-swap-oob=\"outerHTML\""),
        "{}",
        reply.body
    );
    assert_eq!(reply.header("hx-trigger"), "", "no rowVersion event");
}

#[tokio::test]
async fn a_save_with_a_token_ahead_of_the_record_also_gives_409_e_web_013() {
    let world = world();
    let before = world.fixture.snapshot();
    for token in ["1", "7"] {
        let body = S7_BODY.replace("row_version=0", &format!("row_version={token}"));
        let reply = world.post(&world.engineer, SAVE_PATH, &body, true).await;
        assert_htmx_error(&reply, StatusCode::CONFLICT, "E-WEB-013");
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn a_stale_save_without_htmx_gives_409_with_the_error_page_and_no_hx_headers() {
    let world = world();
    save_once(&world).await;
    let reply = world.post(&world.admin, SAVE_PATH, S7_BODY, false).await;
    assert_error(
        &reply,
        StatusCode::CONFLICT,
        "E-WEB-013",
        "plain stale save",
    );
    assert_refusal_headers(&reply, false, "plain stale save");
    assert!(reply.body.contains(STALE_SENTENCE), "{}", reply.body);
}

#[tokio::test]
async fn a_stale_save_names_the_machine_in_both_languages_and_logs_no_security_event() {
    let world = world();
    save_once(&world).await;
    let events = event_texts(&world.fixture);
    for lang in [Lang::En, Lang::Pl] {
        let headers = [cookie_of(lang), HTMX];
        let reply = post_fields(
            &world.fixture.router(),
            SAVE_PATH,
            Some(&world.admin),
            &headers,
            S7_BODY,
        )
        .await;
        assert_eq!(reply.status, StatusCode::CONFLICT, "{lang}: {}", reply.body);
        assert!(reply.body.contains("M7"), "{lang}: {}", reply.body);
        assert!(reply.body.contains("E-WEB-013"), "{lang}: {}", reply.body);
    }
    assert_eq!(event_texts(&world.fixture), events);
}

// --- 404 E-WEB-005: an archived machine ----------------------------------------------------------

#[tokio::test]
async fn connection_save_on_an_archived_machine_gives_404_e_web_005_and_keeps_the_record() {
    let world = world();
    seed_archived_m3(&world);
    let before = world.fixture.snapshot();
    assert!(
        before.0.contains("ConnectionRow"),
        "the fixture stores a record"
    );
    let reply = world
        .post(&world.engineer, ARCHIVED_PATH, ADS_BODY, true)
        .await;
    assert_error(
        &reply,
        StatusCode::NOT_FOUND,
        "E-WEB-005",
        "save on an archived machine",
    );
    assert_eq!(world.fixture.snapshot(), before);
}
