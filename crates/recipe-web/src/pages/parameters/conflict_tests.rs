//! Tests of `409 E-WEB-001` on each draft route of a parameter (task 11.4).
//!
//! Spec "Conflict fragment" and "Form rendered again keeps the input", design D10 and D16,
//! ADR-0008, ADR-0039 (h). Two sessions edit one draft: the ProcessEngineer `user2` saves first,
//! and the Administrator `user1` sends the old `row_version`.

use axum::http::StatusCode;

use super::test_kit::{
    ADD, INT_FIELDS, SPEED_FIELDS, assert_panel_error, changed, form, parameter_path, post_htmx,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::field_value;
use crate::test_support::Reply;
use crate::test_support::plant::{HTMX, World};
use crate::test_support::recipes::{LINE_KEY, SPEED_KEY};

/// Send the form `pairs` to `path` as the Administrator, as an HTMX request in English.
async fn post_as_admin(world: &World, path: &str, pairs: &[(&str, &str)]) -> Reply {
    world
        .post_with(&world.admin, path, &form(pairs), &[HTMX])
        .await
}

/// Assert the conflict answer: the code, the editor of the last save, and no out-of-band form.
fn assert_conflict(reply: &Reply, what: &str) {
    assert_panel_error(reply, StatusCode::CONFLICT, "E-WEB-001");
    assert!(reply.body.contains("user2"), "{what}: {}", reply.body);
    assert!(
        reply.body.contains("Current change number: 2"),
        "{what}: {}",
        reply.body
    );
    assert!(
        !reply.body.contains("hx-swap-oob=\"true\""),
        "{what}: {}",
        reply.body
    );
}

/// Return each draft route of a parameter with a valid body of the `row_version` 1.
fn draft_routes() -> Vec<(String, Vec<(&'static str, &'static str)>)> {
    let value = vec![("row_version", "1"), ("value", "1500")];
    let limits = vec![
        ("row_version", "1"),
        ("limits[0].leaf_path", ""),
        ("limits[0].max", "2000"),
    ];
    let translation = vec![
        ("row_version", "1"),
        ("name", "Prędkość"),
        ("description", ""),
    ];
    let row = vec![("row_version", "1")];
    vec![
        (ADD.to_owned(), INT_FIELDS.to_vec()),
        (
            parameter_path(SPEED_KEY, "/definition"),
            SPEED_FIELDS.to_vec(),
        ),
        (parameter_path(SPEED_KEY, "/value"), value),
        (parameter_path(SPEED_KEY, "/limits"), limits),
        (parameter_path(SPEED_KEY, "/translations/pl"), translation),
        (
            parameter_path(LINE_KEY, "/translations/pl/delete"),
            row.clone(),
        ),
        (parameter_path(SPEED_KEY, "/delete"), row),
    ]
}

#[tokio::test]
async fn parameters_each_draft_route_gives_e_web_001_to_the_second_session() {
    for (path, pairs) in draft_routes() {
        let world = recipe_world();
        let first = post_htmx(
            &world,
            &parameter_path(SPEED_KEY, "/value"),
            &[("row_version", "1"), ("value", "1300")],
        )
        .await;
        assert_eq!(first.status, StatusCode::OK, "{path}: {}", first.body);
        let before = world.fixture.snapshot();
        let reply = post_as_admin(&world, &path, &pairs).await;
        assert_conflict(&reply, &path);
        assert_eq!(world.fixture.snapshot(), before, "{path}");
    }
}

#[tokio::test]
async fn parameters_a_form_rendered_again_keeps_the_old_row_version_and_its_save_conflicts() {
    let world = recipe_world();
    let first = post_htmx(
        &world,
        &parameter_path(SPEED_KEY, "/value"),
        &[("row_version", "1"), ("value", "1300")],
    )
    .await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let path = parameter_path(SPEED_KEY, "/definition");
    let switched = post_as_admin(
        &world,
        &path,
        &changed(&SPEED_FIELDS, &[("fields", Some("1"))]),
    )
    .await;
    assert_eq!(switched.status, StatusCode::OK, "{}", switched.body);
    assert_eq!(field_value(&switched.body, "row_version"), "1");
    let added = post_as_admin(
        &world,
        ADD,
        &changed(&INT_FIELDS, &[("limit_mode", Some("[*]"))]),
    )
    .await;
    assert_eq!(added.status, StatusCode::OK, "{}", added.body);
    assert_eq!(field_value(&added.body, "row_version"), "1");
    let saved = post_as_admin(&world, &path, &SPEED_FIELDS).await;
    assert_conflict(&saved, "save after the form again");
}
