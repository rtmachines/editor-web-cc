//! Tests of the cap on the bound texts of a limit row at the routes (ADR-0225 (a) to (d), task 2.2).
//!
//! A `min`, `max`, or `max_length` text above `LIMIT_TEXT_MAX` bytes gives one line `422 E-WEB-008`
//! from the reader. The answer holds no out-of-band form and no grid, and the command writes nothing.

use axum::http::StatusCode;

use crate::error::FieldReason;
use crate::pages::parameters::grid::build_count::{builds, reset};
use crate::pages::parameters::test_kit::{
    ADD, INT_FIELDS, SPEED_FIELDS, assert_panel_error, changed, form, parameter_path, post_htmx,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::summary_lines;
use crate::settings::LIMIT_TEXT_MAX;
use crate::test_support::Reply;
use crate::test_support::plant::World;
use crate::test_support::recipes::{LABEL_KEY, SPEED_KEY};

/// The text of `reason-limit-text` in English.
const LIMIT_TEXT_LINE: &str = "a limit field has more than 256 bytes. Make the text shorter";

/// The label of `row_version` in English (`field-row-version`). A bad `row_version` shows it.
const ROW_VERSION_LABEL: &str = "Change number";

/// The start of the line with the label `Form` (`reason-limit-text` follows it).
const LABEL_TEXT_LINE: &str = "Field Form is not valid: ";

/// Return a bound text of `LIMIT_TEXT_MAX` plus one bytes.
fn over_long() -> String {
    "1".repeat(LIMIT_TEXT_MAX + 1)
}

/// Return the path of the definition route of `SPEED`.
fn definition() -> String {
    parameter_path(SPEED_KEY, "/definition")
}

/// Return the opening tag of the input `id` in `body`.
fn input_tag<'a>(body: &'a str, id: &str) -> &'a str {
    let (_before, rest) = body.split_once(&format!("<input id=\"{id}\"")).unwrap();
    rest.split('>').next().unwrap()
}

/// Return the definition fields of `SPEED` with an array type and one row `[*]` with `min` `text`.
fn array_fields<'a>(
    text: &'a str,
    extra: &[(&'a str, Option<&'a str>)],
) -> Vec<(&'a str, &'a str)> {
    let mut changes = vec![
        ("type", Some("ARRAY[1..4] OF INT")),
        ("value_type", Some("ARRAY[1..4] OF INT")),
        ("value", None),
        ("s7_address", Some("DB2.DBB0")),
        ("limits[0].leaf_path", Some("[*]")),
        ("limits[0].min", Some(text)),
    ];
    changes.extend_from_slice(extra);
    changed(&SPEED_FIELDS, &changes)
}

/// Assert one line `422 E-WEB-008` with the text of `LimitText`, and no form and no grid.
fn assert_one_limit_text_line(reply: &Reply, htmx: bool) {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-008"), "{}", reply.body);
    assert!(reply.body.contains(LIMIT_TEXT_LINE), "{}", reply.body);
    assert!(reply.body.contains(LABEL_TEXT_LINE), "{}", reply.body);
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    for needle in [
        "class=\"field-error\"",
        "hx-swap-oob=\"true\"",
        "name=\"limit_mode\"",
    ] {
        assert!(!reply.body.contains(needle), "{needle}: {}", reply.body);
    }
    assert_eq!(
        reply.header("hx-retarget") == "#error-panel",
        htmx,
        "{:?}",
        reply.headers
    );
    assert_eq!(builds(), 0, "the refusal builds no grid");
}

/// Send `pairs` to `path` and assert the one line of `LimitText`. The data stays unchanged.
async fn assert_refused(world: &World, path: &str, pairs: &[(&str, &str)], htmx: bool) {
    let before = world.fixture.snapshot();
    reset();
    let reply = if htmx {
        post_htmx(world, path, pairs).await
    } else {
        world.post(&world.engineer, path, &form(pairs), false).await
    };
    if htmx {
        assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    }
    assert_one_limit_text_line(&reply, htmx);
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn limit_text_definition_save_gives_one_line_with_htmx() {
    let world = recipe_world();
    let text = over_long();
    assert_refused(&world, &definition(), &array_fields(&text, &[]), true).await;
}

#[tokio::test]
async fn limit_text_definition_save_gives_one_line_without_htmx() {
    let world = recipe_world();
    let text = over_long();
    assert_refused(&world, &definition(), &array_fields(&text, &[]), false).await;
}

#[tokio::test]
async fn limit_text_definition_fields_fragment_gives_the_same_line() {
    let world = recipe_world();
    let text = over_long();
    let pairs = array_fields(&text, &[("fields", Some("1"))]);
    assert_refused(&world, &definition(), &pairs, true).await;
}

#[tokio::test]
async fn limit_text_definition_limit_mode_switch_gives_the_same_line() {
    let world = recipe_world();
    let text = over_long();
    let pairs = array_fields(&text, &[("limit_mode", Some("[*]"))]);
    assert_refused(&world, &definition(), &pairs, true).await;
}

#[tokio::test]
async fn limit_text_add_with_an_array_type_gives_one_line() {
    let world = recipe_world();
    let text = over_long();
    let pairs = changed(
        &INT_FIELDS,
        &[
            ("type", Some("ARRAY[1..8] OF INT")),
            ("value_type", Some("ARRAY[1..8] OF INT")),
            ("value", None),
            ("s7_address", Some("DB9.DBB0")),
            ("limits[0].leaf_path", Some("[*]")),
            ("limits[0].min", Some(text.as_str())),
        ],
    );
    assert_refused(&world, ADD, &pairs, true).await;
}

/// The stored `max_length` of `LABEL` before the refused save.
const STORED_MAX_LENGTH: &str = "10";

#[tokio::test]
async fn limit_text_limits_save_gives_the_line_of_the_cap_and_not_of_max_length() {
    let world = recipe_world();
    let path = parameter_path(LABEL_KEY, "/limits");
    let stored = [
        ("row_version", "1"),
        ("limits[0].leaf_path", ""),
        ("limits[0].max_length", STORED_MAX_LENGTH),
    ];
    let saved = post_htmx(&world, &path, &stored).await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    let text = over_long();
    let pairs = [
        ("row_version", "2"),
        ("limits[0].leaf_path", ""),
        ("limits[0].max_length", text.as_str()),
    ];
    let max_length_line = FieldReason::MaxLength.to_string();
    assert_refused(&world, &path, &pairs, true).await;
    let reply = post_htmx(&world, &path, &pairs).await;
    assert!(!reply.body.contains(&max_length_line), "{}", reply.body);
    let page = world
        .get_en(&world.engineer, &parameter_path(LABEL_KEY, ""))
        .await;
    let tag = input_tag(&page.body, "limits-0-max_length");
    assert!(
        tag.contains(&format!("value=\"{STORED_MAX_LENGTH}\"")),
        "{tag}"
    );
}

#[tokio::test]
async fn limit_text_limits_save_with_a_bad_row_version_gives_only_the_line_of_the_cap() {
    let world = recipe_world();
    let text = over_long();
    let pairs = [
        ("row_version", "abc"),
        ("limits[0].leaf_path", ""),
        ("limits[0].min", text.as_str()),
    ];
    let reply = post_htmx(&world, &parameter_path(SPEED_KEY, "/limits"), &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(reply.body.contains(LIMIT_TEXT_LINE), "{}", reply.body);
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    assert!(!reply.body.contains(ROW_VERSION_LABEL), "{}", reply.body);
}

/// Compatibility, which passes before and after the task: a numeric text of exactly the cap in
/// bytes keeps its value. `1.` with zeros in the row `[*]` of an array stores the minimum 1.
#[tokio::test]
async fn limit_text_a_minimum_of_the_cap_in_bytes_with_trailing_zeros_stores_one() {
    let world = recipe_world();
    let text = format!("1.{}", "0".repeat(LIMIT_TEXT_MAX - 2));
    assert_eq!(text.len(), LIMIT_TEXT_MAX);
    let values = [
        ("value[1]", Some("10")),
        ("value[2]", Some("20")),
        ("value[3]", Some("30")),
        ("value[4]", Some("40")),
    ];
    let pairs = changed(
        &array_fields(&text, &values),
        &[("s7_address", Some("DB20.DBB0"))],
    );
    let reply = post_htmx(&world, &definition(), &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let page = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert!(
        page.body
            .contains("name=\"limits[0].leaf_path\" value=\"[*]\""),
        "{}",
        page.body
    );
    assert!(input_tag(&page.body, "limits-0-min").contains("value=\"1\""));
}
