//! Tests of the requirement "Translation fallback" for the version page (ADR-0126, R10, A17, A44).
//!
//! The fixture parameter `SPEED` has the English name `SPEED`. The route stores its `pl` name.

use axum::http::StatusCode;

use super::rebind_world::body_v3;
use crate::pages::parameters::test_kit::{ADD, INT_FIELDS, form, parameter_path, post_htmx};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::plant::{ENGLISH, HTMX, POLISH, World};
use crate::test_support::post_fields;
use crate::test_support::recipes::{HOLD_KEY, SPEED_KEY};

/// The version page of the draft of `R-100`.
const VERSION_PATH: &str = "/recipes/R-100/versions/3";
/// The Polish name that a test stores for `SPEED`.
const STORED_NAME: &str = "Prędkość linii";

/// Return the markup of the row of the parameter `tag`, from its link to the end of the row.
fn row_of<'a>(html: &'a str, tag: &str) -> &'a str {
    let (_before, rest) = html.split_once(&format!(">{tag}</a></td>")).unwrap();
    rest.split("</tr>").next().unwrap()
}

/// Store the `pl` translation of `SPEED` through the route of the draft.
async fn store_pl_name(world: &World) {
    let pairs = [
        ("row_version", "1"),
        ("name", STORED_NAME),
        ("description", ""),
    ];
    let path = parameter_path(SPEED_KEY, "/translations/pl");
    let reply = post_htmx(world, &path, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

#[tokio::test]
async fn translation_fallback_missing_polish_translation_shows_the_english_name_with_the_marker() {
    let world = recipe_world();
    let reply = world.get(&world.engineer, VERSION_PATH, &[POLISH]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(&reply.body, "SPEED");
    assert!(
        row.contains("<td><span lang=\"en\">SPEED</span> (en)</td>"),
        "{row}"
    );
}

#[tokio::test]
async fn translation_fallback_stored_polish_translation_replaces_the_english_name() {
    let world = recipe_world();
    store_pl_name(&world).await;
    let reply = world.get(&world.engineer, VERSION_PATH, &[POLISH]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(&reply.body, "SPEED");
    let cell = format!("<td>{STORED_NAME}</td>");
    assert!(row.contains(&cell), "{row}");
    assert!(!row.contains("(en)"), "{row}");
    assert!(!row.contains("<span lang=\"en\">"), "{row}");
}

#[tokio::test]
async fn translation_fallback_english_ui_language_shows_the_english_name_without_the_marker() {
    let world = recipe_world();
    store_pl_name(&world).await;
    let reply = world.get(&world.engineer, VERSION_PATH, &[ENGLISH]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(&reply.body, "SPEED");
    assert!(row.contains("<td>SPEED</td>"), "{row}");
    assert!(!row.contains("(en)"), "{row}");
    assert!(!row.contains(STORED_NAME), "{row}");
}

#[tokio::test]
async fn translation_fallback_english_base_text_has_the_language_tag_and_the_marker_stands_outside()
{
    let world = recipe_world();
    let reply = world.get(&world.engineer, VERSION_PATH, &[POLISH]).await;
    let row = row_of(&reply.body, "SPEED");
    let (inside, after) = row
        .split_once("<span lang=\"en\">")
        .unwrap()
        .1
        .split_once("</span>")
        .unwrap();
    assert_eq!(inside, "SPEED", "{row}");
    assert!(after.starts_with(" (en)"), "{row}");
}

#[tokio::test]
async fn translation_fallback_row_of_a_new_parameter_shows_the_english_name_with_the_marker() {
    let world = recipe_world();
    let reply = post_fields(
        &world.fixture.router(),
        ADD,
        Some(&world.engineer),
        &[POLISH, HTMX],
        &form(&INT_FIELDS),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(&reply.body, "NEW_P");
    assert!(
        row.contains("<td><span lang=\"en\">New parameter</span> (en)</td>"),
        "{row}"
    );
}

/// Assert that the HTMX answer `body` shows the stored `pl` name of `SPEED` and no marker.
fn assert_translated_row(body: &str) {
    let row = row_of(body, "SPEED");
    let cell = format!("<td>{STORED_NAME}</td>");
    assert!(row.contains(&cell), "{row}");
    assert!(!row.contains("(en)"), "{row}");
}

#[tokio::test]
async fn translation_fallback_table_fragment_shows_the_stored_polish_name() {
    let world = recipe_world();
    store_pl_name(&world).await;
    let reply = world
        .get(
            &world.engineer,
            "/recipes/R-100/versions/3/parameters?page=1",
            &[POLISH, HTMX],
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_translated_row(&reply.body);
}

#[tokio::test]
async fn translation_fallback_table_of_the_removal_answer_shows_the_stored_polish_name() {
    let world = recipe_world();
    store_pl_name(&world).await;
    let path = parameter_path(HOLD_KEY, "/delete");
    let reply = post_fields(
        &world.fixture.router(),
        &path,
        Some(&world.engineer),
        &[POLISH, HTMX],
        "row_version=2",
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains(">HOLD</a>"), "{}", reply.body);
    assert_translated_row(&reply.body);
}

#[tokio::test]
async fn translation_fallback_table_of_the_rebind_answer_shows_the_stored_polish_name() {
    let world = recipe_world();
    // `body_v3` seeds the new revision first, so the later save of the name survives.
    let body = body_v3(&world, "2", &[1, 2, 3, 4], "1");
    store_pl_name(&world).await;
    let path = "/recipes/R-100/versions/3/rebind";
    let reply = post_fields(
        &world.fixture.router(),
        path,
        Some(&world.engineer),
        &[POLISH, HTMX],
        &body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_translated_row(&reply.body);
}
