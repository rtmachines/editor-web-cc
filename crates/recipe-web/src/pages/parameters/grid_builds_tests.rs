//! Tests of the number of grids that a save of a parameter form builds (ADR-0223 (p), task 4.5).
//!
//! `ready` builds no grid for a valid save. `failed` builds the sent grid only after the command
//! refused the save. The count of `GridView::build` is thread-local. A route test runs the router
//! on its own thread, so each test resets the count before the request.
use axum::http::StatusCode;

use super::grid::build_count::{builds, reset};
use super::test_kit::{
    ADD, INT_FIELDS, SPEED_FIELDS, assert_panel_error, changed, form, parameter_path, post_htmx,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::plant::HTMX;
use crate::test_support::recipes::SPEED_KEY;

#[tokio::test]
async fn grid_builds_zero_for_a_valid_edit_that_answers_303() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    reset();
    let reply = world
        .post(&world.engineer, &path, &form(&SPEED_FIELDS), false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(builds(), 0);
}

#[tokio::test]
async fn grid_builds_zero_for_a_valid_add_that_answers_303() {
    let world = recipe_world();
    reset();
    let reply = world
        .post(&world.engineer, ADD, &form(&INT_FIELDS), false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(builds(), 0);
}

#[tokio::test]
async fn grid_builds_one_for_the_htmx_success_of_the_edit() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    reset();
    let reply = post_htmx(&world, &path, &SPEED_FIELDS).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(builds(), 1, "only the clean form of the answer has a grid");
}

#[tokio::test]
async fn grid_builds_zero_for_a_409_conflict() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let first = post_htmx(&world, &path, &SPEED_FIELDS).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    reset();
    let reply = world
        .post_with(&world.admin, &path, &form(&SPEED_FIELDS), &[HTMX])
        .await;
    assert_panel_error(&reply, StatusCode::CONFLICT, "E-WEB-001");
    assert_eq!(builds(), 0);
}

#[tokio::test]
async fn grid_builds_one_for_a_value_that_the_command_refuses() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let pairs = changed(&SPEED_FIELDS, &[("value", Some("5000"))]);
    reset();
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-004");
    let line = "<div class=\"field-error\" id=\"value-0-error\">";
    assert!(reply.body.contains(line), "{}", reply.body);
    assert_eq!(builds(), 1);
}

#[tokio::test]
async fn grid_builds_one_for_a_field_that_the_checks_refuse() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let pairs = changed(&SPEED_FIELDS, &[("tag", Some("bad tag"))]);
    reset();
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(reply.body.contains("id=\"tag-error\""), "{}", reply.body);
    assert_eq!(builds(), 1);
}

#[tokio::test]
async fn grid_builds_one_for_a_bad_maximum_length_under_its_input() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let pairs = changed(&SPEED_FIELDS, &[("limits[0].max_length", Some("abc"))]);
    reset();
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("-error\""), "{}", reply.body);
    assert_eq!(builds(), 1);
}
