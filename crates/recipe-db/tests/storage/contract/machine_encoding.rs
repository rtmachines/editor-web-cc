//! Contract cases of `machine_has_version` and `set_machine_string_encoding` (ADR-0202 (d), task 11.12).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions.

use recipe_domain::plant::{MachineId, StringEncoding};
use recipe_domain::recipe::{Recipe, RecipeVersionId};

use super::harness::{Harness, begin, commit};
use super::images::{assert_shape, machine_shape, text};
use super::recipe_seed::{Plant, create_recipe, plant};
use super::seed::{insert, machine, read};

/// Run `machine_has_version` in a transaction that nothing commits.
async fn has_version(h: &dyn Harness, id: MachineId) -> bool {
    let mut tx = begin(h).await;
    tx.plant().machine_has_version(id).await.unwrap()
}

/// Mark a recipe deleted with the lock of `DeleteRecipe`.
async fn delete_recipe(h: &dyn Harness, recipe: &Recipe) {
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_recipe(recipe.id())
        .await
        .unwrap()
        .expect("the recipe exists");
    tx.recipes().mark_recipe_deleted(recipe.id()).await.unwrap();
    commit(h, tx, recipe.machine()).await;
}

/// Discard a live draft with the port sequence of `DiscardDraft`.
async fn discard(h: &dyn Harness, version: RecipeVersionId) {
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_version(version)
        .await
        .unwrap()
        .expect("the version exists");
    tx.recipes().discard_version(version).await.unwrap();
    commit(h, tx, MachineId::from(uuid::Uuid::nil())).await;
}

async fn the_machine_without_a_recipe_has_no_version(h: &dyn Harness) {
    let plant = plant(h, 1, "M1").await;

    assert!(!has_version(h, plant.machine).await);
}

async fn a_live_draft_counts_as_a_version(h: &dyn Harness) {
    let plant = plant(h, 2, "M2").await;
    create_recipe(h, &plant, 2, "REF-2").await;

    assert!(has_version(h, plant.machine).await);
}

async fn a_discarded_draft_counts_as_a_version(h: &dyn Harness) {
    let plant = plant(h, 3, "M3").await;
    let recipe = create_recipe(h, &plant, 3, "REF-3").await;
    discard(h, recipe.versions()[0].id()).await;

    assert!(has_version(h, plant.machine).await);
}

async fn a_version_of_a_deleted_recipe_counts_as_a_version(h: &dyn Harness) {
    let plant = plant(h, 4, "M4").await;
    let recipe = create_recipe(h, &plant, 4, "REF-4").await;
    delete_recipe(h, &recipe).await;

    assert!(has_version(h, plant.machine).await);
}

async fn a_version_of_another_machine_does_not_count(h: &dyn Harness) {
    let other: Plant = plant(h, 5, "M5").await;
    create_recipe(h, &other, 5, "REF-5").await;
    let own = plant(h, 6, "M6").await;

    assert!(!has_version(h, own.machine).await);
    assert!(has_version(h, other.machine).await);
}

async fn set_machine_string_encoding_gives_the_row_image_after_the_change(h: &dyn Harness) {
    let built = machine(7, "M7");
    insert(h, &built).await;
    let mut tx = begin(h).await;
    tx.plant()
        .lock_machine_for_no_key_update(built.id())
        .await
        .unwrap()
        .expect("the machine exists");

    let image = tx
        .plant()
        .set_machine_string_encoding(built.id(), StringEncoding::Utf8)
        .await
        .unwrap();
    commit(h, tx, built.id()).await;

    assert_shape(&image, &machine_shape(false));
    assert_eq!(text(&image, "string_encoding"), "Utf8");
    assert_eq!(text(&image, "code"), "M7");
    let stored = read(h, built.id()).await.expect("the machine exists");
    assert_eq!(stored.string_encoding(), StringEncoding::Utf8);
}

contract_case!(
    the_machine_without_a_recipe_has_no_version,
    adr_0202_machine_has_version_is_false_without_a_recipe_memory,
    adr_0202_machine_has_version_is_false_without_a_recipe_postgres
);
contract_case!(
    a_live_draft_counts_as_a_version,
    adr_0202_machine_has_version_is_true_for_a_live_draft_memory,
    adr_0202_machine_has_version_is_true_for_a_live_draft_postgres
);
contract_case!(
    a_discarded_draft_counts_as_a_version,
    adr_0202_machine_has_version_is_true_for_a_discarded_draft_memory,
    adr_0202_machine_has_version_is_true_for_a_discarded_draft_postgres
);
contract_case!(
    a_version_of_a_deleted_recipe_counts_as_a_version,
    adr_0202_machine_has_version_is_true_for_a_deleted_recipe_memory,
    adr_0202_machine_has_version_is_true_for_a_deleted_recipe_postgres
);
contract_case!(
    a_version_of_another_machine_does_not_count,
    adr_0202_machine_has_version_is_false_for_another_machine_memory,
    adr_0202_machine_has_version_is_false_for_another_machine_postgres
);
contract_case!(
    set_machine_string_encoding_gives_the_row_image_after_the_change,
    adr_0202_set_machine_string_encoding_gives_the_image_after_the_change_memory,
    adr_0202_set_machine_string_encoding_gives_the_image_after_the_change_postgres
);
