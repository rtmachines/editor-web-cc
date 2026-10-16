//! Tests of the error answers of `POST /recipes/{ref}/versions/{n}/rebind` (task 12.2).
//!
//! `api.md` section 2.5 (the error codes of the route) and section 5.3 (the HTMX headers). ADR-0043
//! (e) and (l), ADR-0041, design D9.

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;

use super::rebind_world::{
    REBIND, assert_htmx_answer, body_v3, form_id, oob_form, rebind_body, seed_big, summary_lines,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::plant::main_of;

#[tokio::test]
async fn rebind_with_a_bad_text_for_a_real_leaf_gives_an_e_rcp_005_line_under_its_input() {
    let world = recipe_world();
    let mut body = body_v3(&world, "1", &[2, 3, 4], "1");
    body.push_str(&format!(
        "&new_member_values[{}][1].temp_min=abc",
        crate::test_support::recipes::ZONES_KEY
    ));
    let reply = world.post(&world.engineer, REBIND, &body, true).await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-005");
    let form = oob_form(&reply.body, &form_id(world.ids.zone_v2));
    let line = "<div class=\"field-error\" id=\"new-0-0-error\">";
    assert!(form.contains(line), "{form}");
    assert!(form.contains("value=\"abc\""), "{form}");
}

/// RR1-1, ADR-0047 (a): the page of a refused rebind without HTMX holds a plain form.
///
/// The page has no `#parameter-table`, so the form has no `hx-*` attribute, and a second save
/// sends a normal `POST` also while HTMX runs.
#[tokio::test]
async fn rebind_refused_without_htmx_shows_a_plain_form_on_the_page() {
    let world = recipe_world();
    let mut body = body_v3(&world, "1", &[2, 3, 4], "1");
    body.push_str(&format!(
        "&new_member_values[{}][1].temp_min=abc",
        crate::test_support::recipes::ZONES_KEY
    ));
    let reply = world.post(&world.engineer, REBIND, &body, false).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.starts_with("<!DOCTYPE html>"), "{}", reply.body);
    let tag = format!("<form id=\"{}\"", form_id(world.ids.zone_v2));
    let (_before, form) = main_of(&reply.body).split_once(&tag).unwrap();
    let first_tag = form.split('>').next().unwrap();
    assert!(!first_tag.contains("hx-"), "{tag}{first_tag}>");
    assert!(!reply.body.contains("hx-target"), "{}", reply.body);
    assert!(form.contains("value=\"abc\""), "{form}");
}

#[tokio::test]
async fn rebind_of_an_operator_with_a_bad_field_gives_e_web_002_and_no_form() {
    // ADR-0013: the permission check comes before the check of the fields.
    let world = recipe_world();
    let before = world.fixture.snapshot();
    for body in ["row_version=1&bogus=1", "row_version=x"] {
        let events = world.fixture.sink.events().len();
        let reply = world.post(&world.operator, REBIND, body, true).await;
        assert_htmx_answer(&reply, StatusCode::FORBIDDEN, "E-WEB-002");
        let after = world.fixture.sink.events();
        assert_eq!(after.len(), events.checked_add(1).unwrap(), "{body}");
        assert!(
            after
                .last()
                .is_some_and(|event| event.kind() == SecurityEventKind::Forbidden),
            "{body}"
        );
        assert!(!reply.body.contains("<form"), "{body}: {}", reply.body);
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn rebind_on_a_version_that_is_not_a_draft_gives_e_rcp_002() {
    let world = recipe_world();
    let body = body_v3(&world, "1", &[1, 2, 3, 4], "1");
    let reply = world
        .post(
            &world.engineer,
            "/recipes/R-100/versions/2/rebind",
            &body,
            true,
        )
        .await;
    assert_htmx_answer(&reply, StatusCode::CONFLICT, "E-RCP-002");
}

#[tokio::test]
async fn rebind_to_the_source_revision_gives_e_rcp_052() {
    let world = recipe_world();
    let zone = world.ids.zone_v2;
    let reply = world
        .post(
            &world.engineer,
            REBIND,
            &rebind_body("1", (zone, zone), &[]),
            true,
        )
        .await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-052");
}

#[tokio::test]
async fn rebind_with_a_malformed_reference_gives_e_web_005() {
    let world = recipe_world();
    let body = body_v3(&world, "1", &[1, 2, 3, 4], "1");
    let reply = world
        .post(
            &world.engineer,
            "/recipes/-x/versions/3/rebind",
            &body,
            true,
        )
        .await;
    assert_htmx_answer(&reply, StatusCode::NOT_FOUND, "E-WEB-005");
}

#[tokio::test]
async fn rebind_above_the_leaf_limit_gives_e_plt_018() {
    let world = recipe_world();
    let (one, two) = seed_big(&world);
    let reply = world
        .post(
            &world.engineer,
            REBIND,
            &rebind_body("1", (one, two), &[]),
            true,
        )
        .await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-018");
}

#[tokio::test]
async fn rebind_on_a_deleted_recipe_gives_e_rcp_012() {
    let world = recipe_world();
    let body = body_v3(&world, "1", &[1, 2, 3, 4], "12.5");
    let reply = world
        .post(
            &world.engineer,
            "/recipes/R-300/versions/1/rebind",
            &body,
            true,
        )
        .await;
    assert_htmx_answer(&reply, StatusCode::CONFLICT, "E-RCP-012");
}

#[tokio::test]
async fn rebind_lists_every_bad_hidden_field_at_once_with_one_line_for_the_type_ids() {
    let world = recipe_world();
    let body = "row_version=x&from_type_id=bad&to_type_id=bad";
    let reply = world.post(&world.engineer, REBIND, body, true).await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(summary_lines(&reply.body), 2, "{}", reply.body);
}

#[tokio::test]
async fn rebind_refuses_a_type_id_that_is_not_in_the_hyphenated_form() {
    let world = recipe_world();
    let zone = world.ids.zone_v2.to_string().replace('-', "");
    let body = format!("row_version=1&from_type_id={zone}&to_type_id={zone}");
    let reply = world.post(&world.engineer, REBIND, &body, true).await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
}
