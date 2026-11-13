//! Tests of the stored struct with a reserved name (ADR-0143 (i1), design D3).
//!
//! A struct such as `Int@1` was legal under the earlier rule set. The type page serves it. The
//! form of a new struct and the form of a new revision refuse a reserved name.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use recipe_domain::plant::{
    MemberName, PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape, RuleSet, TypeMember,
    TypeName,
};

use super::test_kit::{form_body, get_as, post_htmx, primitive_id, struct_id};
use crate::i18n::Lang;
use crate::test_support::plant::{World, world};
use crate::test_support::text;

const NEW: &str = "/machines/M7/types";
const INT_PAGE: &str = "/machines/M7/types/Int";
const INT_REVISIONS: &str = "/machines/M7/types/Int/revisions";

/// Seed the struct `Int@1` of `M7` with the member `speed` of type `INT`.
///
/// `PlcType::from_stored` with the earlier rule set builds the row, as the database read does.
fn seed_reserved_struct(world: &World) {
    let row = PlcType::from_stored(
        PlcTypeRecord {
            id: struct_id(0x40),
            machine: Some(world.ids.m7),
            platforms: BTreeSet::from([PlcPlatform::S7_300]),
            shape: RecordShape::Struct {
                name: TypeName::from_stored("Int", RuleSet::Earlier).unwrap(),
                revision: 1,
                supersedes: None,
                members: vec![TypeMember {
                    name: MemberName::parse("speed").unwrap(),
                    type_id: primitive_id(Primitive::Int),
                }],
            },
        },
        RuleSet::Earlier,
    )
    .unwrap();
    world.fixture.store.seed_type(&row);
}

#[tokio::test]
async fn types_reserved_name_the_type_page_of_a_stored_int_struct_shows_its_members() {
    let world = world();
    seed_reserved_struct(&world);
    let reply = get_as(&world, &world.operator, INT_PAGE, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("<h1>Type Int@1</h1>"), "{}", reply.body);
    assert!(reply.body.contains("speed"), "{}", reply.body);
}

#[tokio::test]
async fn types_reserved_name_the_form_of_a_new_struct_refuses_a_reserved_name() {
    let world = world();
    for name in ["Int", "real", "ARRAY", "OF", "STRUCT", "end_struct"] {
        let before = world.fixture.snapshot();
        let body = form_body(Some(name), &[("speed", "INT")], &[]);
        let reply = post_htmx(&world, &world.engineer, NEW, &body).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{name}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-008"), "{name}: {}", reply.body);
        assert!(
            reply.body.contains("href=\"#type_name\""),
            "{name}: {}",
            reply.body
        );
        assert_eq!(
            world.fixture.snapshot(),
            before,
            "{name} must write nothing"
        );
    }
}

#[tokio::test]
async fn types_reserved_name_the_form_of_a_new_revision_of_int_gives_e_plt_013_reserved_name() {
    let world = world();
    seed_reserved_struct(&world);
    let before = world.fixture.snapshot();
    // The form adds one member, so the revision differs from `Int@1` and E-PLT-052 does not apply.
    let body = form_body(None, &[("speed", "INT"), ("torque", "INT")], &[]);
    let reply = post_htmx(&world, &world.engineer, INT_REVISIONS, &body).await;
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
