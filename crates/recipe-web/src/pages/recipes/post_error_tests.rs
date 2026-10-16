//! HTMX tests of the error codes of the `POST` routes of the recipe and version pages (design D16).
//!
//! Each answer goes into `#error-panel` and keeps the URL: `HX-Retarget`, `HX-Reswap`, and
//! `HX-Push-Url: false` (`api.md` section 5.3, ADR-0037 (z)). `POST /recipes` has no path value,
//! so it has no test of a malformed path.

use axum::http::StatusCode;
use recipe_domain::plant::MachineId;
use recipe_domain::recipe::VersionState;

use super::test_world::{recipe_world, seed_recipe_of};
use crate::pages::test_helpers::assert_htmx_error;
use crate::test_support::Reply;
use crate::test_support::plant::M12_ID;

/// Assert an HTMX error answer with `status`, `code`, the swap headers, and the kept URL.
fn assert_panel_error(reply: &Reply, status: StatusCode, code: &str) {
    assert_htmx_error(reply, status, code);
    assert_eq!(reply.header("hx-push-url"), "false", "{code}");
    assert_eq!(
        reply.header("hx-reswap"),
        "innerHTML show:#error-panel:top",
        "{code}"
    );
}

#[tokio::test]
async fn recipes_post_create_on_an_archived_machine_gives_e_plt_010() {
    let world = recipe_world();
    let body = "machine_code=M12&reference_number=R-510&name=Old&change_note=Note";
    let reply = world.post(&world.engineer, "/recipes", body, true).await;
    assert_panel_error(&reply, StatusCode::CONFLICT, "E-PLT-010");
}

#[tokio::test]
async fn recipes_post_create_errors_keep_the_url() {
    let world = recipe_world();
    let cases = [
        (
            "machine_code=M99&reference_number=R-511&name=N&change_note=C",
            StatusCode::NOT_FOUND,
            "E-PLT-019",
        ),
        (
            "machine_code=M7&reference_number=R-100&name=N&change_note=C",
            StatusCode::CONFLICT,
            "E-RCP-001",
        ),
        (
            "machine_code=&reference_number=R-512&name=N&change_note=C",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-WEB-008",
        ),
    ];
    for (body, status, code) in cases {
        let reply = world.post(&world.engineer, "/recipes", body, true).await;
        assert_panel_error(&reply, status, code);
    }
}

#[tokio::test]
async fn recipes_post_rename_of_a_malformed_or_unknown_reference_gives_e_web_005() {
    let world = recipe_world();
    for path in ["/recipes/-x/rename", "/recipes/R-999/rename"] {
        let reply = world.post(&world.engineer, path, "name=New", true).await;
        assert_panel_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005");
    }
    let deleted = world
        .post(&world.admin, "/recipes/R-300/rename", "name=New", true)
        .await;
    assert_panel_error(&deleted, StatusCode::CONFLICT, "E-RCP-012");
    let empty = world
        .post(&world.engineer, "/recipes/R-100/rename", "name=", true)
        .await;
    assert_panel_error(&empty, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
}

#[tokio::test]
async fn recipes_post_version_of_a_malformed_or_unknown_reference_gives_e_web_005() {
    let world = recipe_world();
    for path in ["/recipes/-x/versions", "/recipes/R-999/versions"] {
        let reply = world
            .post(&world.engineer, path, "based_on=&change_note=Note", true)
            .await;
        assert_panel_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005");
    }
}

#[tokio::test]
async fn recipes_post_version_of_a_deleted_recipe_gives_e_rcp_012() {
    let world = recipe_world();
    let body = "based_on=&change_note=Note";
    let reply = world
        .post(&world.engineer, "/recipes/R-300/versions", body, true)
        .await;
    assert_panel_error(&reply, StatusCode::CONFLICT, "E-RCP-012");
}

#[tokio::test]
async fn recipes_post_version_on_an_archived_machine_gives_e_plt_010() {
    let world = recipe_world();
    let machine = MachineId::from(M12_ID);
    seed_recipe_of(
        &world.fixture,
        machine,
        (800, "R-800"),
        &[VersionState::Approved],
    );
    let body = "based_on=&change_note=Note";
    let reply = world
        .post(&world.engineer, "/recipes/R-800/versions", body, true)
        .await;
    assert_panel_error(&reply, StatusCode::CONFLICT, "E-PLT-010");
}

#[tokio::test]
async fn recipes_post_version_errors_of_the_draft_and_the_base_keep_the_url() {
    let world = recipe_world();
    let draft = world
        .post(
            &world.engineer,
            "/recipes/R-100/versions",
            "based_on=2&change_note=N",
            true,
        )
        .await;
    assert_panel_error(&draft, StatusCode::CONFLICT, "E-RCP-011");
    let base = world
        .post(
            &world.engineer,
            "/recipes/R-200/versions",
            "based_on=x&change_note=N",
            true,
        )
        .await;
    assert_panel_error(&base, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
}
