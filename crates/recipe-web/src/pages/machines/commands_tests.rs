//! Tests of `POST /machines/{code}/rename` and `POST /machines/{code}/archive` (task 9.4, spec
//! "Rename and archive of a machine").

use axum::http::StatusCode;

use super::test_world::seed_machines;
use crate::i18n::Lang;
use crate::pages::draft_reply::refusal_tests::assert_operator_refused;
use crate::pages::test_helpers::{assert_clean_html, assert_htmx_error};
use crate::test_support::plant::{HTMX, cookie_of, text_with, world};
use crate::test_support::{icon_tag, post_fields, text};

/// The opening tag of the rename form when an answer swaps it out of band (ADR-0041 (e)).
const OOB_RENAME_TAG: &str = "<form id=\"machine-rename-form\" method=\"post\" action=\"/machines/M7/rename\" hx-boost=\"true\" hx-target=\"#machine-name\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";

// --- Rename: success ----------------------------------------------------------------------------

#[tokio::test]
async fn machines_rename_with_htmx_answers_the_new_name_the_toast_and_the_empty_error_panel() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let headers = [cookie_of(lang), HTMX];
        let reply = post_fields(
            &world.fixture.router(),
            "/machines/M7/rename",
            Some(&world.engineer),
            &headers,
            "name=Press+line+2",
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(
            reply.body.starts_with("Press line 2<div id=\"toast\""),
            "{}",
            reply.body
        );
        let toast = format!(
            "<div id=\"toast\" hx-swap-oob=\"innerHTML\"><div class=\"toast\">{}",
            icon_tag("alert-success")
        );
        assert!(reply.body.contains(&toast), "{lang}: {}", reply.body);
        let sentence = text_with(lang, "machine-renamed", "code", "M7");
        assert!(
            reply.body.contains(&format!("<span>{sentence}</span>")),
            "{lang}"
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
async fn machines_rename_escapes_the_new_name_in_the_fragment() {
    let world = world();
    let reply = world
        .post(
            &world.engineer,
            "/machines/M7/rename",
            "name=%3Cb%3Ex%3C%2Fb%3E",
            true,
        )
        .await;
    assert!(
        reply.body.starts_with("&#60;b&#62;x&#60;/b&#62;"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("<b>x</b>"));
}

#[tokio::test]
async fn machines_rename_without_htmx_redirects_to_the_machine_page_and_the_name_changes() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(&world.admin, "/machines/M7/rename", "name=Renamed", false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7");
    assert_eq!(world.audit_count(), before.checked_add(1).unwrap());
    let page = world.get_en(&world.engineer, "/machines/M7").await;
    assert!(
        page.body
            .contains("<span id=\"machine-name\">Renamed</span>"),
        "{}",
        page.body
    );
}

/// ADR-0041 (h): a success swaps the rename form out of band with the saved name and no line.
#[tokio::test]
async fn machines_rename_by_htmx_swaps_the_clean_form_with_the_saved_name() {
    let world = world();
    let reply = world
        .post(
            &world.engineer,
            "/machines/M7/rename",
            "name=Press+line+2",
            true,
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = reply.body.split(OOB_RENAME_TAG).nth(1).unwrap();
    assert!(form.contains("value=\"Press line 2\""), "{form}");
    assert!(form.contains("id=\"machine-rename-submit\""), "{form}");
    for mark in ["aria-invalid", "field-error"] {
        assert!(!form.contains(mark), "{mark}: {form}");
    }
    assert_clean_html(&reply.body);
}

/// Spec "Every field error at once": a bad name swaps the form with the sent text and its line.
#[tokio::test]
async fn machines_rename_with_a_bad_name_swaps_the_form_with_the_sent_text_and_a_line() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(&world.engineer, "/machines/M7/rename", "name=+", true)
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(reply.header("hx-push-url"), "false");
    assert!(reply.body.contains("Errors: 1."), "{}", reply.body);
    let form = reply.body.split(OOB_RENAME_TAG).nth(1).unwrap();
    assert!(form.contains("value=\" \""), "{form}");
    assert!(form.contains("aria-invalid=\"true\""), "{form}");
    assert!(
        form.contains("<div class=\"field-error\" id=\"machine_name-error\">"),
        "{form}"
    );
    assert_eq!(world.audit_count(), before);
}

// --- Rename: errors -----------------------------------------------------------------------------

#[tokio::test]
async fn machines_rename_of_an_unknown_archived_or_malformed_code_gives_404_e_web_005() {
    let world = world();
    for code in ["M99", "M12", "X7"] {
        let path = format!("/machines/{code}/rename");
        let reply = world.post(&world.engineer, &path, "name=A", true).await;
        assert_htmx_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005");
    }
}

#[tokio::test]
async fn machines_rename_names_the_field_of_a_bad_name_and_changes_nothing() {
    let world = world();
    let before = world.audit_count();
    for name in ["+", "%01"] {
        let body = format!("name={name}");
        let reply = world
            .post(&world.engineer, "/machines/M7/rename", &body, true)
            .await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
        assert!(
            reply.body.contains(&text(Lang::En, "field-machine-name")),
            "{name}"
        );
        assert!(reply.body.contains("href=\"#machine_name\""), "{name}");
        assert!(
            reply.body.contains(&text(Lang::En, "reason-name")),
            "{name}"
        );
    }
    assert_eq!(world.audit_count(), before);
}

#[tokio::test]
async fn machines_rename_rejects_a_missing_or_an_unknown_field_and_an_operator() {
    let world = world();
    for body in ["", "name=A&extra=1", "other=A"] {
        let reply = world
            .post(&world.engineer, "/machines/M7/rename", body, true)
            .await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    }
    let before = world.audit_count();
    let reply = world
        .post(&world.operator, "/machines/M7/rename", "name=A", true)
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"));
    assert_eq!(world.audit_count(), before);
}

/// ADR-0013: the form query checks the permission before the field check, so a bad field of an
/// Operator gets `403 E-WEB-002` and no form.
#[tokio::test]
async fn machines_rename_of_an_operator_with_a_bad_field_gives_e_web_002_and_no_form() {
    let world = world();
    for body in ["name=+", "name=A&extra=1"] {
        assert_operator_refused(&world, "/machines/M7/rename", body).await;
    }
}

// --- Archive: success ---------------------------------------------------------------------------

#[tokio::test]
async fn machines_archive_redirects_to_the_list_with_the_code_with_and_without_htmx() {
    for htmx in [false, true] {
        let world = world();
        seed_machines(&world.fixture, 1, "Spare");
        let before = world.audit_count();
        let reply = world
            .post(&world.engineer, "/machines/M101/archive", "", htmx)
            .await;
        assert_eq!(
            reply.status,
            StatusCode::SEE_OTHER,
            "{htmx}: {}",
            reply.body
        );
        assert_eq!(reply.header("location"), "/machines?archived=M101");
        assert_eq!(world.audit_count(), before.checked_add(1).unwrap());
    }
}

#[tokio::test]
async fn machines_archive_shows_the_message_on_the_list_and_the_machine_leaves_the_live_routes() {
    let world = world();
    seed_machines(&world.fixture, 1, "Spare");
    let reply = world
        .post(&world.engineer, "/machines/M101/archive", "", false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    let list = world
        .get_en(&world.engineer, "/machines?archived=M101")
        .await;
    assert!(
        list.body.contains("<span>Machine M101 is archived.</span>"),
        "{}",
        list.body
    );
    let gone = world.get_en(&world.engineer, "/machines/M101").await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn machines_archive_accepts_the_token_field() {
    let world = world();
    seed_machines(&world.fixture, 1, "Spare");
    let body = format!("csrf_token={}", world.admin.csrf);
    let reply = world
        .post(&world.admin, "/machines/M101/archive", &body, false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
}

// --- Archive: errors ----------------------------------------------------------------------------

#[tokio::test]
async fn machines_archive_with_a_live_draft_gives_409_e_plt_011_in_the_error_panel() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(&world.engineer, "/machines/M7/archive", "", true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-PLT-011");
    assert!(reply.body.contains("R-100"), "{}", reply.body);
    assert_eq!(world.audit_count(), before);
    let page = world.get_en(&world.engineer, "/machines/M7").await;
    assert_eq!(page.status, StatusCode::OK, "the machine stays live");
}

#[tokio::test]
async fn machines_archive_of_an_unknown_archived_or_malformed_code_gives_404_e_web_005() {
    let world = world();
    for code in ["M99", "M12", "X7"] {
        let path = format!("/machines/{code}/archive");
        let reply = world.post(&world.engineer, &path, "", true).await;
        assert_htmx_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005");
    }
}

#[tokio::test]
async fn machines_archive_rejects_an_unknown_field_and_an_operator_and_archives_nothing() {
    let world = world();
    seed_machines(&world.fixture, 1, "Spare");
    let reply = world
        .post(&world.engineer, "/machines/M101/archive", "extra=1", true)
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    let before = world.audit_count();
    let reply = world
        .post(&world.operator, "/machines/M101/archive", "", true)
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"));
    assert_eq!(world.audit_count(), before);
}
