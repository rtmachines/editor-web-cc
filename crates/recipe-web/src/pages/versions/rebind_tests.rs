//! Tests of `POST /recipes/{ref}/versions/{n}/rebind`: success, lists, and form (task 12.2).
//!
//! Specs "Rebind form of an outdated draft" and "Texts of the new members of a rebind". ADR-0043
//! (l), ADR-0041, ADR-0039 (e) and (f), ADR-0008, design D6, D7, D9, and D10.

use axum::http::StatusCode;
use recipe_domain::plant::Primitive;
use recipe_domain::recipe::ParameterKey;

use super::rebind_world::{
    DRAFT_PAGE, GATE_KEY, REBIND, ZONE_ONE_KEY, assert_htmx_answer, body_v3, form_id, oob_form,
    rebind_body, seed_gate, seed_zone_one, seed_zone_v4, summary_lines, zones_fields,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::plant::World;
use crate::test_support::recipes::LINE_KEY;

/// The four elements of `ZONES`.
const ALL: [u32; 4] = [1, 2, 3, 4];

/// Seed `ST_Zone@4` with `members` and `ZONE_ONE`, and return the body of the rebind to it.
///
/// `ZONE_ONE` has a limit on `.temp_max`, so the rebind gives `E-RCP-029` when `ST_Zone@4` drops
/// the member. The body sends each new leaf of `ZONE_ONE` and `ZONES` that `with_min` names.
fn zone_v4_body(world: &World, members: &[(&str, Primitive)], with_min: bool) -> String {
    let zone_v4 = seed_zone_v4(world, members);
    seed_zone_one(world);
    let mut fields = Vec::new();
    if with_min {
        fields.push((ZONE_ONE_KEY, ".temp_min".to_owned(), "1"));
        fields.extend(zones_fields(&ALL, "1"));
    }
    rebind_body("1", (world.ids.zone_v2, zone_v4), &fields)
}

#[tokio::test]
async fn rebind_moves_zones_and_answers_the_table_the_outdated_list_and_the_toast() {
    let world = recipe_world();
    let body = body_v3(&world, "1", &ALL, "+12.5+");
    let reply = world.post(&world.engineer, REBIND, &body, true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    assert!(
        reply.body.contains("<tbody id=\"parameter-rows\">"),
        "{}",
        reply.body
    );
    let row = "<td class=\"data\">ARRAY[1..4] OF ST_Zone@3</td>";
    assert!(reply.body.contains(row), "{}", reply.body);
    let (_table, list) = reply
        .body
        .split_once("<div id=\"outdated-list\" hx-swap-oob=\"true\">")
        .unwrap();
    let list = list.split("<div id=\"toast\"").next().unwrap();
    assert!(list.contains("\">LINE</a>"), "{list}");
    assert!(!list.contains("\">ZONES</a>"), "{list}");
    assert!(!list.contains("<form"), "{list}");
    let toast = "<div id=\"toast\" hx-swap-oob=\"innerHTML\">";
    assert!(reply.body.contains(toast), "{}", reply.body);
    let done = "The parameters use ST_Zone@3.";
    assert!(reply.body.contains(done), "{}", reply.body);
    let panel = "<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>";
    assert!(reply.body.contains(panel), "{}", reply.body);
}

#[tokio::test]
async fn rebind_without_htmx_goes_to_the_version_page() {
    let world = recipe_world();
    let body = body_v3(&world, "1", &ALL, "12.5");
    let reply = world.post(&world.engineer, REBIND, &body, false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), DRAFT_PAGE);
}

#[tokio::test]
async fn rebind_without_the_field_of_a_new_member_gives_one_e_rcp_028_line_under_its_input() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let body = body_v3(&world, "1", &[1, 2, 3], "12.5");
    let reply = world.post(&world.engineer, REBIND, &body, true).await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-028");
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    assert!(reply.body.contains("href=\"#new-0-3\""), "{}", reply.body);
    let form = oob_form(&reply.body, &form_id(world.ids.zone_v2));
    assert!(form.starts_with(" method=\"post\""), "{form}");
    assert!(form.contains(" hx-swap-oob=\"true\">"), "{form}");
    let line = "<div class=\"field-error\" id=\"new-0-3-error\">";
    assert!(form.contains(line), "{form}");
    assert!(!form.contains("id=\"new-0-2-error\""), "{form}");
    assert!(form.contains("value=\"12.5\""), "{form}");
    assert!(form.contains("name=\"row_version\" value=\"1\""), "{form}");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn rebind_of_a_member_with_a_limit_gives_an_e_rcp_029_line_under_the_first_input() {
    let world = recipe_world();
    let members = [("speed", Primitive::Int), ("temp_min", Primitive::Real)];
    let body = zone_v4_body(&world, &members, true);
    let before = world.fixture.snapshot();
    let reply = world.post(&world.engineer, REBIND, &body, true).await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-029");
    assert!(reply.body.contains("href=\"#new-0-0\""), "{}", reply.body);
    let form = oob_form(&reply.body, &form_id(world.ids.zone_v2));
    let line = "<div class=\"field-error\" id=\"new-0-0-error\">";
    assert!(form.contains(line), "{form}");
    assert!(form.contains("E-RCP-029"), "{form}");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn rebind_line_of_a_parameter_without_an_input_stands_under_its_fieldset() {
    let world = recipe_world();
    let body = zone_v4_body(&world, &[("speed", Primitive::Int)], false);
    let reply = world.post(&world.engineer, REBIND, &body, true).await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-029");
    assert!(
        reply.body.contains("href=\"#rebind-parameter-0\""),
        "{}",
        reply.body
    );
    let form = oob_form(&reply.body, &form_id(world.ids.zone_v2));
    let fieldset =
        "<fieldset id=\"rebind-parameter-0\" aria-describedby=\"rebind-parameter-0-error\">";
    assert!(form.contains(fieldset), "{form}");
    let line = "<div class=\"field-error\" id=\"rebind-parameter-0-error\">";
    assert!(form.contains(line), "{form}");
}

#[tokio::test]
async fn rebind_form_keeps_its_id_when_a_new_group_sorts_first() {
    let world = recipe_world();
    seed_gate(&world);
    let body = body_v3(&world, "1", &[1, 2, 3], "12.5");
    let reply = world.post(&world.engineer, REBIND, &body, true).await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-028");
    let id = form_id(world.ids.zone_v2);
    assert!(
        reply.body.contains(&format!("<form id=\"{id}\"")),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn rebind_refuses_a_member_field_that_the_form_does_not_hold() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let mut body = body_v3(&world, "1", &ALL, "12.5");
    body.push_str(&format!(
        "&new_member_values[{LINE_KEY}].zones[1].temp_min=1"
    ));
    let reply = world.post(&world.engineer, REBIND, &body, true).await;
    assert_htmx_answer(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn rebind_stores_the_text_of_a_new_string_member_unchanged() {
    let world = recipe_world();
    let (one, two) = seed_gate(&world);
    let fields = [(GATE_KEY, ".label".to_owned(), "++a++")];
    let reply = world
        .post(
            &world.engineer,
            REBIND,
            &rebind_body("1", (one, two), &fields),
            true,
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let gate = world
        .fixture
        .store
        .state()
        .parameters_of(world.ids.draft)
        .into_iter()
        .find(|parameter| parameter.key == ParameterKey::from(GATE_KEY))
        .unwrap();
    let stored = gate.value.as_json().to_string();
    assert!(stored.contains("\"label\":\"  a  \""), "{stored}");
}

#[tokio::test]
async fn rebind_with_an_old_row_version_from_a_second_session_gives_the_conflict() {
    let world = recipe_world();
    let body = body_v3(&world, "1", &ALL, "12.5");
    let first = world.post(&world.engineer, REBIND, &body, true).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let second = world.post(&world.admin, REBIND, &body, true).await;
    assert_htmx_answer(&second, StatusCode::CONFLICT, "E-WEB-001");
    assert!(second.body.contains("user2"), "{}", second.body);
    assert!(!second.body.contains("<form"), "{}", second.body);
    assert_eq!(
        second.body.matches("hx-swap-oob").count(),
        1,
        "{}",
        second.body
    );
    assert!(
        second
            .body
            .ends_with("<div id=\"toast\" hx-swap-oob=\"innerHTML\"></div>"),
        "{}",
        second.body
    );
}
