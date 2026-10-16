//! Tests of `GET /recipes/{ref}/versions/new` and `POST /recipes/{ref}/versions` (task 9.3).
//!
//! Spec "Version form and creation", ADR-0043 (h), ADR-0041 (a) and (e), `api.md` section 2.5.

use axum::http::StatusCode;
use recipe_domain::recipe::VersionState;

use crate::pages::recipes::test_world::{
    recipe_world, seed_parameter_on_unknown_unit, seed_recipe_of_m20,
};
use crate::pages::test_helpers::{assert_clean_html, assert_error, assert_htmx_error};
use crate::test_support::plant::{World, main_of};

/// The opening tag of the form of a new version of `R-100`.
const FORM_TAG: &str = "<form id=\"version-new-form\" method=\"post\" action=\"/recipes/R-100/versions\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";
/// The opening tag of the form of a new version of `R-600` when an answer swaps it out of band.
const OOB_FORM_TAG: &str = "<form id=\"version-new-form\" method=\"post\" action=\"/recipes/R-600/versions\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";

/// Seed the recipe `R-600` of `M20` with one approved version and no draft.
fn seed_approved_recipe(world: &World) {
    seed_recipe_of_m20(&world.fixture, (600, "R-600"), &[VersionState::Approved]);
}

#[tokio::test]
async fn versions_new_form_offers_each_version_and_the_empty_draft_with_the_approved_default() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-100/versions/new")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("<h1>New version of recipe R-100</h1>"),
        "{main}"
    );
    let form = main.split(FORM_TAG).nth(1).unwrap();
    assert!(
        form.contains(
            "<select id=\"based_on\" name=\"based_on\" aria-describedby=\"based_on-hint\">"
        ),
        "{form}"
    );
    let third = form.find("<option value=\"3\">3 (Draft)</option>").unwrap();
    let second = form
        .find("<option value=\"2\" selected>2 (Approved)</option>")
        .unwrap();
    let first = form
        .find("<option value=\"1\">1 (Retired)</option>")
        .unwrap();
    let empty = form
        .find("<option value=\"\">Empty draft</option>")
        .unwrap();
    assert!(third < second && second < first && first < empty, "{form}");
    assert!(
        form.contains("<input id=\"change_note\" name=\"change_note\" type=\"text\""),
        "{form}"
    );
    assert!(
        form.contains("<button type=\"submit\" id=\"version-create-submit\""),
        "{form}"
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn versions_new_form_refuses_an_operator_and_a_deleted_recipe() {
    let world = recipe_world();
    let operator = world
        .get_en(&world.operator, "/recipes/R-100/versions/new")
        .await;
    assert_error(&operator, StatusCode::FORBIDDEN, "E-WEB-002", "operator");
    let deleted = world
        .get_en(&world.admin, "/recipes/R-300/versions/new")
        .await;
    assert_error(&deleted, StatusCode::CONFLICT, "E-RCP-012", "deleted");
}

#[tokio::test]
async fn versions_create_copies_the_base_and_goes_to_the_new_draft() {
    let world = recipe_world();
    seed_approved_recipe(&world);
    let body = "based_on=1&change_note=Next+version";
    let reply = world
        .post(&world.engineer, "/recipes/R-600/versions", body, false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-600/versions/2");
}

#[tokio::test]
async fn versions_create_with_an_empty_base_makes_an_empty_draft() {
    let world = recipe_world();
    seed_approved_recipe(&world);
    let body = "based_on=&change_note=Empty";
    let reply = world
        .post(&world.engineer, "/recipes/R-600/versions", body, false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-600/versions/2");
}

#[tokio::test]
async fn versions_create_with_a_base_that_names_no_version_gives_e_web_008_for_based_on() {
    let world = recipe_world();
    seed_approved_recipe(&world);
    let before = world.fixture.snapshot();
    let body = "based_on=9&change_note=Kept+note";
    let reply = world
        .post(&world.engineer, "/recipes/R-600/versions", body, true)
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(
        reply
            .body
            .contains("<a class=\"target-link\" href=\"#based_on\">"),
        "{}",
        reply.body
    );
    let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(
        form.contains("<div class=\"field-error\" id=\"based_on-error\">"),
        "{form}"
    );
    assert!(form.contains("value=\"Kept note\""), "{form}");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn versions_create_lists_a_bad_base_and_a_bad_note_at_once() {
    let world = recipe_world();
    seed_approved_recipe(&world);
    let body = "based_on=abc&change_note=";
    let reply = world
        .post(&world.engineer, "/recipes/R-600/versions", body, true)
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(reply.body.contains("Errors: 2."), "{}", reply.body);
    let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 2, "{form}");
}

#[tokio::test]
async fn versions_create_on_a_recipe_with_a_draft_gives_e_rcp_011() {
    let world = recipe_world();
    let body = "based_on=2&change_note=Again";
    let reply = world
        .post(&world.engineer, "/recipes/R-100/versions", body, true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-RCP-011");
    // K12, ADR-0041 (e): the form comes back with the sent texts and no stale line.
    assert!(reply.body.contains("Errors: 1."), "{}", reply.body);
    let (_before, rest) = reply
        .body
        .split_once("<form id=\"version-new-form\"")
        .unwrap();
    let (tag, form) = rest.split_once('>').unwrap();
    assert!(tag.contains("hx-swap-oob=\"true\""), "{tag}");
    assert!(form.contains("value=\"Again\""), "{form}");
    for mark in ["aria-invalid", "field-error"] {
        assert!(!form.contains(mark), "{mark}: {form}");
    }
}

#[tokio::test]
async fn versions_create_with_a_failed_snapshot_check_shows_one_line_per_item() {
    let world = recipe_world();
    let ids = seed_recipe_of_m20(&world.fixture, (700, "R-700"), &[VersionState::Approved]);
    let base = *ids.first().unwrap();
    seed_parameter_on_unknown_unit(&world.fixture, base, (1, "P_ONE"));
    seed_parameter_on_unknown_unit(&world.fixture, base, (2, "P_TWO"));
    let body = "based_on=1&change_note=Copy";
    let reply = world
        .post(&world.engineer, "/recipes/R-700/versions", body, true)
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-020");
    assert_eq!(reply.header("hx-push-url"), "false");
    assert!(reply.body.contains("Errors: 2."), "{}", reply.body);
    for tag in ["P_ONE", "P_TWO"] {
        let line = format!(
            "<span class=\"alert-code\">E-PLT-020</span> <span class=\"data\">{tag}</span>"
        );
        assert!(reply.body.contains(&line), "{tag}: {}", reply.body);
    }
}

#[tokio::test]
async fn versions_create_of_an_operator_gives_e_web_002() {
    let world = recipe_world();
    seed_approved_recipe(&world);
    let body = "based_on=1&change_note=Mine";
    let reply = world
        .post(&world.operator, "/recipes/R-600/versions", body, true)
        .await;
    assert_htmx_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002");
}
