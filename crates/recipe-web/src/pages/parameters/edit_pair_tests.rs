//! Tests of the pair rule of value fields and limit fields (ADR-0213 (i)).

use axum::http::StatusCode;

use super::edit_tests::{oob_form_tag, text_input_value};
use super::test_kit::{SPEED_FIELDS, assert_panel_error, changed, parameter_path, post_htmx};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::summary_lines;
use crate::test_support::plant::World;
use crate::test_support::recipes::SPEED_KEY;

/// Assert that `SPEED` still has its stored value and limits after a refused save.
async fn assert_speed_unchanged(world: &World) {
    let page = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(
        text_input_value(&page.body, "value"),
        "1200",
        "{}",
        page.body
    );
    assert_eq!(text_input_value(&page.body, "limits[0].min"), "0");
    assert_eq!(text_input_value(&page.body, "limits[0].max"), "3000");
}

/// ADR-0213 (i), design D5: value fields without limit fields give `E-WEB-008` and write nothing.
///
/// HEAD answered `200` and deleted every limit of the parameter.
#[tokio::test]
async fn value_fields_without_limit_fields_give_e_web_008() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = changed(
        &SPEED_FIELDS,
        &[
            ("value", Some("1500")),
            ("limits[0].leaf_path", None),
            ("limits[0].min", None),
            ("limits[0].max", None),
        ],
    );
    let path = parameter_path(SPEED_KEY, "/definition");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    assert!(
        !reply.body.contains("parameter-edit-form"),
        "{}",
        reply.body
    );
    assert_eq!(world.fixture.snapshot(), before);
    assert_speed_unchanged(&world).await;
}

/// ADR-0213 (i), design D5: limit fields without value fields give `E-WEB-008` and write nothing.
///
/// HEAD passed the request to the use case with a new value and the sent limits.
#[tokio::test]
async fn limit_fields_without_value_fields_give_e_web_008() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = changed(
        &SPEED_FIELDS,
        &[("value", None), ("limits[0].max", Some("4000"))],
    );
    let path = parameter_path(SPEED_KEY, "/definition");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    assert!(
        !reply.body.contains("parameter-edit-form"),
        "{}",
        reply.body
    );
    assert_eq!(world.fixture.snapshot(), before);
    assert_speed_unchanged(&world).await;
}

/// Design D5: a `BOOL` leaf takes no limit, so its form sends no limit fields and the save works.
#[tokio::test]
async fn bool_value_without_limit_fields_is_saved() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    // The first save changes the type of `SPEED` to `BOOL`. It sends both parts of the new value.
    let retype = changed(
        &SPEED_FIELDS,
        &[
            ("type", Some("BOOL")),
            ("value_type", Some("BOOL")),
            ("value", Some("true")),
            ("s7_address", Some("DB2.DBX0.0")),
            ("limits[0].leaf_path", None),
            ("limits[0].min", None),
            ("limits[0].max", None),
        ],
    );
    let first = post_htmx(&world, &path, &retype).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    // The second save keeps the type. It sends a value and no limit field.
    let pairs = changed(
        &retype,
        &[("row_version", Some("2")), ("value", Some("false"))],
    );
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("Parameter SPEED is saved."),
        "{}",
        reply.body
    );
}

/// Design D5: a limit row with blank `min` and `max` is a limit part. A new value next to it works.
#[tokio::test]
async fn blank_limit_row_with_a_value_is_saved() {
    let world = recipe_world();
    let pairs = changed(
        &SPEED_FIELDS,
        &[
            ("value", Some("1500")),
            ("limits[0].min", Some("")),
            ("limits[0].max", Some("")),
        ],
    );
    let path = parameter_path(SPEED_KEY, "/definition");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = reply.body.split(&oob_form_tag()).nth(1).unwrap();
    assert_eq!(text_input_value(form, "value"), "1500");
    assert!(!form.contains("field-error"), "{form}");
    // The stored parameter has the new value and no bound: both blank fields removed the limits.
    let page = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(
        text_input_value(&page.body, "value"),
        "1500",
        "{}",
        page.body
    );
    assert_eq!(
        text_input_value(&page.body, "limits[0].min"),
        "",
        "{}",
        page.body
    );
    assert_eq!(
        text_input_value(&page.body, "limits[0].max"),
        "",
        "{}",
        page.body
    );
}
