//! Tests of `POST .../units/{path}/rename` and `POST .../units/{path}/archive` (task 9.3, specs
//! "Unit page" and "Plant forms without user data in an hx attribute").

use axum::http::StatusCode;

use super::test_world::{seed_approved_use, world};
use crate::i18n::Lang;
use crate::pages::test_helpers::{assert_clean_html, assert_htmx_error};
use crate::test_support::plant::{ENGLISH, HTMX, cookie_of, main_of, text_with};
use crate::test_support::{icon_tag, post_fields, text};

/// The rename route of the unit `U1.2` of `M7`.
const RENAME: &str = "/machines/M7/units/U1.2/rename";
/// The archive route of the unit `U1.2` of `M7`.
const ARCHIVE: &str = "/machines/M7/units/U1.2/archive";

// --- Rename: success ----------------------------------------------------------------------------

#[tokio::test]
async fn units_rename_with_htmx_answers_the_new_name_the_toast_and_the_empty_error_panel() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let headers = [cookie_of(lang), HTMX];
        let reply = post_fields(
            &world.fixture.router(),
            RENAME,
            Some(&world.engineer),
            &headers,
            "name=Press+infeed",
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(
            reply.body.starts_with("Press infeed<div id=\"toast\""),
            "{}",
            reply.body
        );
        let toast = format!(
            "<div id=\"toast\" hx-swap-oob=\"innerHTML\"><div class=\"toast\">{}",
            icon_tag("alert-success")
        );
        assert!(reply.body.contains(&toast), "{lang}: {}", reply.body);
        let sentence = text_with(lang, "unit-renamed", "path", "U1.2");
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
async fn units_rename_escapes_the_new_name_in_the_fragment() {
    let world = world();
    let body = "name=%3Cb%3Ex%3C%2Fb%3E";
    let reply = world.post(&world.engineer, RENAME, body, true).await;
    assert!(
        reply.body.starts_with("&#60;b&#62;x&#60;/b&#62;"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("<b>x</b>"));
}

#[tokio::test]
async fn units_rename_without_htmx_redirects_to_the_unit_page_and_the_name_changes() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(&world.admin, RENAME, "name=Renamed", false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7/units/U1.2");
    assert_eq!(world.audit_count(), before.checked_add(1).unwrap());
    let page = world
        .get_en(&world.engineer, "/machines/M7/units/U1.2")
        .await;
    assert!(
        page.body.contains("<span id=\"unit-name\">Renamed</span>"),
        "{}",
        page.body
    );
}

// --- Rename: errors -----------------------------------------------------------------------------

#[tokio::test]
async fn units_rename_of_an_archived_unit_gives_409_e_plt_009() {
    let world = world();
    let reply = world
        .post(
            &world.engineer,
            "/machines/M31/units/U2/rename",
            "name=A",
            true,
        )
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-PLT-009");
}

#[tokio::test]
async fn units_rename_gives_the_code_of_each_bad_path_value() {
    let world = world();
    let cases = [
        (
            "/machines/M7/units/U1.9/rename",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/machines/M99/units/U1/rename",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/machines/M12/units/U1/rename",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/machines/X7/units/U1/rename",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/machines/M7/units/X/rename",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-017",
        ),
        (
            "/machines/M7/units/U1.2.4.5.6.7/rename",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-002",
        ),
    ];
    for (path, status, code) in cases {
        let reply = world.post(&world.engineer, path, "name=A", true).await;
        assert_htmx_error(&reply, status, code);
    }
}

#[tokio::test]
async fn units_rename_names_the_field_of_a_bad_name_and_changes_nothing() {
    let world = world();
    let before = world.audit_count();
    for name in ["+", "%01"] {
        let reply = world
            .post(&world.engineer, RENAME, &format!("name={name}"), true)
            .await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
        assert!(
            reply.body.contains(&text(Lang::En, "field-unit-name")),
            "{name}"
        );
        assert!(reply.body.contains("href=\"#unit_name\""), "{name}");
        assert!(
            reply.body.contains(&text(Lang::En, "reason-name")),
            "{name}"
        );
    }
    assert_eq!(world.audit_count(), before);
}

#[tokio::test]
async fn units_rename_rejects_a_missing_or_an_unknown_field_and_an_operator() {
    let world = world();
    for body in ["", "name=A&extra=1", "other=A"] {
        let reply = world.post(&world.engineer, RENAME, body, true).await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    }
    let before = world.audit_count();
    let reply = world.post(&world.operator, RENAME, "name=A", true).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"));
    assert_eq!(world.audit_count(), before);
}

// --- Archive: success ---------------------------------------------------------------------------

#[tokio::test]
async fn units_archive_redirects_to_the_machine_page_with_the_path_and_shows_the_message() {
    for htmx in [false, true] {
        let world = world();
        let reply = world.post(&world.engineer, ARCHIVE, "", htmx).await;
        assert_eq!(
            reply.status,
            StatusCode::SEE_OTHER,
            "{htmx}: {}",
            reply.body
        );
        assert_eq!(reply.header("location"), "/machines/M7?archived=U1.2");
    }
    let world = world();
    let reply = world.post(&world.engineer, ARCHIVE, "", false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    let page = world
        .get_en(&world.engineer, "/machines/M7?archived=U1.2")
        .await;
    assert!(
        page.body
            .contains("<span>Unit U1.2 and its sub-units are archived.</span>"),
        "{}",
        page.body
    );
}

#[tokio::test]
async fn units_archive_archives_the_sub_units_and_the_audit_log_holds_one_entry_for_each() {
    let world = world();
    let before = world.audit_count();
    let reply = world.post(&world.admin, ARCHIVE, "", false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(
        world.audit_count(),
        before.checked_add(16).unwrap(),
        "U1.2 and 15 below it"
    );
    let page = world
        .get_en(&world.engineer, "/machines/M7/units/U1.2")
        .await;
    assert!(
        page.body.contains("badge-retired"),
        "{}",
        main_of(&page.body)
    );
    let tree = world
        .get(&world.engineer, "/machines/M7/units", &[ENGLISH, HTMX])
        .await;
    assert!(!tree.body.contains("href=\"/machines/M7/units/U1.2\""));
    assert!(!tree.body.contains("href=\"/machines/M7/units/U1.2.4.5.6\""));
}

#[tokio::test]
async fn units_archive_accepts_the_token_field() {
    let world = world();
    let body = format!("csrf_token={}", world.engineer.csrf);
    let reply = world.post(&world.engineer, ARCHIVE, &body, false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
}

// --- Archive: errors ----------------------------------------------------------------------------

#[tokio::test]
async fn units_archive_of_the_last_main_unit_gives_409_e_plt_008() {
    let world = world();
    let reply = world
        .post(&world.engineer, "/machines/M31/units/U1/archive", "", true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-PLT-008");
}

#[tokio::test]
async fn units_archive_of_an_archived_unit_gives_409_e_plt_009() {
    let world = world();
    let reply = world
        .post(&world.engineer, "/machines/M31/units/U2/archive", "", true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-PLT-009");
}

#[tokio::test]
async fn units_archive_of_a_unit_that_a_draft_uses_gives_409_e_plt_012() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(&world.engineer, "/machines/M7/units/U1/archive", "", true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-PLT-012");
    assert!(reply.body.contains("R-100"), "{}", reply.body);
    assert_eq!(world.audit_count(), before);
}

#[tokio::test]
async fn units_archive_of_a_unit_that_an_approved_version_uses_gives_409_e_plt_014() {
    let world = world();
    seed_approved_use(&world.fixture, "U1.3");
    let reply = world
        .post(&world.engineer, "/machines/M7/units/U1.3/archive", "", true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-PLT-014");
}

#[tokio::test]
async fn units_archive_gives_the_code_of_each_bad_path_value() {
    let world = world();
    let cases = [
        (
            "/machines/M7/units/U1.9/archive",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/machines/M99/units/U1/archive",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/machines/X7/units/U1/archive",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/machines/M7/units/X/archive",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-017",
        ),
        (
            "/machines/M7/units/U1.2.4.5.6.7/archive",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-002",
        ),
    ];
    for (path, status, code) in cases {
        let reply = world.post(&world.engineer, path, "", true).await;
        assert_htmx_error(&reply, status, code);
    }
}

#[tokio::test]
async fn units_archive_rejects_an_unknown_field_and_an_operator_and_archives_nothing() {
    let world = world();
    let reply = world.post(&world.engineer, ARCHIVE, "extra=1", true).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    let before = world.audit_count();
    let reply = world.post(&world.operator, ARCHIVE, "", true).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"));
    assert_eq!(world.audit_count(), before);
}
