//! Tests of `POST /recipes/{ref}/delete` and `POST /recipes/{ref}/restore` and of their forms
//! (task 11.1, ADR-0049 (g) to (j), `api.md` section 2.4, design D9 and D10).

use axum::http::StatusCode;
use recipe_domain::recipe::VersionState;

use super::test_world::{recipe_world, seed_recipe_of_m20};
use crate::pages::test_helpers::{assert_clean_html, assert_error, assert_htmx_error};
use crate::test_support::plant::{ENGLISH, HTMX, World, main_of};
use crate::test_support::{Fixture, post_fields};

/// The opening tag of the delete form of `R-100`.
const DELETE_FORM_TAG: &str = "<form method=\"post\" action=\"/recipes/R-100/delete\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";
/// The opening tag of the restore form of `R-300`.
const RESTORE_FORM_TAG: &str = "<form method=\"post\" action=\"/recipes/R-300/restore\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";

/// Seed the recipe `R-401` of `M20` with one retired version and no other version.
fn retired_only(world: &World) {
    seed_recipe_of_m20(&world.fixture, (401, "R-401"), &[VersionState::Retired]);
}

#[tokio::test]
async fn lifecycle_recipe_page_shows_the_delete_form_in_a_danger_disclosure_to_admin_and_engineer()
{
    let world = recipe_world();
    for session in [&world.admin, &world.engineer] {
        let reply = world.get_en(session, "/recipes/R-100").await;
        let main = main_of(&reply.body);
        assert_eq!(
            main.matches("<details class=\"danger\">").count(),
            1,
            "{main}"
        );
        let (_before, rest) = main.split_once(DELETE_FORM_TAG).unwrap();
        let form = rest.split("</form>").next().unwrap();
        assert!(form.contains("name=\"csrf_token\""), "{form}");
        assert!(!form.contains("row_version"), "{form}");
        assert!(form.contains("id=\"danger-notice\""), "{form}");
        assert!(
            form.contains("aria-describedby=\"danger-notice\">Delete recipe R-100</button>"),
            "{form}"
        );
        assert!(form.contains("class=\"htmx-indicator\">The server processes the request."));
        assert!(main.contains("<span>Delete this recipe</span>"), "{main}");
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn lifecycle_recipe_page_shows_no_delete_form_to_an_operator_or_for_a_deleted_recipe() {
    let world = recipe_world();
    let operator = world.get_en(&world.operator, "/recipes/R-100").await;
    assert!(
        !operator.body.contains("/recipes/R-100/delete"),
        "{}",
        operator.body
    );
    let deleted = world.get_en(&world.admin, "/recipes/R-300").await;
    assert!(!deleted.body.contains("/delete"), "{}", deleted.body);
}

#[tokio::test]
async fn lifecycle_delete_of_a_recipe_without_an_approved_version_gives_303_to_the_list() {
    let world = recipe_world();
    let before = world.audit_count();
    let reply = world
        .post(&world.engineer, "/recipes/R-200/delete", "", false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes");
    assert!(world.audit_count() > before);
    let list = world.get_en(&world.admin, "/recipes").await;
    assert!(
        !list.body.contains("href=\"/recipes/R-200\""),
        "{}",
        list.body
    );
    let second = world
        .post(&world.admin, "/recipes/R-200/delete", "", false)
        .await;
    assert_error(&second, StatusCode::CONFLICT, "E-RCP-012", "second delete");
}

#[tokio::test]
async fn lifecycle_delete_codes_follow_the_role_and_the_versions() {
    let world = recipe_world();
    retired_only(&world);
    let approved = world
        .post(&world.engineer, "/recipes/R-100/delete", "", false)
        .await;
    assert_error(
        &approved,
        StatusCode::CONFLICT,
        "E-RCP-010",
        "engineer, approved",
    );
    let retired = world
        .post(&world.engineer, "/recipes/R-401/delete", "", false)
        .await;
    assert_error(
        &retired,
        StatusCode::CONFLICT,
        "E-RCP-026",
        "engineer, retired",
    );
    let admin_approved = world
        .post(&world.admin, "/recipes/R-100/delete", "", false)
        .await;
    assert_error(
        &admin_approved,
        StatusCode::CONFLICT,
        "E-RCP-010",
        "admin, approved",
    );
    let admin_retired = world
        .post(&world.admin, "/recipes/R-401/delete", "", false)
        .await;
    assert_eq!(
        admin_retired.status,
        StatusCode::SEE_OTHER,
        "{}",
        admin_retired.body
    );
    assert_eq!(admin_retired.header("location"), "/recipes");
}

#[tokio::test]
async fn lifecycle_delete_errors_by_htmx_retarget_the_error_panel_and_keep_the_url() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let conflict = world
        .post(&world.admin, "/recipes/R-100/delete", "", true)
        .await;
    assert_htmx_error(&conflict, StatusCode::CONFLICT, "E-RCP-010");
    let operator = world
        .post(&world.operator, "/recipes/R-100/delete", "", true)
        .await;
    assert_htmx_error(&operator, StatusCode::FORBIDDEN, "E-WEB-002");
    let unknown = world
        .post(&world.admin, "/recipes/R-999/delete", "", true)
        .await;
    assert_htmx_error(&unknown, StatusCode::NOT_FOUND, "E-WEB-005");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn lifecycle_delete_of_a_recipe_with_only_retired_versions_by_htmx_gives_e_rcp_026() {
    let world = recipe_world();
    retired_only(&world);
    let before = world.fixture.snapshot();
    let reply = world
        .post(&world.engineer, "/recipes/R-401/delete", "", true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-RCP-026");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn lifecycle_delete_of_a_deleted_recipe_gives_e_rcp_012() {
    let world = recipe_world();
    let reply = world
        .post(&world.admin, "/recipes/R-300/delete", "", true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-RCP-012");
}

#[tokio::test]
async fn lifecycle_list_with_show_deleted_holds_a_restore_form_in_each_deleted_row() {
    let world = recipe_world();
    let reply = world.get_en(&world.admin, "/recipes?show_deleted=1").await;
    let main = main_of(&reply.body);
    let (_before, rest) = main.split_once(RESTORE_FORM_TAG).unwrap();
    let form = rest.split("</form>").next().unwrap();
    assert!(form.contains("name=\"csrf_token\""), "{form}");
    assert!(
        form.contains("class=\"btn btn-secondary\">Restore R-300</button>"),
        "{form}"
    );
    assert!(form.contains("class=\"htmx-indicator\">The server processes the request."));
    assert!(!form.contains("btn-danger"), "{form}");
    assert_eq!(main.matches("/restore\"").count(), 1, "{main}");
    assert!(main.contains("<th scope=\"col\">Actions</th>"), "{main}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn lifecycle_list_restore_form_stands_inside_the_actions_container_of_its_cell() {
    let world = recipe_world();
    let reply = world.get_en(&world.admin, "/recipes?show_deleted=1").await;
    let main = main_of(&reply.body);
    let (before, rest) = main.split_once(RESTORE_FORM_TAG).unwrap();
    assert!(before.ends_with("<td><div class=\"actions\">"), "{before}");
    let (_form, after) = rest.split_once("</form>").unwrap();
    assert!(after.starts_with("</div></td>"), "{after}");
}

/// A disabled button loses the focus. So the delete form and the restore form have no
/// `hx-disabled-elt` (ADR-0049 (j), `design-system.md` section 10.6, QA-1).
#[tokio::test]
async fn lifecycle_delete_and_restore_forms_have_no_hx_disabled_elt() {
    let world = recipe_world();
    let page = world.get_en(&world.admin, "/recipes/R-100").await;
    let main = main_of(&page.body);
    assert!(main.contains("/recipes/R-100/delete"), "{main}");
    assert!(!main.contains("hx-disabled-elt"), "{main}");
    let list = world.get_en(&world.admin, "/recipes?show_deleted=1").await;
    let main = main_of(&list.body);
    assert!(main.contains("/recipes/R-300/restore"), "{main}");
    assert!(!main.contains("hx-disabled-elt"), "{main}");
}

#[tokio::test]
async fn lifecycle_list_without_show_deleted_or_for_other_roles_holds_no_restore_form() {
    let world = recipe_world();
    let plain = world.get_en(&world.admin, "/recipes").await;
    assert!(!plain.body.contains("/restore"), "{}", plain.body);
    assert!(!plain.body.contains("Actions"), "{}", plain.body);
    let engineer = world
        .get_en(&world.engineer, "/recipes?show_deleted=1")
        .await;
    assert!(!engineer.body.contains("/restore"), "{}", engineer.body);
}

#[tokio::test]
async fn lifecycle_restore_gives_303_to_the_recipe_and_the_recipe_is_live_again() {
    let world = recipe_world();
    let reply = world
        .post(&world.admin, "/recipes/R-300/restore", "", false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-300");
    let page = world.get_en(&world.operator, "/recipes/R-300").await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
}

#[tokio::test]
async fn lifecycle_restore_codes_for_a_live_recipe_and_for_the_other_roles() {
    let world = recipe_world();
    let live = world
        .post(&world.admin, "/recipes/R-100/restore", "", true)
        .await;
    assert_htmx_error(&live, StatusCode::CONFLICT, "E-RCP-025");
    for session in [&world.engineer, &world.operator] {
        let reply = world
            .post(session, "/recipes/R-300/restore", "", true)
            .await;
        assert_htmx_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002");
    }
    let unknown = world
        .post(&world.admin, "/recipes/R-999/restore", "", true)
        .await;
    assert_htmx_error(&unknown, StatusCode::NOT_FOUND, "E-WEB-005");
}

#[tokio::test]
async fn lifecycle_restore_on_an_archived_machine_gives_e_plt_010_and_writes_nothing() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let before = fixture.snapshot();
    let reply = post_fields(
        &fixture.router(),
        "/recipes/R-700/restore",
        Some(&data.admin),
        &[ENGLISH, HTMX],
        "",
    )
    .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-PLT-010");
    assert_eq!(fixture.snapshot(), before);
}
