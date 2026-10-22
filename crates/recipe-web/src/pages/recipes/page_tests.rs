//! Tests of `GET /recipes/{ref}` and `POST /recipes/{ref}/rename` (task 9.2).
//!
//! Specs "Recipe page", "Rename of a recipe", and "Pages of a deleted recipe". ADR-0043 (e),
//! ADR-0041 (e) and (h), design D7.

use axum::http::StatusCode;

use super::test_world::recipe_world;
use crate::pages::draft_reply::refusal_tests::assert_operator_refused;
use crate::pages::test_helpers::{assert_clean_html, assert_error, assert_htmx_error};
use crate::test_support::icon_tag;
use crate::test_support::plant::{ENGLISH, POLISH, main_of};

/// The opening tag of the rename form on the recipe page of `R-100`.
const RENAME_TAG: &str = "<form id=\"recipe-rename-form\" method=\"post\" action=\"/recipes/R-100/rename\" hx-boost=\"true\" hx-target=\"#recipe-name\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">";
/// The opening tag of the same form when an answer swaps it out of band.
const OOB_RENAME_TAG: &str = "<form id=\"recipe-rename-form\" method=\"post\" action=\"/recipes/R-100/rename\" hx-boost=\"true\" hx-target=\"#recipe-name\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";
/// The link to the form of a new version of `R-100`.
const NEW_VERSION_LINK: &str =
    "<a class=\"target-link\" href=\"/recipes/R-100/versions/new\">New version</a>";

#[tokio::test]
async fn recipes_page_of_an_operator_shows_three_versions_with_their_states_and_no_form() {
    let world = recipe_world();
    let reply = world.get_en(&world.operator, "/recipes/R-100").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<h1>Recipe R-100</h1>"), "{main}");
    let third = main
        .find("<a href=\"/recipes/R-100/versions/3\">3</a>")
        .unwrap();
    let second = main
        .find("<a href=\"/recipes/R-100/versions/2\">2</a>")
        .unwrap();
    let first = main
        .find("<a href=\"/recipes/R-100/versions/1\">1</a>")
        .unwrap();
    assert!(third < second && second < first, "{main}");
    for icon in ["status-draft", "status-approved", "status-retired"] {
        assert!(main.contains(&icon_tag(icon)), "{icon}: {main}");
    }
    assert!(!main.contains("<form id=\"recipe-rename-form\""), "{main}");
    assert!(!main.contains("/versions/new"), "{main}");
    assert!(
        main.contains("<span id=\"recipe-name\">Press recipe</span>"),
        "{main}"
    );
    assert!(
        main.contains("<a class=\"target-link\" href=\"/machines/M7\">M7</a>"),
        "{main}"
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn recipes_page_of_a_process_engineer_holds_the_rename_form_and_the_new_version_link() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes/R-100").await;
    let main = main_of(&reply.body);
    let form = main.split(RENAME_TAG).nth(1).unwrap();
    let form = form.split("</form>").next().unwrap();
    assert!(
        form.contains("<input id=\"recipe_name\" name=\"name\" type=\"text\""),
        "{form}"
    );
    assert!(form.contains("value=\"Press recipe\""), "{form}");
    assert!(
        form.contains("<button type=\"submit\" id=\"recipe-rename-submit\">"),
        "{form}"
    );
    assert!(main.contains(NEW_VERSION_LINK), "{main}");
}

#[tokio::test]
async fn recipes_page_of_a_deleted_recipe_shows_the_state_to_an_administrator_with_no_form() {
    let world = recipe_world();
    let reply = world.get_en(&world.admin, "/recipes/R-300").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("<span class=\"badge badge-large badge-deleted\">"),
        "{main}"
    );
    assert!(main.contains("<span>Deleted</span>"), "{main}");
    assert!(!main.contains("<form"), "{main}");
    assert!(!main.contains("/versions/new"), "{main}");
    assert!(
        main.contains("<a href=\"/recipes/R-300/versions/1\">1</a>"),
        "{main}"
    );
}

/// The opening tag of the one row of links of the recipe page (ADR-0049 (i), ADR-0051 (g)).
const ROW_OPEN: &str =
    "<p class=\"actions\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\">";

/// The boosted row of `reference` with the optional link to a new version and the history link.
///
/// `new_label` is the text of the new version link, or `None` when the row has no such link.
fn links_row(reference: &str, new_label: Option<&str>, history_label: &str) -> String {
    let new_link = new_label.map_or_else(String::new, |label| {
        format!("<a class=\"target-link\" href=\"/recipes/{reference}/versions/new\">{label}</a>")
    });
    format!(
        "{ROW_OPEN}{new_link}<a class=\"target-link\" href=\"/recipes/{reference}/history\">{history_label}</a></p>"
    )
}

/// The row of `reference` in English with the history link only.
fn history_link(reference: &str) -> String {
    links_row(reference, None, "History")
}

/// The fixed expectation of the row of `R-100` for each role and each language (ADR-0049 (i)).
///
/// The Operator has no right to create a version. The two other roles have it.
#[tokio::test]
async fn recipes_page_holds_one_row_with_the_links_that_the_role_may_use_in_both_languages() {
    let world = recipe_world();
    let roles = [
        ("Operator", &world.operator, false),
        ("ProcessEngineer", &world.engineer, true),
        ("Administrator", &world.admin, true),
    ];
    let languages = [
        (ENGLISH, "New version", "History"),
        (POLISH, "Nowa wersja", "Historia"),
    ];
    for (role, session, may_create) in roles {
        for (cookie, new_label, history_label) in languages {
            let reply = world.get(session, "/recipes/R-100", &[cookie]).await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
            let main = main_of(&reply.body);
            let case = format!("{role} {history_label}: {main}");
            assert_eq!(main.matches(ROW_OPEN).count(), 1, "{case}");
            assert_eq!(main.matches("/history\"").count(), 1, "{case}");
            assert_eq!(
                main.matches("/versions/new\"").count(),
                usize::from(may_create),
                "{case}"
            );
            let row = links_row("R-100", may_create.then_some(new_label), history_label);
            assert!(main.contains(&row), "{case}");
            assert_clean_html(&reply.body);
        }
    }
}

#[tokio::test]
async fn recipes_page_of_a_deleted_recipe_links_to_the_history_for_an_administrator() {
    let world = recipe_world();
    let reply = world.get_en(&world.admin, "/recipes/R-300").await;
    let main = main_of(&reply.body);
    assert!(main.contains(&history_link("R-300")), "{main}");
}

#[tokio::test]
async fn recipes_page_of_a_deleted_recipe_gives_e_rcp_012_to_the_other_roles() {
    let world = recipe_world();
    for session in [&world.engineer, &world.operator] {
        let reply = world.get_en(session, "/recipes/R-300").await;
        assert_error(&reply, StatusCode::CONFLICT, "E-RCP-012", "R-300");
    }
}

#[tokio::test]
async fn recipes_page_path_value_folds_the_case_and_refuses_a_bad_or_unknown_reference() {
    let world = recipe_world();
    let folded = world.get_en(&world.operator, "/recipes/r-100").await;
    assert_eq!(folded.status, StatusCode::OK, "{}", folded.body);
    for path in ["/recipes/-x", "/recipes/R-999", "/recipes/%FF"] {
        let reply = world.get_en(&world.operator, path).await;
        assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", path);
    }
}

#[tokio::test]
async fn recipes_page_content_answer_holds_the_title_and_the_focused_heading() {
    let world = recipe_world();
    let reply = world.get_content(&world.operator, "/recipes/R-100").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>Recipe R-100 · Recipe editor</title>")
    );
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>Recipe R-100</h1>")
    );
    assert!(!reply.body.contains("class=\"topbar\""), "{}", reply.body);
}

#[tokio::test]
async fn recipes_rename_by_htmx_answers_the_name_the_toast_and_the_clean_form() {
    let world = recipe_world();
    let path = "/recipes/R-100/rename";
    let reply = world
        .post(&world.engineer, path, "name=Renamed+recipe", true)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .starts_with("Renamed recipe<div id=\"toast\" hx-swap-oob=\"innerHTML\">"),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains("Recipe R-100 has the new name."),
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>")
    );
    let form = reply.body.split(OOB_RENAME_TAG).nth(1).unwrap();
    assert!(form.contains("value=\"Renamed recipe\""), "{form}");
    assert!(!form.contains("aria-invalid"), "{form}");
    let page = world.get_en(&world.operator, "/recipes/R-100").await;
    assert!(
        page.body
            .contains("<span id=\"recipe-name\">Renamed recipe</span>")
    );
}

#[tokio::test]
async fn recipes_rename_without_htmx_goes_to_the_recipe_page() {
    let world = recipe_world();
    let reply = world
        .post(
            &world.engineer,
            "/recipes/R-100/rename",
            "name=Other",
            false,
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-100");
}

#[tokio::test]
async fn recipes_rename_with_an_empty_name_swaps_the_form_with_a_line_and_writes_nothing() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let reply = world
        .post(&world.engineer, "/recipes/R-100/rename", "name=", true)
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    let form = reply.body.split(OOB_RENAME_TAG).nth(1).unwrap();
    assert!(form.contains("aria-invalid=\"true\""), "{form}");
    assert!(
        form.contains("<div class=\"field-error\" id=\"recipe_name-error\">"),
        "{form}"
    );
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn recipes_rename_of_a_deleted_recipe_gives_e_rcp_012_and_of_an_operator_e_web_002() {
    let world = recipe_world();
    let deleted = world
        .post(&world.admin, "/recipes/R-300/rename", "name=Back", true)
        .await;
    assert_htmx_error(&deleted, StatusCode::CONFLICT, "E-RCP-012");
    let operator = world
        .post(&world.operator, "/recipes/R-100/rename", "name=Mine", true)
        .await;
    assert_htmx_error(&operator, StatusCode::FORBIDDEN, "E-WEB-002");
}

/// K12, ADR-0041 (e): a refused rename swaps the form with the sent name and no stale line.
#[tokio::test]
async fn recipes_rename_refused_by_the_use_case_swaps_the_form_without_a_line() {
    let world = recipe_world();
    let reply = world
        .post(&world.admin, "/recipes/R-300/rename", "name=Kept", true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-RCP-012");
    assert!(reply.body.contains("Errors: 1."), "{}", reply.body);
    let (_before, rest) = reply
        .body
        .split_once("<form id=\"recipe-rename-form\"")
        .unwrap();
    let (tag, form) = rest.split_once('>').unwrap();
    assert!(tag.contains("action=\"/recipes/R-300/rename\""), "{tag}");
    assert!(tag.contains("hx-swap-oob=\"true\""), "{tag}");
    assert!(form.contains("value=\"Kept\""), "{form}");
    for mark in ["aria-invalid", "field-error"] {
        assert!(!form.contains(mark), "{mark}: {form}");
    }
}

/// K13, ADR-0013: an Operator with a bad field gets `403 E-WEB-002` and no form.
#[tokio::test]
async fn recipes_rename_of_an_operator_with_a_bad_field_gives_e_web_002_and_no_form() {
    let world = recipe_world();
    for body in ["name=", "name=A&extra=1"] {
        assert_operator_refused(&world, "/recipes/R-100/rename", body).await;
    }
}
