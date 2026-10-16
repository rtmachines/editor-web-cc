//! Tests of `GET .../parameters/{key}`: the parameter page (task 11.2).
//!
//! Specs "One edit form of a parameter" and "Leaf grid", design D7 and D8, ADR-0039 (i),
//! ADR-0043 (k).

use axum::http::StatusCode;

use super::test_kit::{parameter_path, position};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error, field_value};
use crate::test_support::plant::main_of;
use crate::test_support::recipes::{HOLD_KEY, LINE_A_KEY, SPEED_KEY};

#[tokio::test]
async fn parameters_page_of_a_draft_holds_one_edit_form_with_the_stored_value() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("Parameter SPEED</h1>"), "{main}");
    assert!(main.contains("<div id=\"parameter-summary\">"), "{main}");
    let tag = format!(
        "<form id=\"parameter-edit-form\" method=\"post\" action=\"{}\" hx-boost=\"true\" hx-target=\"#parameter-summary\" hx-swap=\"innerHTML show:none focus-scroll:true\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">",
        parameter_path(SPEED_KEY, "/definition")
    );
    assert_eq!(main.matches("<form id=\"parameter-").count(), 1, "{main}");
    let form = main.split(&tag).nth(1).unwrap();
    let save = position(form, "id=\"parameter-save-submit\"");
    assert!(save < position(form, "id=\"unit_path\""), "{form}");
    assert!(form.contains("id=\"value-0\" name=\"value\""), "{form}");
    assert!(form.contains("value=\"1200\""), "{form}");
    assert!(form.contains("value=\"3000\""), "{form}");
    assert_eq!(field_value(form, "value_type"), "INT");
    assert_eq!(field_value(form, "row_version"), "1");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn parameters_page_shows_the_leaves_of_a_struct_in_a_struct_and_a_time_literal() {
    let world = recipe_world();
    let line = world
        .get_en(&world.engineer, &parameter_path(LINE_A_KEY, ""))
        .await;
    for name in ["value.head.a", "value.head.b", "value.count"] {
        let field = format!("name=\"{name}\"");
        assert!(line.body.contains(&field), "{name}: {}", line.body);
    }
    let hold = world
        .get_en(&world.engineer, &parameter_path(HOLD_KEY, ""))
        .await;
    assert!(hold.body.contains("value=\"T#1m30s\""), "{}", hold.body);
}

#[tokio::test]
async fn parameters_page_of_an_operator_shows_the_grid_without_a_form() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.operator, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(!main.contains("<form"), "{main}");
    assert!(main.contains("<td class=\"data\">1200</td>"), "{main}");
    assert!(main.contains("<div id=\"parameter-summary\">"), "{main}");
}

#[tokio::test]
async fn parameters_page_of_a_missing_or_malformed_key_gives_e_web_005() {
    let world = recipe_world();
    let unknown = parameter_path(uuid::Uuid::from_u128(0xdead), "");
    for path in [unknown.as_str(), "/recipes/R-100/versions/3/parameters/x"] {
        let reply = world.get_en(&world.engineer, path).await;
        assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", path);
    }
}
