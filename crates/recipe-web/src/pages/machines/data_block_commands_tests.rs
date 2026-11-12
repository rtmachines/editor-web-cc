//! Tests of `POST /machines/{code}/data-block` (task 6.3, specs "Data block form of a machine" and
//! "Change of the data block number", design D6 and D8). Task 6.4 adds the unknown field, CSRF, and
//! read-only tests. The XSS test is in `plant_xss_tests.rs`.

use axum::http::StatusCode;
use recipe_app::memory::{Fault, FaultPoint, JournalEntry};
use recipe_app::ports::Isolation;

use super::test_world::seed_machine_with_archived_unit;
use crate::i18n::Lang;
use crate::pages::draft_reply::refusal_tests::assert_operator_refused;
use crate::pages::test_helpers::{
    assert_clean_html, assert_htmx_error, field_value, summary_lines,
};
use crate::test_support::plant::{HTMX, World, cookie_of, text_with, world};
use crate::test_support::{TestSession, icon_tag, post_fields};

/// The opening tag of the data block form of `M7` when an answer swaps it out of band.
const OOB_FORM_TAG: &str = "<form id=\"machine-data-block-form\" method=\"post\" action=\"/machines/M7/data-block\" hx-boost=\"true\" hx-target=\"#machine-db-number\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";

#[tokio::test]
async fn machines_data_block_with_htmx_answers_the_number_the_toast_and_the_empty_error_panel() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let headers = [cookie_of(lang), HTMX];
        let reply = post_fields(
            &world.fixture.router(),
            "/machines/M7/data-block",
            Some(&world.engineer),
            &headers,
            "db_number=42",
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(
            reply.body.starts_with("42<div id=\"toast\""),
            "{}",
            reply.body
        );
        let toast = format!(
            "<div id=\"toast\" hx-swap-oob=\"innerHTML\"><div class=\"toast\">{}",
            icon_tag("alert-success")
        );
        assert!(reply.body.contains(&toast), "{lang}: {}", reply.body);
        let sentence = text_with(lang, "machine-data-block-set", "code", "M7");
        assert!(
            reply.body.contains(&format!("<span>{sentence}</span>")),
            "{lang}: {}",
            reply.body
        );
        assert!(
            reply
                .body
                .contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>"),
            "{lang}: {}",
            reply.body
        );
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn machines_data_block_by_htmx_swaps_the_clean_form_with_the_saved_number() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(
            &world.admin,
            "/machines/M7/data-block",
            "db_number=42",
            true,
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(form.contains("value=\"42\""), "{form}");
    assert!(form.contains("id=\"machine-data-block-submit\""), "{form}");
    for mark in ["aria-invalid", "field-error"] {
        assert!(!form.contains(mark), "{mark}: {form}");
    }
    assert_eq!(world.audit_count(), before.checked_add(1).unwrap());
    let page = world.get_en(&world.engineer, "/machines/M7").await;
    assert!(
        page.body
            .contains("<span id=\"machine-db-number\">42</span>"),
        "{}",
        page.body
    );
}

#[tokio::test]
async fn machines_data_block_without_htmx_redirects_to_the_machine_page() {
    let world = world();
    let reply = world
        .post(
            &world.engineer,
            "/machines/M7/data-block",
            "db_number=7",
            false,
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7");
}

#[tokio::test]
async fn machines_data_block_out_of_range_gives_422_e_plt_024_with_the_form_and_the_line() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(
            &world.engineer,
            "/machines/M7/data-block",
            "db_number=16001",
            true,
        )
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-024");
    assert_eq!(reply.header("hx-push-url"), "false");
    let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(form.contains("value=\"16001\""), "{form}");
    assert!(form.contains("aria-invalid=\"true\""), "{form}");
    assert!(
        form.contains("<div class=\"field-error\" id=\"db_number-error\">"),
        "{form}"
    );
    assert_eq!(world.audit_count(), before);
}

#[tokio::test]
async fn machines_data_block_empty_on_an_s7_machine_gives_422_e_plt_050_with_the_line() {
    let world = world();
    let reply = world
        .post(
            &world.engineer,
            "/machines/M7/data-block",
            "db_number=",
            true,
        )
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-050");
    let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(
        form.contains("<div class=\"field-error\" id=\"db_number-error\">"),
        "{form}"
    );
}

#[tokio::test]
async fn machines_data_block_on_an_iec_machine_gives_422_e_plt_051() {
    let world = world();
    seed_machine_with_archived_unit(&world.fixture);
    let reply = world
        .post(
            &world.engineer,
            "/machines/M20/data-block",
            "db_number=5",
            true,
        )
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-051");
}

#[tokio::test]
async fn machines_data_block_with_a_text_that_is_no_number_gives_one_e_web_008_line() {
    let world = world();
    let before = world.audit_count();
    for sent in ["0", "abc", "65536", "1.5"] {
        let body = format!("db_number={sent}");
        let reply = world
            .post(&world.engineer, "/machines/M7/data-block", &body, true)
            .await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
        assert!(reply.body.contains("Errors: 1."), "{}", reply.body);
        let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
        assert!(form.contains(&format!("value=\"{sent}\"")), "{form}");
        assert!(
            form.contains("<div class=\"field-error\" id=\"db_number-error\">"),
            "{form}"
        );
    }
    assert_eq!(world.audit_count(), before);
}

#[tokio::test]
async fn machines_data_block_of_an_unknown_or_archived_machine_gives_404_e_web_005() {
    let world = world();
    for code in ["M99", "M12"] {
        let path = format!("/machines/{code}/data-block");
        let reply = world
            .post(&world.engineer, &path, "db_number=5", true)
            .await;
        assert_htmx_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005");
    }
}

#[tokio::test]
async fn machines_data_block_of_an_operator_gives_e_web_002_and_no_form() {
    let world = world();
    for body in ["db_number=5", "db_number=0", "db_number="] {
        assert_operator_refused(&world, "/machines/M7/data-block", body).await;
    }
}

/// ADR-0138: a changed form body gives one `E-WEB-008` line and no out-of-band form.
#[tokio::test]
async fn machines_data_block_with_an_unknown_missing_or_repeated_field_gives_one_e_web_008_line() {
    let world = world();
    let before = world.fixture.snapshot();
    for body in [
        "db_number=5&extra=1",
        "other=5",
        "",
        "db_number=5&db_number=6",
        "db_number[0]=5",
        "db_number=5&row_version=1",
    ] {
        let reply = world
            .post(&world.engineer, "/machines/M7/data-block", body, true)
            .await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
        assert_eq!(summary_lines(&reply.body), 1, "{body}: {}", reply.body);
        assert!(!reply.body.contains("<form"), "{body}: {}", reply.body);
        assert!(
            !reply.body.contains("hx-swap-oob=\"true\""),
            "{body}: {}",
            reply.body
        );
        assert_eq!(world.fixture.snapshot(), before, "{body}");
    }
}

#[tokio::test]
async fn machines_data_block_with_a_bad_csrf_token_gives_403_e_web_007_and_writes_nothing() {
    let world = world();
    let before = world.fixture.snapshot();
    let forged = TestSession {
        cookie: world.engineer.cookie.clone(),
        csrf: "0".repeat(world.engineer.csrf.len()),
    };
    for htmx in [true, false] {
        let reply = world
            .post(&forged, "/machines/M7/data-block", "db_number=42", htmx)
            .await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
        assert!(reply.body.contains("E-WEB-007"), "{}", reply.body);
        assert!(!reply.body.contains("<form"), "{}", reply.body);
        assert_eq!(world.fixture.snapshot(), before);
    }
}

#[tokio::test]
async fn machines_data_block_in_read_only_mode_gives_503_e_web_006_and_writes_nothing() {
    let world = world();
    world.fixture.state.read_only().set(true);
    let before = world.fixture.snapshot();
    for htmx in [true, false] {
        let reply = world
            .post(
                &world.engineer,
                "/machines/M7/data-block",
                "db_number=42",
                htmx,
            )
            .await;
        assert_eq!(
            reply.status,
            StatusCode::SERVICE_UNAVAILABLE,
            "{}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-006"), "{}", reply.body);
        assert!(!reply.body.contains("<form"), "{}", reply.body);
        assert_eq!(world.fixture.snapshot(), before);
    }
}

#[tokio::test]
async fn machines_data_block_answers_carry_the_hint_of_the_platform() {
    let world = world();
    let refused = world
        .post(
            &world.engineer,
            "/machines/M7/data-block",
            "db_number=16001",
            true,
        )
        .await;
    let form = refused.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(form.contains("1 to 16000"), "{form}");
    assert!(!form.contains("59999"), "{form}");
    assert!(!form.contains("empty"), "{form}");
    let not_a_number = world
        .post(
            &world.engineer,
            "/machines/M7/data-block",
            "db_number=abc",
            true,
        )
        .await;
    let form = not_a_number.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(form.contains("1 to 16000"), "{form}");
    let saved = world
        .post(
            &world.engineer,
            "/machines/M7/data-block",
            "db_number=42",
            true,
        )
        .await;
    let form = saved.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(form.contains("1 to 16000"), "{form}");
}

/// The number of read-only transactions that the journal holds.
fn read_only_begins(world: &World) -> usize {
    let only_reads =
        |entry: &&JournalEntry| matches!(entry, JournalEntry::Begin(Isolation::ReadOnly));
    world
        .fixture
        .store
        .journal()
        .iter()
        .filter(only_reads)
        .count()
}

#[tokio::test]
async fn machines_data_block_refusal_and_save_of_the_command_open_no_second_read() {
    let world = world();
    for (body, code) in [
        ("db_number=16001", Some("E-PLT-024")),
        ("db_number=", Some("E-PLT-050")),
        ("db_number=42", None),
    ] {
        let before = read_only_begins(&world);
        let reply = world
            .post(&world.engineer, "/machines/M7/data-block", body, true)
            .await;
        if let Some(code) = code {
            assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, code);
        }
        let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
        assert!(form.contains("1 to 16000"), "{body}: {form}");
        assert_eq!(read_only_begins(&world) - before, 1, "{body}");
    }
}

#[tokio::test]
async fn machines_data_block_with_a_bad_text_and_a_failing_read_gives_the_storage_error() {
    let world = world();
    world
        .fixture
        .store
        .fail_next(FaultPoint::Begin, Fault::StatementTimeout, 1);
    let reply = world
        .post(
            &world.engineer,
            "/machines/M7/data-block",
            "db_number=abc",
            true,
        )
        .await;
    assert!(reply.status.is_server_error(), "{}", reply.body);
    assert!(reply.body.contains("E-DB-"), "{}", reply.body);
    assert!(!reply.body.contains(OOB_FORM_TAG), "{}", reply.body);
}

/// Asserts that the connection form shows the saved values and the bumped `row_version`.
async fn assert_form_reloaded_with_saved_values(world: &World) {
    let page = world
        .get_en(&world.engineer, "/machines/M7/connection/edit")
        .await;
    assert_eq!(field_value(&page.body, "row_version"), "2");
    let page_form = crate::test_support::plant::main_of(&page.body);
    assert!(page_form.contains(" value=\"10.0.0.5\""), "{page_form}");
    assert!(!page_form.contains("10.0.0.9"), "{page_form}");
}

/// Design D6: the data block command copies the number into the connection record and bumps its
/// `row_version`. A connection form that was loaded before the command gets `409 E-WEB-013`.
#[tokio::test]
async fn connection_form_loaded_before_a_data_block_change_gets_409_e_web_013_on_save() {
    let world = world();
    let saved = world
        .post(
            &world.engineer,
            "/machines/M7/connection",
            "row_version=0&host=10.0.0.5&rack=0&slot=2",
            true,
        )
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    let form = world
        .get_en(&world.engineer, "/machines/M7/connection/edit")
        .await;
    let loaded = field_value(&form.body, "row_version");
    assert_eq!(loaded, "1");

    let changed = world
        .post(
            &world.admin,
            "/machines/M7/data-block",
            "db_number=42",
            true,
        )
        .await;
    assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
    let after_change = world.fixture.snapshot();

    let stale_body = format!("row_version={loaded}&host=10.0.0.9&rack=1&slot=3");
    let stale = world
        .post(
            &world.engineer,
            "/machines/M7/connection",
            &stale_body,
            true,
        )
        .await;
    assert_htmx_error(&stale, StatusCode::CONFLICT, "E-WEB-013");
    assert!(stale.body.contains("E-WEB-013"), "{}", stale.body);
    assert!(!stale.body.contains("<form"), "{}", stale.body);
    assert_eq!(
        world.fixture.snapshot(),
        after_change,
        "the stale save writes nothing"
    );

    assert_form_reloaded_with_saved_values(&world).await;
}
