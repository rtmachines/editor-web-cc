//! Tests of the struct form for the reserved names (ADR-0203) and the member count (ADR-0204) of
//! `CodesysIec` and `TwinCat3`.
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

#[tokio::test]
async fn adr_0204_struct_form_names_too_few_members() {
    let world = world();
    let (code, _id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    let before = world.fixture.snapshot();
    let body = form_body(Some("ST_One"), &[("value", "INT")], &[]);
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
    let reason = text(Lang::En, "type-reason-too-few-members");
    assert!(reply.body.contains(&reason), "{}", reply.body);
    assert_eq!(world.fixture.snapshot(), before, "nothing is written");
}

/// Return the part of an error answer that holds the out-of-band form of the struct.
fn swapped_form(body: &str) -> &str {
    body.split("<form id=\"udt-form\"").nth(1).unwrap()
}

/// Assert that the answer links its only line to `id` and puts the code under that input.
fn assert_line_under(body: &str, id: &str) {
    assert!(body.contains("Errors: 1."), "{body}");
    assert!(body.contains(&format!("href=\"#{id}\"")), "{id}: {body}");
    let form = swapped_form(body);
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 1, "{form}");
    assert_eq!(form.matches("class=\"field-error\"").count(), 1, "{form}");
    let line = format!("<div class=\"field-error\" id=\"{id}-error\">");
    let (_input, below) = form.split_once(&line).unwrap();
    assert!(below.contains("E-PLT-013"), "{id}: {form}");
}

/// ADR-0041 (e): the reserved name of a new struct stands under the input `type_name`.
#[tokio::test]
async fn adr_0203_reserved_struct_name_gets_its_line_under_the_name_input() {
    let world = world();
    let (code, _id) = seed_machine_on(&world.fixture, 8, PlcPlatform::TwinCat3);
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
    assert_line_under(&reply.body, "type_name");
}

/// ADR-0041 (e): the reserved member name stands under the name input of the first such row.
#[tokio::test]
async fn adr_0203_reserved_member_name_gets_its_line_under_the_name_input_of_the_row() {
    let world = world();
    let (code, _id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    let rows = [("speed", "INT"), ("END_IF", "BOOL"), ("then", "INT")];
    let body = form_body(Some("ST_Coil"), &rows, &[]);
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
    assert_line_under(&reply.body, "members-1-name");
}

/// ADR-0050 (d): `TooFewMembers` has no input. The summary line has no link and no input has a line.
#[tokio::test]
async fn adr_0204_too_few_members_names_no_input() {
    let world = world();
    let (code, _id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    let body = form_body(Some("ST_One"), &[("value", "INT")], &[]);
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
    assert!(
        !reply.body.contains("class=\"target-link\""),
        "{}",
        reply.body
    );
    let form = swapped_form(&reply.body);
    assert!(!form.contains("aria-invalid=\"true\""), "{form}");
    assert!(!form.contains("class=\"field-error\""), "{form}");
}

/// A refused referenced struct keeps its line under the type input of the member row. `type_expr`
/// names that struct and not the defined struct, so the rows of the defined struct get no line.
#[tokio::test]
async fn adr_0203_refused_referenced_struct_keeps_its_line_under_the_type_input() {
    let world = world();
    let (code, _id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    let rows = [("speed", "INT"), ("zone", "ST_Nope@1")];
    let body = form_body(Some("ST_Coil"), &rows, &[]);
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
    assert_line_under(&reply.body, "members-1-type");
}
