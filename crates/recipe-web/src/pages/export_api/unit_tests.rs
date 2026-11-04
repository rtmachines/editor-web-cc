//! Tests of the unit scope route of the transfer client (design D5, ADR-0109 (w) to (ab)).
//!
//! The data is `R-800` on `M7` (`seed_unit_recipe`). Its approved version 1 holds `SPEED` on
//! `U1`, `ZONE1` on `U1.2`, `ZONE2` on `U1.2.4`, and `FEED` on `U1.20`. The unit `U1.3` holds no
//! parameter. The `ExportClient` account `user1` sends every request with HTTP Basic.

use axum::body::Body;
use axum::http::{Method, StatusCode};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use recipe_domain::plant::{UnitId, UnitPath, UnitUsage};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::tests::{
    affected_world, audit_count, client, client_get, exported_actors, field, seeded,
};
use crate::test_support::recipes::approval::ApprovalData;
use crate::test_support::{Fixture, call, request};

/// The unit route of `R-800` on `M7`, without the unit path.
const UNITS: &str = "/export/v2/machines/M7/recipes/R-800/approved/units";
/// The identifier of the unit `U1.21` of `M7`.
const SPARE_UNIT_ID: Uuid = Uuid::from_u128(0x7357_0002_0007_0000_0000_0000_0000_0021);
/// The instant of the archive of `U1.21` in Unix seconds.
const ARCHIVED_AT_SECONDS: i64 = 1_000_000;

/// Build a fixture with the account `user1`, the approval data, and the recipe `R-800`.
async fn seeded_units() -> (Fixture, ApprovalData) {
    let (fixture, data) = seeded().await;
    fixture.seed_unit_recipe(&data).await;
    (fixture, data)
}

/// Return the path of the unit route for `unit`.
fn unit_route(unit: &str) -> String {
    format!("{UNITS}/{unit}")
}

/// Return true when the document holds a parameter with the tag `tag`.
fn has_parameter(body: &str, tag: &str) -> bool {
    body.contains(&format!("\"tag\": \"{tag}\""))
}

/// Return true when the document holds a type with the name `name`.
fn has_type(body: &str, name: &str) -> bool {
    body.contains(&format!("\"name\": \"{name}\""))
}

/// Store the archived unit `U1.21` below `U1` of `M7`, so `M7` stays live.
fn archive_spare_unit(fixture: &Fixture, data: &ApprovalData) {
    let mut machine = fixture
        .store
        .state()
        .machine(data.recipes.plant.m7)
        .unwrap();
    let parent = UnitPath::parse("U1").unwrap();
    let spare = UnitPath::parse("U1.21").unwrap();
    machine
        .add_unit(
            UnitId::from(SPARE_UNIT_ID),
            Some(&parent),
            "21",
            Text::parse("Spare unit").unwrap(),
        )
        .unwrap();
    let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(ARCHIVED_AT_SECONDS).unwrap());
    machine
        .archive_unit(&spare, at, &UnitUsage::default())
        .unwrap();
    fixture.store.seed_machine(&machine);
}

#[tokio::test]
async fn export_unit_of_a_version_with_a_rule_fault_sends_loadable_false() {
    let world = affected_world().await;

    let reply = client_get(
        &world.fixture,
        "/export/v2/machines/M30/recipes/R-400/approved/units/U1",
    )
    .await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    // The version stays `Approved`, so the file name has no `RETIRED_` prefix.
    assert_eq!(
        reply.header("content-disposition"),
        "attachment; filename=\"recipe_R-400_v1_U1.json\""
    );
    assert_eq!(field(&reply.body, "loadable"), "false");
    assert_eq!(field(&reply.body, "status"), "\"Approved\"");
    assert_eq!(exported_actors(&world.fixture), vec![Some(client())]);
}

#[tokio::test]
async fn export_unit_with_a_descendant_unit_holds_both_parameters_and_their_types() {
    let (fixture, _data) = seeded_units().await;

    let reply = client_get(&fixture, &unit_route("U1.2")).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(field(&reply.body, "schema_version"), "\"2\"");
    assert!(
        reply
            .body
            .contains("\"scope\": {\n    \"unit_path\": \"U1.2\"\n  }"),
        "{}",
        reply.body
    );
    assert!(has_parameter(&reply.body, "ZONE1"), "{}", reply.body);
    assert!(has_parameter(&reply.body, "ZONE2"), "{}", reply.body);
    assert!(!has_parameter(&reply.body, "SPEED"), "{}", reply.body);
    assert!(!has_parameter(&reply.body, "FEED"), "{}", reply.body);
    assert!(has_type(&reply.body, "ST_Cell"), "{}", reply.body);
    assert!(!has_type(&reply.body, "ST_Zone"), "{}", reply.body);
}

#[tokio::test]
async fn export_unit_path_that_is_a_text_prefix_only_does_not_match_the_other_unit() {
    let (fixture, _data) = seeded_units().await;

    let prefix = client_get(&fixture, &unit_route("U1.2")).await;
    let other = client_get(&fixture, &unit_route("U1.20")).await;

    assert_eq!(prefix.status, StatusCode::OK, "{}", prefix.body);
    assert!(!has_parameter(&prefix.body, "FEED"), "{}", prefix.body);
    assert_eq!(other.status, StatusCode::OK, "{}", other.body);
    assert!(has_parameter(&other.body, "FEED"), "{}", other.body);
    assert!(!has_parameter(&other.body, "ZONE1"), "{}", other.body);
    assert!(!has_parameter(&other.body, "ZONE2"), "{}", other.body);
    assert!(!has_type(&other.body, "ST_Cell"), "{}", other.body);
}

#[tokio::test]
async fn export_unit_of_the_main_unit_holds_every_descendant_parameter() {
    let (fixture, _data) = seeded_units().await;

    let reply = client_get(&fixture, &unit_route("U1")).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    for tag in ["SPEED", "ZONE1", "ZONE2", "FEED"] {
        assert!(has_parameter(&reply.body, tag), "{tag}: {}", reply.body);
    }
}

#[tokio::test]
async fn export_unit_without_a_parameter_gives_empty_scope_and_no_entry() {
    let (fixture, _data) = seeded_units().await;
    let before = exported_actors(&fixture).len();

    let reply = client_get(&fixture, &unit_route("U1.3")).await;

    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert!(reply.body.contains("E-EXP-005"), "{}", reply.body);
    assert!(
        reply.body.contains(
            "Wersja 1 receptury R-800 nie ma parametru w jednostce U1.3 ani poniżej niej."
        ),
        "{}",
        reply.body
    );
    assert_eq!(exported_actors(&fixture).len(), before);
}

#[tokio::test]
async fn export_unit_that_does_not_exist_gives_not_found_and_no_entry() {
    let (fixture, _data) = seeded_units().await;
    let before = exported_actors(&fixture).len();

    let reply = client_get(&fixture, &unit_route("U1.9")).await;

    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-005"), "{}", reply.body);
    assert_eq!(exported_actors(&fixture).len(), before);
}

#[tokio::test]
async fn export_unit_that_is_archived_on_a_live_machine_counts_as_no_unit() {
    let (fixture, data) = seeded_units().await;
    archive_spare_unit(&fixture, &data);
    let before = exported_actors(&fixture).len();

    let reply = client_get(&fixture, &unit_route("U1.21")).await;

    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-005"), "{}", reply.body);
    assert_eq!(exported_actors(&fixture).len(), before);
}

#[tokio::test]
async fn export_unit_with_a_malformed_path_gives_the_unit_path_error_and_no_entry() {
    let (fixture, _data) = seeded_units().await;
    let before = audit_count(&fixture);
    for (unit, status, code) in [
        ("X1", StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-017"),
        ("u1.2", StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-017"),
        ("U1..2", StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-017"),
        ("U1.%FF", StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-017"),
        (
            "U1.2.3.4.5.6",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-002",
        ),
    ] {
        let reply = client_get(&fixture, &unit_route(unit)).await;

        assert_eq!(reply.status, status, "{unit}: {}", reply.body);
        assert!(reply.body.contains(code), "{unit}: {}", reply.body);
    }
    assert_eq!(audit_count(&fixture), before);
}

#[tokio::test]
async fn export_unit_with_a_bad_machine_or_reference_gives_not_found() {
    let (fixture, _data) = seeded_units().await;
    for path in [
        "/export/v2/machines/m7/recipes/R-800/approved/units/U1.2",
        "/export/v2/machines/M99/recipes/R-800/approved/units/U1.2",
        "/export/v2/machines/M20/recipes/R-800/approved/units/U1.2",
        "/export/v2/machines/M7/recipes/R%20800/approved/units/U1.2",
        "/export/v2/machines/M7/recipes/%FF/approved/units/U1.2",
    ] {
        let reply = client_get(&fixture, path).await;

        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-005"), "{path}: {}", reply.body);
    }
}

#[tokio::test]
async fn export_unit_answer_names_the_file_with_the_unit_path_and_is_not_cached() {
    let (fixture, _data) = seeded_units().await;
    let before = exported_actors(&fixture).len();

    let reply = client_get(&fixture, &unit_route("U1.2")).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(
        reply.header("content-disposition"),
        "attachment; filename=\"recipe_R-800_v1_U1.2.json\""
    );
    assert_eq!(reply.header("cache-control"), "no-store");
    assert_eq!(reply.header("content-type"), "application/json");
    let after = exported_actors(&fixture);
    assert_eq!(after.len(), before + 1);
    assert_eq!(after.last(), Some(&Some(client())));
}

#[tokio::test]
async fn export_unit_route_accepts_only_get() {
    let (fixture, _data) = seeded_units().await;
    let header = format!("Basic {}", STANDARD.encode("user1:tr4nsfer-s3cret"));
    let builder = request(Method::HEAD, &unit_route("U1.2")).header("authorization", &header);

    let head = call(&fixture.router(), builder.body(Body::empty()).unwrap()).await;

    assert_eq!(head.status, StatusCode::NOT_FOUND, "{}", head.body);
    assert!(exported_actors(&fixture).is_empty());
}
