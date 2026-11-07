//! Tests of the export routes for an approved version that breaks a platform range rule
//! (ADR-0139 (f), ADR-0140 (e)).
//!
//! The stored value or address passes the earlier rule set and breaks the current one. Each export
//! answers with the file, `loadable` `false`, and the stored hash.

use recipe_app::ports::{Isolation, UnitOfWork};
use recipe_domain::recipe::RecipeVersionId;
use uuid::Uuid;

use super::tests::{assert_file, export, exports, field};
use crate::test_support::Fixture;
use crate::test_support::plant::World;
use crate::test_support::recipes::platform_range_world::{
    DATA_BLOCK_REFERENCE, LDT_REFERENCE, LDT_VERSION_ID, data_block_above_59999_world,
    ldt_after_current_range_world,
};

/// Return the two session routes of the approved version 1 of the recipe `reference`.
fn session_routes(reference: &str) -> [String; 2] {
    [
        format!("/export/v1/recipes/{reference}/approved"),
        format!("/export/v1/recipes/{reference}/versions/1"),
    ]
}

/// Return the content hash that the store holds for the approved version `version`.
async fn stored_hash(fixture: &Fixture, version: Uuid) -> String {
    let mut tx = fixture.store.begin(Isolation::ReadOnly).await.unwrap();
    let detail = tx
        .recipe_reads()
        .version_detail(RecipeVersionId::from(version))
        .await
        .unwrap()
        .unwrap();
    detail.content_sha256.unwrap().to_string()
}

/// Send both session routes and assert the answer of a version that breaks a range rule.
async fn assert_loadable_false_on_both_routes(world: &World, reference: &str) {
    let file_name = format!("recipe_{reference}_v1.json");
    for path in session_routes(reference) {
        let reply = export(&world.fixture, &world.admin, &path).await;
        assert_file(&reply, &file_name);
        assert_eq!(field(&reply.body, "loadable"), "false", "{path}");
        assert_eq!(field(&reply.body, "status"), "\"Approved\"", "{path}");
    }
    assert_eq!(exports(&world.fixture).len(), 2);
}

/// Scenario "Approved version with an LDT after the current range on TwinCat3" (ADR-0139 (f)).
///
/// The stored `LDT` count 10413792000000000000 is above 2^63-1. Both routes send the file with
/// `loadable` `false` and the stored hash.
#[tokio::test]
async fn export_routes_of_an_ldt_after_the_current_range_on_twincat3_send_loadable_false() {
    let world = ldt_after_current_range_world().await;
    assert_loadable_false_on_both_routes(&world, LDT_REFERENCE).await;
    let stored = stored_hash(&world.fixture, LDT_VERSION_ID).await;
    for path in session_routes(LDT_REFERENCE) {
        let reply = export(&world.fixture, &world.admin, &path).await;
        let sent = field(&reply.body, "content_sha256").trim_matches('"');
        assert_eq!(sent, stored, "{path}");
    }
}

/// Scenario "Data block above 59999 on S7_1500" (ADR-0140 (e)).
///
/// The stored `INT` parameter `SPEED` sits at `DB60000.DBW0`. Both routes send the file with
/// `loadable` `false`.
#[tokio::test]
async fn export_routes_of_a_data_block_above_59999_on_s7_1500_send_loadable_false() {
    let world = data_block_above_59999_world().await;
    assert_loadable_false_on_both_routes(&world, DATA_BLOCK_REFERENCE).await;
}
