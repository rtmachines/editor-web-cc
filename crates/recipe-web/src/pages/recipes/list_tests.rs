//! Tests of `GET /recipes` with its filters, its pages, and the deleted recipes (task 9.1).
//!
//! Specs "Recipe list page", "Filters of the recipe list", "Deleted recipes in the list", and
//! "Search text and pages of a list". ADR-0043 (a) to (d).

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;

use super::test_world::{recipe_world, seed_many_recipes};
use crate::pages::test_helpers::{assert_clean_html, assert_error};
use crate::test_support::icon_tag;
use crate::test_support::plant::main_of;

/// The row of `R-100`: the link, the name, the machine, the approved version, and the draft.
const R100_LINK: &str = "<a href=\"/recipes/R-100\">R-100</a>";
/// The row of `R-200`.
const R200_LINK: &str = "<a href=\"/recipes/R-200\">R-200</a>";
/// The row of the deleted recipe `R-300`.
const R300_LINK: &str = "<a href=\"/recipes/R-300\">R-300</a>";

#[tokio::test]
async fn recipes_list_shows_the_live_recipes_in_the_byte_order_of_the_reference() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let first = main.find(R100_LINK).unwrap();
    let second = main.find(R200_LINK).unwrap();
    assert!(first < second, "{main}");
    assert!(!main.contains("R-300"), "{main}");
    assert!(main.contains("<td>Press recipe</td>"), "{main}");
    assert!(main.contains("<td class=\"data\">M7</td>"), "{main}");
    assert!(main.contains("<caption id=\"recipes-caption\">Recipe list</caption>"));
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn recipes_list_row_shows_the_approved_version_and_the_draft_with_their_states() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes").await;
    let main = main_of(&reply.body);
    let row = main.split(R100_LINK).nth(1).unwrap();
    let row = row.split("</tr>").next().unwrap();
    assert!(
        row.contains("<a href=\"/recipes/R-100/versions/2\">2</a>"),
        "{row}"
    );
    assert!(
        row.contains("<a href=\"/recipes/R-100/versions/3\">3</a>"),
        "{row}"
    );
    assert!(row.contains(&icon_tag("status-approved")), "{row}");
    assert!(row.contains(&icon_tag("status-draft")), "{row}");
    let other = main.split(R200_LINK).nth(1).unwrap();
    let other = other.split("</tr>").next().unwrap();
    assert!(other.contains("<td>none</td>"), "{other}");
}

#[tokio::test]
async fn recipes_list_filter_q_matches_a_part_of_the_name_with_no_difference_of_case() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes?q=PACKING").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains(R200_LINK), "{main}");
    assert!(!main.contains(R100_LINK), "{main}");
    assert!(main.contains("value=\"PACKING\""), "{main}");
}

#[tokio::test]
async fn recipes_list_filter_q_of_spaces_only_is_no_search_text() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes?q=+++").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains(R100_LINK), "{main}");
    assert!(main.contains(R200_LINK), "{main}");
}

#[tokio::test]
async fn recipes_list_filter_machine_keeps_the_recipes_of_one_machine() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes?machine=M20").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains(R200_LINK), "{main}");
    assert!(!main.contains(R100_LINK), "{main}");
    assert!(
        main.contains("<option value=\"M20\" selected>M20 Packing line</option>"),
        "{main}"
    );
}

#[tokio::test]
async fn recipes_list_filter_status_keeps_a_recipe_with_a_version_in_that_state() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes?status=retired")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains(R100_LINK), "{main}");
    assert!(!main.contains(R200_LINK), "{main}");
    assert!(
        main.contains("<option value=\"retired\" selected>Retired</option>"),
        "{main}"
    );
    let drafts = world.get_en(&world.engineer, "/recipes?status=draft").await;
    let main = main_of(&drafts.body);
    assert!(
        main.contains(R100_LINK) && main.contains(R200_LINK),
        "{main}"
    );
}

#[tokio::test]
async fn recipes_list_filter_of_a_code_without_a_live_machine_gives_e_web_005() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes?machine=M99").await;
    assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", "machine M99");
    assert!(!reply.body.contains("value=\"M99\""), "{}", reply.body);
}

#[tokio::test]
async fn recipes_list_unknown_state_gives_e_web_008() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes?status=open").await;
    assert_error(
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
        "status",
    );
    assert!(reply.body.contains("State"), "{}", reply.body);
}

#[tokio::test]
async fn recipes_list_malformed_machine_gives_e_web_005() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes?machine=X1").await;
    assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", "machine");
}

#[tokio::test]
async fn recipes_list_search_form_sends_get_with_the_four_fields_and_no_token() {
    let world = recipe_world();
    let reply = world.get_en(&world.admin, "/recipes").await;
    let main = main_of(&reply.body);
    let form = main
        .split("<form method=\"get\" action=\"/recipes\"")
        .nth(1)
        .unwrap();
    let form = form.split("</form>").next().unwrap();
    assert!(
        form.contains("id=\"recipe_q\" name=\"q\" type=\"search\""),
        "{form}"
    );
    assert!(
        form.contains("<select id=\"recipe_machine\" name=\"machine\""),
        "{form}"
    );
    assert!(
        form.contains("<select id=\"recipe_status\" name=\"status\""),
        "{form}"
    );
    assert!(
        form.contains("<option value=\"\" selected>Every machine</option>"),
        "{form}"
    );
    assert!(
        form.contains("<option value=\"M7\">M7 Press line</option>"),
        "{form}"
    );
    assert!(!form.contains("M12"), "{form}");
    assert!(form.contains("name=\"show_deleted\" value=\"1\""), "{form}");
    assert!(!form.contains("csrf"), "{form}");
}

#[tokio::test]
async fn recipes_list_offers_the_deleted_recipes_and_the_new_recipe_link_only_by_permission() {
    let world = recipe_world();
    let engineer = world.get_en(&world.engineer, "/recipes").await;
    let main = main_of(&engineer.body);
    assert!(!main.contains("show_deleted"), "{main}");
    assert!(
        main.contains("<a class=\"target-link\" href=\"/recipes/new\">New recipe</a>"),
        "{main}"
    );
    let operator = world.get_en(&world.operator, "/recipes").await;
    let main = main_of(&operator.body);
    assert!(!main.contains("/recipes/new"), "{main}");
    assert!(!main.contains("show_deleted"), "{main}");
}

#[tokio::test]
async fn recipes_list_show_deleted_of_an_administrator_adds_the_deleted_recipe() {
    let world = recipe_world();
    let reply = world.get_en(&world.admin, "/recipes?show_deleted=1").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let row = main.split(R300_LINK).nth(1).unwrap();
    let row = row.split("</tr>").next().unwrap();
    assert!(row.contains(&icon_tag("status-retired")), "{row}");
    assert!(row.contains("<span>Deleted</span>"), "{row}");
    assert!(main.contains(R100_LINK), "{main}");
    assert!(
        main.contains("name=\"show_deleted\" value=\"1\" checked"),
        "{main}"
    );
}

#[tokio::test]
async fn recipes_list_show_deleted_of_a_process_engineer_gives_e_web_002() {
    let world = recipe_world();
    let before = world.fixture.sink.events().len();
    let reply = world
        .get_en(&world.engineer, "/recipes?show_deleted=1")
        .await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", "show_deleted");
    let events = world.fixture.sink.events();
    assert_eq!(events.len(), before.checked_add(1).unwrap());
    assert!(
        events
            .last()
            .is_some_and(|event| event.kind() == SecurityEventKind::Forbidden)
    );
}

#[tokio::test]
async fn recipes_list_second_page_shows_one_row_and_a_link_to_the_first_page() {
    let world = recipe_world();
    seed_many_recipes(&world.fixture, 99);
    let first = world.get_en(&world.engineer, "/recipes").await;
    let main = main_of(&first.body);
    assert_eq!(main.matches("<tr>").count(), 101, "{main}");
    assert!(main.contains("href=\"/recipes?page=2\""), "{main}");
    let second = world.get_en(&world.engineer, "/recipes?page=2").await;
    let main = main_of(&second.body);
    assert_eq!(main.matches("<tr>").count(), 2, "{main}");
    assert!(main.contains(R200_LINK), "{main}");
    assert!(main.contains("href=\"/recipes?page=1\""), "{main}");
}

#[tokio::test]
async fn recipes_list_pager_keeps_the_filters() {
    let world = recipe_world();
    seed_many_recipes(&world.fixture, 101);
    let reply = world
        .get_en(
            &world.engineer,
            "/recipes?q=R-1&machine=M20&status=approved",
        )
        .await;
    let main = main_of(&reply.body);
    assert!(
        main.contains("href=\"/recipes?q=R-1&#38;machine=M20&#38;status=approved&#38;page=2\""),
        "{main}"
    );
}

#[tokio::test]
async fn recipes_list_page_after_the_end_shows_the_notice_and_a_link_to_the_first_page() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes?page=2").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("Page 2 is after the last page."), "{main}");
    assert!(main.contains("href=\"/recipes?page=1\""), "{main}");
    assert!(!main.contains("No recipe to show."), "{main}");
}

#[tokio::test]
async fn recipes_list_bad_page_gives_e_web_008() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes?page=0").await;
    assert_error(
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
        "page",
    );
}
