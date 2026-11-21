//! Tests of the vendor aliases of type names in the parameter forms (ADR-0206 (b), (d1), (d3)).
//!
//! The parameter forms parse the field `type` with the current rule set first, so `TOD` gives
//! `TIME_OF_DAY`. After a refusal they parse it again with the earlier rule set, because an edit
//! form sends a stored reference such as `Int@1` again. The form query checks the reference.

use axum::http::StatusCode;
use recipe_app::memory::seed_type_id;
use recipe_domain::plant::{PlcTypeId, Primitive, RuleSet, TypeExpr};
use recipe_domain::recipe::RecipeVersionId;

use super::edit_tests::ZONE_FIELDS;
use super::test_kit::{
    ADD, INT_FIELDS, SPEED_FIELDS, assert_panel_error, changed, parameter_path, post_htmx,
};
use crate::i18n::Lang;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::field_value;
use crate::test_support::plant::{DRAFT_ID, World};
use crate::test_support::recipes::SPEED_KEY;
use crate::test_support::recipes::earlier_name_world::{ZONE_KEY, seed_int_named_parameter};
use crate::test_support::text;

/// The world of the draft 3 of `R-100` on `M7`, with the rows of the time primitives.
fn time_world() -> World {
    let world = recipe_world();
    world.fixture.store.seed_time_primitive_types();
    world
}

/// The type of the stored parameter `tag` of the draft 3 of `R-100`.
fn stored_type(world: &World, tag: &str) -> PlcTypeId {
    world
        .fixture
        .store
        .state()
        .parameters_of(RecipeVersionId::from(DRAFT_ID))
        .into_iter()
        .find(|row| row.tag.as_str() == tag)
        .unwrap()
        .plc_type
}

/// The fields of a valid new `TIME_OF_DAY` parameter `NEW_P`, with the type text `type_text`.
fn tod_fields<'a>(type_text: &'a str, value_type: &'a str) -> Vec<(&'a str, &'a str)> {
    changed(
        &INT_FIELDS,
        &[
            ("type", Some(type_text)),
            ("value_type", Some(value_type)),
            ("s7_address", Some("DB9.DBD0")),
            ("value", Some("TOD#06:00:00")),
        ],
    )
}

#[tokio::test]
async fn adr_0206_preview_of_the_add_form_with_tod_gives_time_of_day() {
    let world = time_world();
    let before = world.fixture.snapshot();
    let pairs = changed(
        &INT_FIELDS,
        &[
            ("type", Some("TOD")),
            ("value_type", None),
            ("value", None),
            ("fields", Some("1")),
        ],
    );

    let reply = post_htmx(&world, ADD, &pairs).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(field_value(&reply.body, "value_type"), "TIME_OF_DAY");
    assert_eq!(
        world.fixture.snapshot(),
        before,
        "the preview writes nothing"
    );
}

#[tokio::test]
async fn adr_0206_add_with_tod_stores_time_of_day() {
    let world = time_world();

    let reply = post_htmx(&world, ADD, &tod_fields("TOD", "TIME_OF_DAY")).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let time_of_day = seed_type_id(Primitive::TimeOfDay, None).unwrap();
    assert_eq!(stored_type(&world, "NEW_P"), time_of_day);
}

#[tokio::test]
async fn adr_0206_add_with_tod_in_the_field_value_type_stores_time_of_day() {
    let world = time_world();

    let reply = post_htmx(&world, ADD, &tod_fields("TOD", "TOD")).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let time_of_day = seed_type_id(Primitive::TimeOfDay, None).unwrap();
    assert_eq!(stored_type(&world, "NEW_P"), time_of_day);
}

#[tokio::test]
async fn adr_0206_preview_of_the_add_form_with_an_alias_as_array_element() {
    let world = time_world();
    let before = world.fixture.snapshot();
    let pairs = changed(
        &INT_FIELDS,
        &[
            ("type", Some("ARRAY[1..2] OF TOD")),
            ("value_type", None),
            ("value", None),
            ("fields", Some("1")),
        ],
    );

    let reply = post_htmx(&world, ADD, &pairs).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(
        field_value(&reply.body, "value_type"),
        "ARRAY[1..2] OF TIME_OF_DAY"
    );
    assert_eq!(
        world.fixture.snapshot(),
        before,
        "the preview writes nothing"
    );
}

#[tokio::test]
async fn adr_0206_edit_with_a_changed_type_tod_shows_the_fields_of_time_of_day() {
    let world = time_world();
    let before = world.fixture.snapshot();
    let pairs = changed(&SPEED_FIELDS, &[("type", Some("TOD"))]);

    let reply = post_htmx(&world, &parameter_path(SPEED_KEY, "/definition"), &pairs).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(field_value(&reply.body, "value_type"), "TIME_OF_DAY");
    assert_eq!(
        world.fixture.snapshot(),
        before,
        "the changed type writes nothing"
    );
}

#[tokio::test]
async fn adr_0206_edit_with_tod_stores_time_of_day() {
    let world = time_world();
    let pairs = changed(
        &SPEED_FIELDS,
        &[
            ("type", Some("TOD")),
            ("value_type", Some("TOD")),
            ("s7_address", Some("DB9.DBD8")),
            ("value", Some("TOD#06:00:00")),
            ("limits[0].leaf_path", None),
            ("limits[0].min", None),
            ("limits[0].max", None),
        ],
    );

    let reply = post_htmx(&world, &parameter_path(SPEED_KEY, "/definition"), &pairs).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let time_of_day = seed_type_id(Primitive::TimeOfDay, None).unwrap();
    assert_eq!(stored_type(&world, "SPEED"), time_of_day);
}

#[tokio::test]
async fn adr_0206_edit_with_the_unchanged_stored_reference_int_at_1_keeps_it() {
    let world = recipe_world();
    seed_int_named_parameter(&world.fixture);
    let stored = stored_type(&world, "ZONE");
    let path = parameter_path(ZONE_KEY, "/definition");

    let preview = post_htmx(
        &world,
        &path,
        &changed(&ZONE_FIELDS, &[("fields", Some("1"))]),
    )
    .await;
    let saved = post_htmx(
        &world,
        &path,
        &changed(&ZONE_FIELDS, &[("name_en", Some("Zone"))]),
    )
    .await;

    assert_eq!(preview.status, StatusCode::OK, "{}", preview.body);
    assert!(
        preview.body.contains("name=\"value.speed\""),
        "{}",
        preview.body
    );
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    assert_eq!(stored_type(&world, "ZONE"), stored);
}

/// Assert that an answer shows `E-PLT-013` with the reason "reserved name", and not the reason
/// "unknown struct".
fn assert_reserved_name(reply: &crate::test_support::Reply) {
    assert_panel_error(reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-013");
    let reserved = text(Lang::En, "type-reason-reserved-name");
    let unknown = text(Lang::En, "type-reason-unknown-struct");
    assert!(reply.body.contains(&reserved), "{}", reply.body);
    assert!(!reply.body.contains(&unknown), "{}", reply.body);
}

#[tokio::test]
async fn adr_0206_preview_of_the_add_form_with_dt_at_1_gives_reserved_name() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = changed(
        &INT_FIELDS,
        &[
            ("type", Some("DT@1")),
            ("value_type", None),
            ("value", None),
            ("fields", Some("1")),
        ],
    );

    let reply = post_htmx(&world, ADD, &pairs).await;

    assert_reserved_name(&reply);
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn adr_0206_save_of_the_add_form_with_dt_at_1_gives_reserved_name() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = changed(&INT_FIELDS, &[("type", Some("DT@1")), ("value_type", None)]);

    let reply = post_htmx(&world, ADD, &pairs).await;

    assert_reserved_name(&reply);
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn adr_0206_preview_of_the_add_form_with_int_at_1_gives_unknown_struct() {
    let world = recipe_world();
    let pairs = changed(
        &INT_FIELDS,
        &[
            ("type", Some("INT@1")),
            ("value_type", None),
            ("value", None),
            ("fields", Some("1")),
        ],
    );

    let reply = post_htmx(&world, ADD, &pairs).await;

    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-013");
    let unknown = text(Lang::En, "type-reason-unknown-struct");
    assert!(reply.body.contains(&unknown), "{}", reply.body);
}

#[test]
fn adr_0206_a_stored_read_of_the_alias_text_stays_a_grammar_error() {
    // The stored reads keep `from_stored(.., RuleSet::Earlier)`, so a stored row with the type text
    // `TOD` is a corrupt row. Only the request parser maps the alias.
    assert!(TypeExpr::from_stored("TOD", RuleSet::Earlier).is_err());
    assert_eq!(TypeExpr::parse("TOD").unwrap().to_string(), "TIME_OF_DAY");
}
