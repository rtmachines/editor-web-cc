//! Tests of `GET /recipes/{ref}/versions` and the version table of the recipe page (task 9.3).
//!
//! Specs "Recipe page" and "Search text and pages of a list". ADR-0043 (a) and (e), design D7.

use axum::http::StatusCode;
use recipe_domain::recipe::VersionState;

use crate::pages::recipes::test_world::{recipe_world, seed_recipe_of_m20};
use crate::pages::test_helpers::{assert_clean_html, assert_error};
use crate::test_support::plant::{ENGLISH, HTMX, World, main_of};

/// The header of an HTMX request that targets the version table.
const TARGET_TABLE: (&str, &str) = ("hx-target", "versions-table");
/// The wrapper of the pager of the version table. Its `hx-*` attributes are literals.
const PAGER_WRAPPER: &str = "<div hx-boost=\"true\" hx-target=\"#versions-table\" hx-swap=\"innerHTML\" hx-push-url=\"false\">";

/// Seed the recipe `R-600` of `M20` with 101 retired versions.
fn seed_long_history(world: &World) {
    let states = vec![VersionState::Retired; 101];
    seed_recipe_of_m20(&world.fixture, (600, "R-600"), &states);
}

/// Count the rows of the version table: each row starts with its number cell.
fn version_rows(html: &str) -> usize {
    html.matches("<tr><td class=\"num\">").count()
}

#[tokio::test]
async fn versions_list_of_htmx_answers_the_table_fragment() {
    let world = recipe_world();
    let headers = [ENGLISH, HTMX, TARGET_TABLE];
    let reply = world
        .get(&world.operator, "/recipes/R-100/versions", &headers)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.starts_with("<div class=\"table-wrap\""),
        "{}",
        reply.body
    );
    assert_eq!(version_rows(&reply.body), 3, "{}", reply.body);
    assert!(!reply.body.contains("<h1"), "{}", reply.body);
    assert!(!reply.body.contains("<html"), "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<caption id=\"versions-caption\">Versions</caption>")
    );
}

#[tokio::test]
async fn versions_list_without_htmx_answers_the_recipe_page() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.operator, "/recipes/R-100/versions")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<h1>Recipe R-100</h1>"), "{main}");
    assert!(main.contains("<div id=\"versions-table\">"), "{main}");
    assert_eq!(version_rows(main), 3, "{main}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn versions_list_shows_one_hundred_rows_and_a_boosted_pager_to_the_next_page() {
    let world = recipe_world();
    seed_long_history(&world);
    let reply = world.get_en(&world.operator, "/recipes/R-600").await;
    let main = main_of(&reply.body);
    assert_eq!(version_rows(main), 100, "{main}");
    assert!(
        main.contains("<a href=\"/recipes/R-600/versions/101\">101</a>"),
        "{main}"
    );
    assert!(
        !main.contains("<a href=\"/recipes/R-600/versions/1\">1</a>"),
        "{main}"
    );
    let pager = main.split(PAGER_WRAPPER).nth(1).unwrap();
    assert!(
        pager.contains("href=\"/recipes/R-600/versions?page=2\""),
        "{pager}"
    );
}

#[tokio::test]
async fn versions_list_second_page_holds_the_oldest_version_and_a_link_back() {
    let world = recipe_world();
    seed_long_history(&world);
    let headers = [ENGLISH, HTMX, TARGET_TABLE];
    let reply = world
        .get(&world.operator, "/recipes/R-600/versions?page=2", &headers)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(version_rows(&reply.body), 1, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<a href=\"/recipes/R-600/versions/1\">1</a>")
    );
    assert!(
        reply
            .body
            .contains("href=\"/recipes/R-600/versions?page=1\"")
    );
    let page = world.get_en(&world.operator, "/recipes/R-600?page=2").await;
    assert_eq!(version_rows(main_of(&page.body)), 1, "{}", page.body);
}

#[tokio::test]
async fn versions_list_page_after_the_end_shows_the_notice() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.operator, "/recipes/R-100/versions?page=2")
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains("Page 2 is after the last page."), "{main}");
    assert!(
        main.contains("href=\"/recipes/R-100/versions?page=1\""),
        "{main}"
    );
}

#[tokio::test]
async fn versions_list_refuses_a_bad_page_and_the_deleted_recipe_for_the_other_roles() {
    let world = recipe_world();
    let bad = world
        .get_en(&world.operator, "/recipes/R-100/versions?page=x")
        .await;
    assert_error(&bad, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008", "page");
    let deleted = world
        .get_en(&world.operator, "/recipes/R-300/versions")
        .await;
    assert_error(&deleted, StatusCode::CONFLICT, "E-RCP-012", "deleted");
    let admin = world.get_en(&world.admin, "/recipes/R-300/versions").await;
    assert_eq!(admin.status, StatusCode::OK, "{}", admin.body);
}

#[tokio::test]
async fn versions_list_row_shows_the_state_the_note_and_the_times() {
    let world = recipe_world();
    let reply = world.get_en(&world.operator, "/recipes/R-100").await;
    let main = main_of(&reply.body);
    let row = main
        .split("<a href=\"/recipes/R-100/versions/2\">2</a>")
        .nth(1)
        .unwrap();
    let row = row.split("</tr>").next().unwrap();
    assert!(
        row.contains("<span class=\"badge badge-approved\">"),
        "{row}"
    );
    assert!(row.contains("<span>Approved</span>"), "{row}");
    assert!(row.contains(" UTC</td>"), "{row}");
}
