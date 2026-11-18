//! Test that `POST .../{key}/value` beside a stored S7 width defect answers `500 E-DB-008`
//! (ADR-0195).
//!
//! The draft gets one more parameter, a `DINT` at `DB9.DBW0`. The width rule of the earlier rule
//! set refuses that address. A value change of `SPEED` must not show it as a rule error.

use axum::http::StatusCode;
use recipe_app::memory::seed_type_id;
use recipe_domain::plant::{Primitive, S7Address};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, StoredParameter, StoredValue,
};
use serde_json::json;
use uuid::Uuid;

use super::test_kit::{assert_panel_error, parameter_path, post_htmx};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::capture_logs;
use crate::test_support::plant::DRAFT_ID;
use crate::test_support::recipes::SPEED_KEY;

/// The key of the seeded parameter `BROKEN`.
const BROKEN_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_00b0);

#[tokio::test]
async fn a_value_change_beside_a_stored_width_defect_gives_e_db_008_and_one_error_line() {
    let world = recipe_world();
    let version = recipe_domain::recipe::RecipeVersionId::from(DRAFT_ID);
    let speed = world
        .fixture
        .store
        .state()
        .parameters_of(version)
        .into_iter()
        .find(|row| row.tag.as_str() == "SPEED")
        .unwrap();
    let broken = StoredParameter {
        id: ParameterId::from(Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0000_00b0)),
        key: ParameterKey::from(BROKEN_KEY),
        tag: ParameterTag::parse("BROKEN").unwrap(),
        plc_type: seed_type_id(Primitive::Dint, None).unwrap(),
        s7_address: Some(S7Address::parse("DB9.DBW0").unwrap()),
        limits: Vec::new(),
        value: StoredValue::from(json!(5)),
        ordinal: Ordinal::parse(0x20).unwrap(),
        ..speed
    };
    world.fixture.store.seed_parameter(version, &broken);
    let (logs, _guard) = capture_logs();

    let reply = post_htmx(
        &world,
        &parameter_path(SPEED_KEY, "/value"),
        &[("row_version", "1"), ("value", "1500")],
    )
    .await;

    assert_panel_error(&reply, StatusCode::INTERNAL_SERVER_ERROR, "E-DB-008");
    let request_id = reply.header("x-request-id");
    assert!(!request_id.is_empty(), "{:?}", reply.headers);
    let reference = format!("Reference: {request_id}.");
    assert!(reply.body.contains(&reference), "{}", reply.body);
    let text = logs.text();
    let errors: Vec<&str> = text.lines().filter(|l| l.contains(" ERROR ")).collect();
    assert_eq!(errors.len(), 1, "{text}");
    let line = errors.first().unwrap();
    // The span prefix also prints `request_id`, so read only the event fields after the message.
    let (_span, event) = line.split_once("request failed ").unwrap();
    assert!(event.contains("error_code=E-DB-008"), "{line}");
    assert!(
        event.contains(&format!("request_id={request_id}")),
        "{line}"
    );
    let (before_chain, chain) = event.split_once("chain=").unwrap();
    assert!(!before_chain.contains(&BROKEN_KEY.to_string()), "{line}");
    assert!(chain.contains(&BROKEN_KEY.to_string()), "{line}");
}
