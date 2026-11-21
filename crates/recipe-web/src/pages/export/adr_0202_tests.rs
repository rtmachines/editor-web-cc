//! Tests of the `loadable` flag in the bytes of the export routes for a `Utf8` machine
//! (ADR-0202 (k), (l)).
//!
//! The data is the approved version 1 of `R-300` on the `Utf8` machine `M10`. The version breaks no
//! current rule. A v1 document is not loadable, because the v1 schema has no field for the
//! encoding. The v2 document of a unit names the encoding and stays loadable. Each test uses the
//! production writer and the credentials of its route (`api.md` section 2.7).

use axum::http::StatusCode;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use recipe_domain::identity::Role;

use super::tests::{export, field};
use crate::test_support::plant::World;
use crate::test_support::recipes::utf8_world::{APPROVED_REFERENCE, utf8_approved_world};
use crate::test_support::{Reply, get_with};

/// The login of the `ExportClient` account of the machine `M10`.
const CLIENT_LOGIN: &str = "plc-m10";
/// The password of that account.
const CLIENT_PASSWORD: &str = "tr4nsfer-s3cret";
/// The number of that account. The numbers 1 to 3 belong to the sessions of the world.
const CLIENT_NUMBER: u8 = 9;

/// Build the world and seed the `ExportClient` account `plc-m10`.
async fn world_with_client() -> World {
    let world = utf8_approved_world().await;
    world.fixture.seed_account_as(
        CLIENT_LOGIN,
        CLIENT_NUMBER,
        (Role::ExportClient, CLIENT_PASSWORD),
        false,
    );
    world
}

/// Send `GET path` as the account `plc-m10` with HTTP Basic.
async fn client_get(world: &World, path: &str) -> Reply {
    let header = format!(
        "Basic {}",
        STANDARD.encode(format!("{CLIENT_LOGIN}:{CLIENT_PASSWORD}"))
    );
    get_with(&world.fixture.router(), path, &[("authorization", &header)]).await
}

/// Assert `200` with a JSON body and return the value of `loadable` in it.
fn loadable_of(reply: &Reply) -> &str {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json");
    field(&reply.body, "loadable")
}

#[tokio::test]
async fn adr_0202_export_bytes_v1_approved_route_of_a_utf8_machine_is_not_loadable() {
    let world = world_with_client().await;
    let path = format!("/export/v1/recipes/{APPROVED_REFERENCE}/approved");

    let reply = export(&world.fixture, &world.operator, &path).await;

    assert_eq!(loadable_of(&reply), "false");
    assert_eq!(field(&reply.body, "schema_version"), "\"1\"");
    assert!(!reply.body.contains("string_encoding"));
}

#[tokio::test]
async fn adr_0202_export_bytes_v1_version_route_of_a_utf8_machine_is_not_loadable() {
    let world = world_with_client().await;
    let path = format!("/export/v1/recipes/{APPROVED_REFERENCE}/versions/1");

    let reply = export(&world.fixture, &world.engineer, &path).await;

    assert_eq!(loadable_of(&reply), "false");
    assert_eq!(field(&reply.body, "status"), "\"Approved\"");
    assert!(!reply.body.contains("string_encoding"));
}

#[tokio::test]
async fn adr_0202_export_bytes_v1_machine_route_of_a_utf8_machine_is_not_loadable() {
    let world = world_with_client().await;
    let path = format!("/export/v1/machines/M10/recipes/{APPROVED_REFERENCE}/approved");

    let reply = client_get(&world, &path).await;

    assert_eq!(loadable_of(&reply), "false");
    assert!(!reply.body.contains("string_encoding"));
}

#[tokio::test]
async fn adr_0202_export_bytes_v2_unit_route_of_a_utf8_machine_keeps_loadable() {
    let world = world_with_client().await;
    let path = format!("/export/v2/machines/M10/recipes/{APPROVED_REFERENCE}/approved/units/U1");

    let reply = client_get(&world, &path).await;

    assert_eq!(loadable_of(&reply), "true");
    assert_eq!(field(&reply.body, "schema_version"), "\"2\"");
    assert_eq!(field(&reply.body, "string_encoding"), "\"Utf8\"");
}
