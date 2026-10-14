//! Postgres only: the row locks of the lock methods of `RecipeRepository`.
//!
//! A probe tries a lock with `NOWAIT` from a second session. `55P03` shows a conflicting lock.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::contract::harness::begin;
use crate::contract::locks::{LOCK_NOT_AVAILABLE, try_row_lock, wait_until_released};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::recipe_seed::{approve, create_recipe, create_version, plant, version_id};
use crate::support::Role;

async fn probe(
    session: &mut PgConnection,
    table: &'static str,
    mode: &str,
    id: Uuid,
) -> Option<String> {
    try_row_lock(session, table, mode, id).await
}

async fn assert_free(session: &mut PgConnection, table: &'static str, id: Uuid) {
    assert_eq!(
        probe(session, table, "UPDATE", id).await,
        None,
        "{table} {id}"
    );
}

async fn assert_held_for_update(session: &mut PgConnection, table: &'static str, id: Uuid) {
    for mode in ["UPDATE", "NO KEY UPDATE", "SHARE", "KEY SHARE"] {
        assert_eq!(
            probe(session, table, mode, id).await.as_deref(),
            Some(LOCK_NOT_AVAILABLE),
            "{table} {id} {mode}"
        );
    }
}

/// `lock_recipe` holds `FOR UPDATE` on the recipe row and no lock on the version row.
#[tokio::test]
async fn postgres_only_lock_recipe_holds_for_update_on_the_recipe() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 20, "M20").await;
    let created = create_recipe(&h, &plant, 20, "REF-20").await;
    let mut session = h.database.connect(Role::Migrator).await;
    let id = *created.id().as_uuid();
    let version = *created.versions()[0].id().as_uuid();
    let mut tx = begin(&h).await;

    tx.recipes()
        .lock_recipe(created.id())
        .await
        .unwrap()
        .unwrap();

    assert_held_for_update(&mut session, "recipe", id).await;
    assert_free(&mut session, "recipe_version", version).await;
    drop(tx);
    wait_until_released(&mut session, "recipe", id).await;
}

/// `lock_recipes_of_machine` locks every recipe of the machine and no recipe of another machine.
#[tokio::test]
async fn postgres_only_lock_recipes_of_machine_locks_exactly_the_recipes_of_the_machine() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 21, "M21").await;
    let other = crate::contract::recipe_seed::plant(&h, 22, "M22").await;
    let first = create_recipe(&h, &plant, 40, "REF-40").await;
    let second = create_recipe(&h, &plant, 41, "REF-41").await;
    let foreign = create_recipe(&h, &other, 42, "REF-42").await;
    let mut session = h.database.connect(Role::Migrator).await;
    let mut tx = begin(&h).await;

    tx.recipes()
        .lock_recipes_of_machine(plant.machine)
        .await
        .unwrap();

    assert_held_for_update(&mut session, "recipe", *first.id().as_uuid()).await;
    assert_held_for_update(&mut session, "recipe", *second.id().as_uuid()).await;
    assert_free(&mut session, "recipe", *foreign.id().as_uuid()).await;
    drop(tx);
    wait_until_released(&mut session, "recipe", *first.id().as_uuid()).await;
    wait_until_released(&mut session, "recipe", *second.id().as_uuid()).await;
}

/// `lock_version` holds `FOR UPDATE` on the version row and no lock on the recipe row.
#[tokio::test]
async fn postgres_only_lock_version_holds_for_update_on_the_version() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 23, "M23").await;
    let created = create_recipe(&h, &plant, 23, "REF-23").await;
    let mut session = h.database.connect(Role::Migrator).await;
    let version = created.versions()[0].id();
    let mut tx = begin(&h).await;

    tx.recipes().lock_version(version).await.unwrap().unwrap();

    assert_held_for_update(&mut session, "recipe_version", *version.as_uuid()).await;
    assert_free(&mut session, "recipe", *created.id().as_uuid()).await;
    drop(tx);
    wait_until_released(&mut session, "recipe_version", *version.as_uuid()).await;
}

/// `lock_versions` holds `FOR UPDATE` on every version of the recipe and on no other version.
#[tokio::test]
async fn postgres_only_lock_versions_locks_every_version_of_the_recipe() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 24, "M24").await;
    let created = create_recipe(&h, &plant, 24, "REF-24").await;
    let first = created.versions()[0].id();
    approve(&h, first).await;
    create_version(&h, (created.id(), version_id(1)), 2, Some(first)).await;
    let foreign = create_recipe(&h, &plant, 25, "REF-25").await;
    let mut session = h.database.connect(Role::Migrator).await;
    let mut tx = begin(&h).await;

    tx.recipes().lock_versions(created.id()).await.unwrap();

    assert_held_for_update(&mut session, "recipe_version", *first.as_uuid()).await;
    assert_held_for_update(&mut session, "recipe_version", *version_id(1).as_uuid()).await;
    assert_free(
        &mut session,
        "recipe_version",
        *foreign.versions()[0].id().as_uuid(),
    )
    .await;
    drop(tx);
    wait_until_released(&mut session, "recipe_version", *first.as_uuid()).await;
    wait_until_released(&mut session, "recipe_version", *version_id(1).as_uuid()).await;
}

/// `lock_version_shared` holds `FOR SHARE`. Another `FOR SHARE` and `FOR KEY SHARE` pass.
#[tokio::test]
async fn postgres_only_lock_version_shared_holds_for_share_on_the_version() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 26, "M26").await;
    let created = create_recipe(&h, &plant, 26, "REF-26").await;
    let mut session = h.database.connect(Role::Migrator).await;
    let version = created.versions()[0].id();
    let id = *version.as_uuid();
    let mut tx = begin(&h).await;

    tx.recipes()
        .lock_version_shared(version)
        .await
        .unwrap()
        .unwrap();

    for mode in ["UPDATE", "NO KEY UPDATE"] {
        assert_eq!(
            probe(&mut session, "recipe_version", mode, id)
                .await
                .as_deref(),
            Some(LOCK_NOT_AVAILABLE),
            "{mode}"
        );
    }
    assert_eq!(
        probe(&mut session, "recipe_version", "SHARE", id).await,
        None
    );
    assert_eq!(
        probe(&mut session, "recipe_version", "KEY SHARE", id).await,
        None
    );
    drop(tx);
    wait_until_released(&mut session, "recipe_version", id).await;
}

/// A lock of a missing row takes no lock and does not fail.
#[tokio::test]
async fn postgres_only_a_missing_recipe_and_a_missing_version_take_no_lock() {
    let h = PgHarness::create().await;
    let mut tx = begin(&h).await;

    let recipe = tx
        .recipes()
        .lock_recipe(crate::contract::recipe_seed::recipe_id(98))
        .await
        .unwrap();
    let version = tx.recipes().lock_version(version_id(98)).await.unwrap();
    let shared = tx
        .recipes()
        .lock_version_shared(version_id(98))
        .await
        .unwrap();

    assert!(recipe.is_none());
    assert!(version.is_none());
    assert!(shared.is_none());
}
