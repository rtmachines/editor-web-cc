//! Tests of `POST .../{key}/value`, `GET .../{key}/limits`, and `POST .../{key}/limits` (task 11.2).
//!
//! `api.md` section 2.6, design D7: the value route answers the read-only grid, and the limit routes
//! answer the read-only limit rows. A time bound shows as an IEC literal (A43).

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;

use super::fields::max_pairs;
use super::test_kit::{assert_panel_error, parameter_path, post_htmx};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::assert_htmx_error;
use crate::test_support::plant::{ENGLISH, HTMX, World};
use crate::test_support::recipes::{HOLD_KEY, SPEED_KEY};

#[tokio::test]
async fn parameters_value_saves_the_value_and_answers_the_read_only_grid() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/value");
    let reply = post_htmx(&world, &path, &[("row_version", "1"), ("value", "1500")]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    assert!(
        reply.body.contains("<td class=\"data\">1500</td>"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("<input"), "{}", reply.body);
}

#[tokio::test]
async fn parameters_value_errors_keep_the_url() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let bad = post_htmx(
        &world,
        &parameter_path(SPEED_KEY, "/value"),
        &[("row_version", "1"), ("value", "abc")],
    )
    .await;
    assert_panel_error(&bad, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-005");
    let over = post_htmx(
        &world,
        &parameter_path(SPEED_KEY, "/value"),
        &[("row_version", "1"), ("value", "4000")],
    )
    .await;
    assert_panel_error(&over, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-004");
    let missing = post_htmx(
        &world,
        &parameter_path(uuid::Uuid::from_u128(0xdead), "/value"),
        &[("row_version", "1"), ("value", "1")],
    )
    .await;
    assert_panel_error(&missing, StatusCode::NOT_FOUND, "E-RCP-051");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_value_with_a_flood_of_pairs_gives_e_web_008_and_writes_nothing() {
    // Spec "Flood of pairs", ADR-0039 (b): one pair above `max_pairs()` gives one line.
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let flood: String = (0..max_pairs())
        .map(|index| format!("&value%5B{index}%5D=1"))
        .collect();
    let body = format!("row_version=1{flood}");
    let path = parameter_path(SPEED_KEY, "/value");
    let reply = world
        .post_with(&world.engineer, &path, &body, &[HTMX])
        .await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(world.fixture.snapshot(), before);
}

/// Send `body` with a bad field to `path` as the Operator. Assert `403 E-WEB-002`, one event
/// `forbidden`, no form in the answer, and no write (ADR-0013: the permission check comes first).
pub(super) async fn assert_operator_refused(world: &World, path: &str, body: &str) {
    let before = world.fixture.snapshot();
    let events = world.fixture.sink.events().len();
    let reply = world.post_with(&world.operator, path, body, &[HTMX]).await;
    assert_htmx_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002");
    let after = world.fixture.sink.events();
    assert_eq!(after.len(), events.checked_add(1).unwrap(), "{path}");
    assert!(
        after
            .last()
            .is_some_and(|event| event.kind() == SecurityEventKind::Forbidden),
        "{path}"
    );
    assert!(!reply.body.contains("<form"), "{path}: {}", reply.body);
    assert_eq!(world.fixture.snapshot(), before, "{path}");
}

#[tokio::test]
async fn parameters_value_of_an_operator_with_a_bad_field_gives_e_web_002() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/value");
    assert_operator_refused(&world, &path, "row_version=1&bogus=1").await;
}

#[tokio::test]
async fn parameters_limits_of_an_operator_with_a_bad_field_gives_e_web_002() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/limits");
    assert_operator_refused(&world, &path, "row_version=x&limits%5B0%5D.min=1").await;
}

#[tokio::test]
async fn parameters_limits_shows_the_rows_of_the_limits() {
    let world = recipe_world();
    let reply = world
        .get(
            &world.operator,
            &parameter_path(SPEED_KEY, "/limits"),
            &[ENGLISH, HTMX],
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<td class=\"data\">0</td><td class=\"data\">3000</td>"),
        "{}",
        reply.body
    );
}

#[test]
fn parameters_limits_reads_the_rows_through_get_limits() {
    // ADR-0046 (d), `api.md` section 2.6: the limits fragment runs `GetLimits`, not `GetParameter`.
    // Every role holds `RecipeView`, so no answer tells the two queries apart. The test reads the source.
    // A checkout with `core.autocrlf=true` gives CRLF line ends, so the test reads lines.
    let source = include_str!("value.rs");
    let (_head, rest) = source.split_once("async fn limits_body").unwrap();
    let body: String = rest
        .lines()
        .take_while(|line| *line != "}")
        .collect::<Vec<_>>()
        .join("\n");
    assert!(body.contains("GetLimits::new"), "{body}");
    assert!(!body.contains("read_parameter"), "{body}");
}

#[tokio::test]
async fn parameters_limits_replaces_the_limits_and_shows_a_time_bound_as_a_literal() {
    let world = recipe_world();
    let pairs = [
        ("row_version", "1"),
        ("limits[0].leaf_path", ""),
        ("limits[0].min", "T#1s"),
        ("limits[0].max", "T#2m"),
    ];
    let reply = post_htmx(&world, &parameter_path(HOLD_KEY, "/limits"), &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    assert!(
        reply.body.contains("<td class=\"data\">T#1s</td>"),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains("<td class=\"data\">T#2m</td>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn parameters_limits_with_a_minimum_above_the_maximum_gives_e_rcp_018() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = [
        ("row_version", "1"),
        ("limits[0].leaf_path", ""),
        ("limits[0].min", "10"),
        ("limits[0].max", "5"),
    ];
    let reply = post_htmx(&world, &parameter_path(SPEED_KEY, "/limits"), &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-018");
    assert_eq!(world.fixture.snapshot(), before);
}
