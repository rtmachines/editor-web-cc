//! Tests of the struct form for the reserved names of `CodesysIec` and `TwinCat3` (ADR-0203).
//!
//! The form parser does not know the platform. The use case refuses the name and gives
//! `E-PLT-013` with the reason text in the language of the session.

use axum::http::StatusCode;
use recipe_domain::plant::PlcPlatform;

use super::test_kit::{form_body, post_htmx, seed_machine_on};
use crate::i18n::Lang;
use crate::test_support::plant::world;
use crate::test_support::text;

#[tokio::test]
async fn adr_0203_struct_form_names_the_reserved_member_name() {
    let world = world();
    let (code, _id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    let before = world.fixture.snapshot();
    let body = form_body(Some("ST_Coil"), &[("END_IF", "BOOL"), ("S", "BOOL")], &[]);
    let reply = post_htmx(
        &world,
        &world.engineer,
        &format!("/machines/{code}/types"),
        &body,
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-PLT-013"), "{}", reply.body);
    let reason = text(Lang::En, "type-reason-reserved-member-name");
    assert!(reply.body.contains(&reason), "{}", reply.body);
    assert_eq!(world.fixture.snapshot(), before, "nothing is written");
}

#[tokio::test]
async fn adr_0203_struct_form_names_the_reserved_struct_name_on_twincat() {
    let world = world();
    let (code, _id) = seed_machine_on(&world.fixture, 8, PlcPlatform::TwinCat3);
    let before = world.fixture.snapshot();
    let body = form_body(Some("VERSION"), &[("major", "INT"), ("minor", "INT")], &[]);
    let reply = post_htmx(
        &world,
        &world.engineer,
        &format!("/machines/{code}/types"),
        &body,
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-PLT-013"), "{}", reply.body);
    let reason = text(Lang::En, "type-reason-reserved-name");
    assert!(reply.body.contains(&reason), "{}", reply.body);
    assert_eq!(world.fixture.snapshot(), before, "nothing is written");
}
