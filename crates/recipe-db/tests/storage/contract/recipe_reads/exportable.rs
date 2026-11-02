//! Contract cases of `RecipeReads::exportable_recipes` (design D4, ADR-0109 (j)).
//!
//! The memory adapter and the statement give the same list. The case that needs two different
//! archive instants carries the prefix `postgres_only`, because the memory clock stands still.

use recipe_app::ports::ExportableRecipes;
use recipe_domain::plant::{MachineId, PlcPlatform};
use recipe_domain::recipe::Recipe;

use super::fixtures::{begin_read, create_named, delete_recipe, discard};
use crate::contract::harness::{Harness, begin, commit, fixture_hash};
use crate::contract::recipe_seed::{Plant, approve, create_version, plant, retire, version_id};
use crate::contract::seed::{archive, code};

/// Read the list of exportable recipes of `wanted` in a read transaction.
async fn exportable(h: &dyn Harness, wanted: &str) -> Option<ExportableRecipes> {
    let mut tx = begin_read(h).await;
    tx.recipe_reads()
        .exportable_recipes(code(wanted))
        .await
        .unwrap()
}

/// Return the reference numbers of the list as text, in the order of the list.
fn references(found: &ExportableRecipes) -> Vec<String> {
    found
        .recipes
        .iter()
        .map(|row| row.reference.to_string())
        .collect()
}

/// Seed a recipe whose first version is approved.
async fn approved_recipe(h: &dyn Harness, plant: &Plant, n: u128, reference: &str) -> Recipe {
    let recipe = create_named(h, plant, n, reference, "Recipe").await;
    approve(h, recipe.versions()[0].id()).await;
    recipe
}

async fn exportable_recipes_gives_none_for_a_code_without_a_machine(h: &dyn Harness) {
    plant(h, 1, "M1").await;

    assert_eq!(exportable(h, "M2").await, None);
}

async fn exportable_recipes_gives_an_empty_list_for_a_machine_without_approved_versions(
    h: &dyn Harness,
) {
    let plant = plant(h, 2, "M2").await;
    create_named(h, &plant, 2, "R-DRAFT", "Recipe").await;

    let found = exportable(h, "M2").await.unwrap();

    assert_eq!(found.code, code("M2"));
    assert_eq!(found.platform, PlcPlatform::TwinCat3);
    assert!(!found.archived);
    assert!(found.recipes.is_empty(), "{found:?}");
}

async fn exportable_recipes_keeps_only_undeleted_recipes_with_an_approved_version(h: &dyn Harness) {
    let plant = plant(h, 3, "M3").await;
    create_named(h, &plant, 3, "R-DRAFT", "Recipe").await;
    let retired = approved_recipe(h, &plant, 4, "R-RETIRED").await;
    retire(h, retired.versions()[0].id()).await;
    let discarded = create_named(h, &plant, 5, "R-DISCARDED", "Recipe").await;
    discard(h, discarded.versions()[0].id()).await;
    let deleted = approved_recipe(h, &plant, 6, "R-DELETED").await;
    delete_recipe(h, &deleted).await;
    approved_recipe(h, &plant, 7, "R-OK").await;

    let found = exportable(h, "M3").await.unwrap();

    assert_eq!(references(&found), ["R-OK"]);
}

async fn exportable_recipes_returns_the_approved_number_and_the_stored_hash_in_byte_order(
    h: &dyn Harness,
) {
    let plant = plant(h, 8, "M8").await;
    for (n, reference) in [(8, "R2"), (9, "R10"), (11, "R-2"), (12, "A1")] {
        approved_recipe(h, &plant, n, reference).await;
    }
    // The recipe R-TEN retires its first version and approves its second version.
    let ten = approved_recipe(h, &plant, 10, "R-TEN").await;
    retire(h, ten.versions()[0].id()).await;
    create_version(h, (ten.id(), version_id(110)), 2, None).await;
    approve(h, version_id(110)).await;

    let found = exportable(h, "M8").await.unwrap();

    // Byte order: '-' (0x2D) < '1' (0x31) < '2' (0x32).
    assert_eq!(
        references(&found),
        ["A1", "R-2", "R-TEN", "R10", "R2"],
        "{found:?}"
    );
    let stored = begin_read(h)
        .await
        .recipe_reads()
        .version_detail(version_id(110))
        .await
        .unwrap()
        .unwrap();
    let ten = &found.recipes[2];
    assert_eq!(ten.name.as_str(), "Recipe");
    assert_eq!(ten.version_number, 2);
    assert_eq!(Some(ten.approved_at), stored.approved_at);
    assert!(
        found
            .recipes
            .iter()
            .all(|row| row.content_sha256 == fixture_hash())
    );
}

async fn exportable_recipes_prefers_the_live_machine_over_an_archived_machine_with_its_code(
    h: &dyn Harness,
) {
    let old = plant(h, 20, "M20").await;
    approved_recipe(h, &old, 20, "R-OLD").await;
    archive(h, old.machine).await;
    let live = plant(h, 21, "M20").await;
    approved_recipe(h, &live, 21, "R-LIVE").await;

    let found = exportable(h, "M20").await.unwrap();

    assert!(!found.archived);
    assert_eq!(references(&found), ["R-LIVE"]);
}

async fn exportable_recipes_gives_the_recipes_of_an_archived_machine_without_a_live_machine(
    h: &dyn Harness,
) {
    let old = plant(h, 30, "M30").await;
    approved_recipe(h, &old, 30, "R-OLD").await;
    archive(h, old.machine).await;

    let found = exportable(h, "M30").await.unwrap();

    assert!(found.archived);
    assert_eq!(found.code, code("M30"));
    assert_eq!(found.platform, PlcPlatform::TwinCat3);
    assert_eq!(references(&found), ["R-OLD"]);
}

/// Archive the machines `ids` in one transaction. The transaction gives them one `archived_at`.
async fn archive_together(h: &dyn Harness, ids: [MachineId; 2]) {
    let mut tx = begin(h).await;
    for id in ids {
        tx.plant().archive_machine(id).await.unwrap();
    }
    commit(h, tx, ids[0]).await;
}

async fn exportable_recipes_breaks_an_archive_tie_with_the_higher_machine_id(h: &dyn Harness) {
    let high = plant(h, 42, "M40").await;
    approved_recipe(h, &high, 42, "R-HIGH").await;
    archive(h, high.machine).await;
    let low = plant(h, 41, "M40").await;
    approved_recipe(h, &low, 41, "R-LOW").await;
    archive_together(h, [low.machine, high.machine]).await;

    let found = exportable(h, "M40").await.unwrap();

    assert!(found.archived);
    assert_eq!(references(&found), ["R-HIGH"]);
}

contract_case!(
    exportable_recipes_gives_none_for_a_code_without_a_machine,
    exportable_recipes_gives_none_for_a_code_without_a_machine_memory,
    exportable_recipes_gives_none_for_a_code_without_a_machine_postgres
);
contract_case!(
    exportable_recipes_gives_an_empty_list_for_a_machine_without_approved_versions,
    exportable_recipes_gives_an_empty_list_for_a_machine_without_approved_versions_memory,
    exportable_recipes_gives_an_empty_list_for_a_machine_without_approved_versions_postgres
);
contract_case!(
    exportable_recipes_keeps_only_undeleted_recipes_with_an_approved_version,
    exportable_recipes_keeps_only_undeleted_recipes_with_an_approved_version_memory,
    exportable_recipes_keeps_only_undeleted_recipes_with_an_approved_version_postgres
);
contract_case!(
    exportable_recipes_returns_the_approved_number_and_the_stored_hash_in_byte_order,
    exportable_recipes_returns_the_approved_number_and_the_stored_hash_in_byte_order_memory,
    exportable_recipes_returns_the_approved_number_and_the_stored_hash_in_byte_order_postgres
);
contract_case!(
    exportable_recipes_prefers_the_live_machine_over_an_archived_machine_with_its_code,
    exportable_recipes_prefers_the_live_machine_over_an_archived_machine_with_its_code_memory,
    exportable_recipes_prefers_the_live_machine_over_an_archived_machine_with_its_code_postgres
);
contract_case!(
    exportable_recipes_gives_the_recipes_of_an_archived_machine_without_a_live_machine,
    exportable_recipes_gives_the_recipes_of_an_archived_machine_without_a_live_machine_memory,
    exportable_recipes_gives_the_recipes_of_an_archived_machine_without_a_live_machine_postgres
);
contract_case!(
    exportable_recipes_breaks_an_archive_tie_with_the_higher_machine_id,
    exportable_recipes_breaks_an_archive_tie_with_the_higher_machine_id_memory,
    exportable_recipes_breaks_an_archive_tie_with_the_higher_machine_id_postgres
);

/// Postgres only: the memory clock stands still, so two archives never differ there.
///
/// The machine that the system archived last wins, even when its id is the lower one.
#[tokio::test]
async fn postgres_only_exportable_recipes_takes_the_machine_archived_last_when_none_is_live() {
    let h = crate::contract::PgHarness::create().await;
    let earlier = plant(&h, 52, "M50").await;
    approved_recipe(&h, &earlier, 52, "R-EARLIER").await;
    archive(&h, earlier.machine).await;
    let later = plant(&h, 51, "M50").await;
    approved_recipe(&h, &later, 51, "R-LATER").await;
    archive(&h, later.machine).await;

    let found = exportable(&h, "M50").await.unwrap();

    assert!(found.archived);
    assert_eq!(references(&found), ["R-LATER"]);
}
