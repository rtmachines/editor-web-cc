//! Test of the vendor alias of a member type in the struct form (ADR-0206 (b)).
//!
//! The struct form parses each member type with `TypeExpr::parse`. So `TOD` stores the catalog
//! name `TIME_OF_DAY`, and the page of the struct shows that name.

use axum::http::StatusCode;
use recipe_domain::plant::PlcPlatform;

use super::test_kit::{form_body, get_as, post_htmx, seed_machine_on};
use crate::test_support::plant::world;

#[tokio::test]
async fn adr_0206_struct_form_stores_the_catalog_name_of_an_alias_member_type() {
    let world = world();
    world.fixture.store.seed_time_primitive_types();
    let (code, _id) = seed_machine_on(&world.fixture, 8, PlcPlatform::TwinCat3);
    let body = form_body(Some("ST_Shift"), &[("start", "TOD"), ("count", "INT")], &[]);

    let reply = post_htmx(
        &world,
        &world.engineer,
        &format!("/machines/{code}/types"),
        &body,
    )
    .await;

    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    let page = get_as(
        &world,
        &world.engineer,
        &format!("/machines/{code}/types/ST_Shift"),
        &[],
    )
    .await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
    // The page of a struct shows only its tree. So the one member `start` gives the one text.
    assert_eq!(page.body.matches("TIME_OF_DAY").count(), 1, "{}", page.body);
    assert!(!page.body.contains("TOD"), "{}", page.body);
}
