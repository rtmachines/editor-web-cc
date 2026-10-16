//! Tests of `POST .../parameters/{key}/definition`: the save of the edit form (task 11.2).
//!
//! Specs "One edit form of a parameter", "Form rendered again keeps the input", and "Value fields
//! of a form", ADR-0039 (e), (h), (i), ADR-0041 (e) and (h).

use axum::http::StatusCode;

use super::test_kit::{SPEED_FIELDS, assert_panel_error, changed, form, parameter_path, post_htmx};
use super::value_tests::assert_operator_refused;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::field_value;
use crate::test_support::recipes::{GRID_KEY, HOLD_KEY, LABEL_KEY, SPEED_KEY, SPEED_TABLE_KEY};

/// The opening tag of the edit form of `SPEED` when an answer swaps it out of band.
fn oob_form_tag() -> String {
    format!(
        "<form id=\"parameter-edit-form\" method=\"post\" action=\"{}\" hx-boost=\"true\" hx-target=\"#parameter-summary\" hx-swap=\"innerHTML show:none focus-scroll:true\" hx-push-url=\"false\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">",
        parameter_path(SPEED_KEY, "/definition")
    )
}

#[tokio::test]
async fn parameters_edit_saves_the_value_and_the_limits_and_swaps_a_clean_form() {
    let world = recipe_world();
    let pairs = changed(
        &SPEED_FIELDS,
        &[
            ("value", Some("3500")),
            ("limits[0].min", Some("3000")),
            ("limits[0].max", Some("4000")),
        ],
    );
    let path = parameter_path(SPEED_KEY, "/definition");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    assert!(
        reply.body.contains("Parameter SPEED is saved."),
        "{}",
        reply.body
    );
    let form = reply.body.split(&oob_form_tag()).nth(1).unwrap();
    assert!(form.contains("value=\"3500\""), "{form}");
    assert!(form.contains("value=\"4000\""), "{form}");
    assert_eq!(field_value(form, "row_version"), "2");
    assert!(!form.contains("field-error"), "{form}");
    let page = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert!(page.body.contains("value=\"3500\""), "{}", page.body);
}

/// The opening tag of the edit form of the parameter `key` when an answer swaps it out of band.
fn oob_tag_of(key: uuid::Uuid) -> String {
    oob_form_tag().replace(
        &parameter_path(SPEED_KEY, "/definition"),
        &parameter_path(key, "/definition"),
    )
}

#[tokio::test]
async fn parameters_edit_swaps_a_clean_form_with_the_stored_time_texts() {
    // ADR-0041 (h), ADR-0039 (c), A43: the clean form shows the stored texts, not the sent ones.
    let world = recipe_world();
    let pairs = [
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "HOLD"),
        ("name_en", "HOLD"),
        ("description_en", "A parameter of the press line"),
        ("type", "TIME"),
        ("value_type", "TIME"),
        ("s7_address", "DB2.DBD24"),
        ("ordinal", "3"),
        ("value", "T#90s"),
        ("limits[0].leaf_path", ""),
        ("limits[0].min", "T#90s"),
        ("limits[0].max", "T#2m"),
    ];
    let reply = post_htmx(&world, &parameter_path(HOLD_KEY, "/definition"), &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = reply.body.split(&oob_tag_of(HOLD_KEY)).nth(1).unwrap();
    assert_eq!(form.matches("value=\"T#1m30s\"").count(), 2, "{form}");
    assert!(!form.contains("T#90s"), "{form}");
    assert_eq!(field_value(form, "row_version"), "2");
}

#[tokio::test]
async fn parameters_edit_swaps_a_clean_form_with_the_stored_number_texts() {
    let world = recipe_world();
    let pairs = [
        ("row_version", "1"),
        ("unit_path", "U1.3"),
        ("tag", "GRID"),
        ("name_en", "GRID"),
        ("description_en", "A parameter of the press line"),
        ("type", "ARRAY[0..1,1..2] OF REAL"),
        ("value_type", "ARRAY[0..1,1..2] OF REAL"),
        ("s7_address", "DB3.DBB20"),
        ("ordinal", "5"),
        ("value[0,1]", "1.50"),
        ("value[0,2]", "1.5"),
        ("value[1,1]", "2.50"),
        ("value[1,2]", "3.5"),
    ];
    let reply = post_htmx(&world, &parameter_path(GRID_KEY, "/definition"), &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = reply.body.split(&oob_tag_of(GRID_KEY)).nth(1).unwrap();
    assert_eq!(form.matches("value=\"1.5\"").count(), 2, "{form}");
    assert_eq!(form.matches("value=\"2.5\"").count(), 1, "{form}");
    assert!(!form.contains("value=\"1.50\""), "{form}");
    assert!(!form.contains("value=\"2.50\""), "{form}");
}

#[tokio::test]
async fn parameters_edit_with_a_changed_type_shows_the_new_fields_and_writes_nothing() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = changed(&SPEED_FIELDS, &[("type", Some("REAL"))]);
    let path = parameter_path(SPEED_KEY, "/definition");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#parameter-edit-form");
    assert!(
        reply
            .body
            .contains("The type changed. Enter the values of the new type."),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains("id=\"grid-hint-number\""),
        "{}",
        reply.body
    );
    assert_eq!(field_value(&reply.body, "value_type"), "REAL");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_edit_keeps_the_spaces_of_a_string_leaf() {
    let world = recipe_world();
    let pairs = [
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "LABEL"),
        ("name_en", "LABEL"),
        ("description_en", "A parameter of the press line"),
        ("type", "STRING(20)"),
        ("value_type", "STRING(20)"),
        ("s7_address", "DB2.DBB2"),
        ("ordinal", "2"),
        ("value", "  Linia A  "),
    ];
    let path = parameter_path(LABEL_KEY, "/definition");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let page = world
        .get_en(&world.engineer, &parameter_path(LABEL_KEY, ""))
        .await;
    assert!(page.body.contains("value=\"  Linia A  \""), "{}", page.body);
}

#[tokio::test]
async fn parameters_edit_of_a_partial_value_names_the_missing_leaf() {
    let world = recipe_world();
    let mut pairs = vec![
        ("row_version", "1"),
        ("unit_path", "U1.3"),
        ("tag", "SPEED_TABLE"),
        ("name_en", "SPEED_TABLE"),
        ("description_en", "A parameter of the press line"),
        ("type", "ARRAY[1..8] OF INT"),
        ("value_type", "ARRAY[1..8] OF INT"),
        ("s7_address", "DB3.DBB0"),
        ("ordinal", "4"),
    ];
    let names = [
        "value[1]", "value[2]", "value[3]", "value[4]", "value[6]", "value[7]", "value[8]",
    ];
    pairs.extend(names.iter().map(|name| (*name, "1")));
    let path = parameter_path(SPEED_TABLE_KEY, "/definition");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-005");
    assert!(
        reply.body.contains("id=\"value-4-error\""),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn parameters_edit_of_a_bad_value_shows_the_line_under_the_input() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = changed(&SPEED_FIELDS, &[("value", Some("abc"))]);
    let path = parameter_path(SPEED_KEY, "/definition");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-005");
    let form = reply.body.split(&oob_form_tag()).nth(1).unwrap();
    assert!(form.contains("id=\"value-0-error\""), "{form}");
    assert!(form.contains("value=\"abc\""), "{form}");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_edit_of_a_key_that_the_draft_lacks_gives_e_rcp_051() {
    let world = recipe_world();
    let path = parameter_path(uuid::Uuid::from_u128(0xdead), "/definition");
    for extra in [None, Some(("fields", "1"))] {
        let pairs = changed(
            &SPEED_FIELDS,
            &[("fields", extra.map(|(_key, value)| value))],
        );
        let reply = post_htmx(&world, &path, &pairs).await;
        assert_panel_error(&reply, StatusCode::NOT_FOUND, "E-RCP-051");
    }
}

#[tokio::test]
async fn parameters_edit_of_an_operator_with_a_bad_field_gives_e_web_002() {
    let world = recipe_world();
    let body = form(&changed(&SPEED_FIELDS, &[("bogus", Some("1"))]));
    let path = parameter_path(SPEED_KEY, "/definition");
    assert_operator_refused(&world, &path, &body).await;
}

#[tokio::test]
async fn parameters_edit_without_htmx_goes_to_the_parameter_page() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let reply = world
        .post(&world.engineer, &path, &form(&SPEED_FIELDS), false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), parameter_path(SPEED_KEY, ""));
}
