//! Tests of the struct form for the reserved names (ADR-0203) and the member count (ADR-0204) of
//! `CodesysIec` and `TwinCat3`.
//!
//! The form parser does not know the platform. The use case refuses the name and gives
//! `E-PLT-013` with the reason text in the language of the session.

use axum::http::StatusCode;
use recipe_domain::plant::{MachineId, PlcPlatform, PlcTypeId, Primitive};

use super::test_kit::{form_body, post_htmx, primitive_id, seed_machine_on, seed_struct_for};
use crate::i18n::Lang;
use crate::test_support::plant::{World, world};
use crate::test_support::{Reply, text};

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

/// Post the struct form `ST_Coil` with the member rows `rows` to the machine `code`.
async fn post_coil(world: &World, code: impl std::fmt::Display, rows: &[(&str, &str)]) -> Reply {
    let body = form_body(Some("ST_Coil"), rows, &[]);
    post_htmx(
        world,
        &world.engineer,
        &format!("/machines/{code}/types"),
        &body,
    )
    .await
}

/// Assert that the answer is `422` and that its line has no link and no input has a line.
fn assert_no_input_named(reply: &Reply) {
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

/// Seed the stored struct `name@revision` of the machine `id` with the members `members`, each
/// of type `INT`.
fn seed_int_struct(
    world: &World,
    (id, platform): (MachineId, PlcPlatform),
    (number, name, revision): (u128, &str, u32),
    members: &[&str],
) {
    let typed: Vec<(&str, PlcTypeId)> = members
        .iter()
        .map(|member| (*member, primitive_id(Primitive::Int)))
        .collect();
    seed_struct_for(world, (id, platform), number, (name, revision), &typed);
}

/// ADR-0041 (e), ADR-0203 (i1): a member type that names a stored struct with a reserved name gets
/// the line under its type input, although `type_expr` is the bare struct name.
#[tokio::test]
async fn adr_0203_referenced_struct_with_reserved_name_gets_its_line_under_the_type_input() {
    let world = world();
    let (code, id) = seed_machine_on(&world.fixture, 8, PlcPlatform::TwinCat3);
    seed_int_struct(
        &world,
        (id, PlcPlatform::TwinCat3),
        (1, "VERSION", 1),
        &["a", "b"],
    );
    let reply = post_coil(&world, code, &[("speed", "INT"), ("zone", "VERSION@1")]).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_line_under(&reply.body, "members-1-type");
    let reason = text(Lang::En, "type-reason-reserved-name");
    assert!(reply.body.contains(&reason), "{}", reply.body);
}

/// ADR-0041 (e), ADR-0203 (i1): the stored struct has a reserved member name and the member type is
/// an array of it.
#[tokio::test]
async fn adr_0203_referenced_struct_with_reserved_member_name_in_an_array_gets_its_line() {
    let world = world();
    let (code, id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    seed_int_struct(
        &world,
        (id, PlcPlatform::CodesysIec),
        (1, "ST_x", 1),
        &["END_IF", "b"],
    );
    let rows = [("speed", "INT"), ("zones", "ARRAY[0..3] OF ST_x@1")];
    let reply = post_coil(&world, code, &rows).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_line_under(&reply.body, "members-1-type");
    let reason = text(Lang::En, "type-reason-reserved-member-name");
    assert!(reply.body.contains(&reason), "{}", reply.body);
}

/// ADR-0041 (e), ADR-0204 (c1): the stored struct has one member on `CodesysIec`.
#[tokio::test]
async fn adr_0204_referenced_struct_with_one_member_gets_its_line_under_the_type_input() {
    let world = world();
    let (code, id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    seed_int_struct(
        &world,
        (id, PlcPlatform::CodesysIec),
        (1, "ST_x", 1),
        &["only"],
    );
    let reply = post_coil(&world, code, &[("speed", "INT"), ("zone", "ST_x@1")]).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_line_under(&reply.body, "members-1-type");
    let reason = text(Lang::En, "type-reason-too-few-members");
    assert!(reply.body.contains(&reason), "{}", reply.body);
}

/// ADR-0041 (e), ADR-0186 (f): on an S7 platform, a member type that names a struct with a Siemens
/// reserved name gets the line under its type input.
#[tokio::test]
async fn adr_0186_referenced_struct_with_siemens_reserved_name_gets_its_line_under_the_type_input()
{
    let world = world();
    let (code, id) = seed_machine_on(&world.fixture, 7, PlcPlatform::S7_300);
    seed_int_struct(
        &world,
        (id, PlcPlatform::S7_300),
        (1, "AXIS_REF", 1),
        &["a", "b"],
    );
    let reply = post_coil(&world, code, &[("speed", "INT"), ("axis", "AXIS_REF@1")]).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_line_under(&reply.body, "members-1-type");
    let reason = text(Lang::En, "type-reason-reserved-name");
    assert!(reply.body.contains(&reason), "{}", reply.body);
}

/// ADR-0050 (d): two members name two revisions of one struct and the web layer cannot see which
/// revision is refused, so no input gets a line.
#[tokio::test]
async fn adr_0203_two_revisions_of_the_refused_struct_name_no_input() {
    let world = world();
    let (code, id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    seed_int_struct(
        &world,
        (id, PlcPlatform::CodesysIec),
        (1, "ST_x", 1),
        &["a", "b"],
    );
    seed_int_struct(
        &world,
        (id, PlcPlatform::CodesysIec),
        (2, "ST_x", 2),
        &["END_IF", "b"],
    );
    let rows = [("old", "ST_x@1"), ("new", "ST_x@2")];
    let reply = post_coil(&world, code, &rows).await;
    assert_no_input_named(&reply);
}

/// ADR-0050 (d): two members name the same refused revision, so the line goes under the first row.
#[tokio::test]
async fn adr_0203_two_references_to_one_refused_revision_name_the_first_row() {
    let world = world();
    let (code, id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    seed_int_struct(
        &world,
        (id, PlcPlatform::CodesysIec),
        (1, "ST_x", 1),
        &["END_IF", "b"],
    );
    let rows = [
        ("speed", "INT"),
        ("zone", "ST_x@1"),
        ("zones", "ARRAY[0..1] OF ST_x@1"),
    ];
    let reply = post_coil(&world, code, &rows).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_line_under(&reply.body, "members-1-type");
}

/// ADR-0050 (d): a revision form whose member names a refused older revision of the defined struct
/// names no wrong input. The arm of the defined struct finds no reserved member name in the rows.
#[tokio::test]
async fn adr_0203_revision_that_names_a_refused_older_revision_of_itself_names_no_input() {
    let world = world();
    let (code, id) = seed_machine_on(&world.fixture, 9, PlcPlatform::CodesysIec);
    seed_int_struct(
        &world,
        (id, PlcPlatform::CodesysIec),
        (1, "ST_x", 1),
        &["END_IF", "b"],
    );
    let body = form_body(None, &[("a", "INT"), ("old", "ST_x@1")], &[]);
    let reply = post_htmx(
        &world,
        &world.engineer,
        &format!("/machines/{code}/types/ST_x/revisions"),
        &body,
    )
    .await;
    assert_no_input_named(&reply);
}
