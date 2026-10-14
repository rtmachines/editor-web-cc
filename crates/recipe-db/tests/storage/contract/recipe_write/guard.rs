//! Postgres only: the guard of a version status write (ADR-0030, delta requirement "Guard of a
//! version status write").
//!
//! `MemoryStore` keeps no record of version locks, so it accepts a status write without a lock.
//! These cases therefore run on `Database` only.

use recipe_app::error::StorageError;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::RecipeVersionId;

use crate::contract::harness::{Harness, begin, commit, fixture_hash};
use crate::contract::images::{field, text};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::recipe_seed::{approve, create_recipe, create_version, plant, version_id};

/// Assert the error of a status write without its lock: `E-DB-008` from `StorageError::Other`.
fn assert_unguarded(error: &StorageError) {
    assert!(
        matches!(error, StorageError::Other { sqlstate: None, .. }),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_008);
}

/// Assert that the version is still a live draft with the row version 1.
async fn assert_untouched_draft(h: &dyn Harness, version: RecipeVersionId) {
    let mut tx = begin(h).await;
    let image = tx.recipes().lock_version(version).await.unwrap().unwrap();
    assert_eq!(text(&image, "status"), "draft");
    assert!(field(&image, "deleted_at").is_null());
    assert!(field(&image, "approved_by").is_null());
    assert_eq!(field(&image, "row_version").as_i64(), Some(1));
}

#[tokio::test]
async fn postgres_only_a_status_write_without_its_lock_gives_e_db_008_and_writes_nothing() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 1, "M1").await;
    let created = create_recipe(&h, &plant, 1, "REF-1").await;
    let draft = created.versions()[0].id();
    let mut tx = begin(&h).await;

    let discard = tx.recipes().discard_version(draft).await.unwrap_err();
    let retire = tx.recipes().retire_version(draft).await.unwrap_err();
    let approve = tx
        .recipes()
        .approve_version(draft, h.user(), fixture_hash())
        .await
        .unwrap_err();
    drop(tx);

    for error in [&discard, &retire, &approve] {
        assert_unguarded(error);
    }
    assert_untouched_draft(&h, draft).await;
}

#[tokio::test]
async fn postgres_only_the_lock_of_another_version_does_not_open_the_guard() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 2, "M2").await;
    let created = create_recipe(&h, &plant, 2, "REF-2").await;
    let first = created.versions()[0].id();
    approve(&h, first).await;
    create_version(&h, (created.id(), version_id(1)), 2, Some(first)).await;
    let mut tx = begin(&h).await;
    tx.recipes().lock_version(first).await.unwrap().unwrap();

    let error = tx
        .recipes()
        .discard_version(version_id(1))
        .await
        .unwrap_err();
    drop(tx);

    assert_unguarded(&error);
    assert_untouched_draft(&h, version_id(1)).await;
}

#[tokio::test]
async fn postgres_only_a_lock_of_all_versions_opens_the_guard_of_each_version() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 3, "M3").await;
    let created = create_recipe(&h, &plant, 3, "REF-3").await;
    let draft = created.versions()[0].id();
    let mut tx = begin(&h).await;
    tx.recipes().lock_versions(created.id()).await.unwrap();

    let image = tx.recipes().discard_version(draft).await.unwrap();
    commit(&h, tx, plant.machine).await;

    assert!(!field(&image, "deleted_at").is_null());
    assert_eq!(field(&image, "row_version").as_i64(), Some(2));
}

#[tokio::test]
async fn postgres_only_a_lock_does_not_outlive_its_transaction() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 4, "M4").await;
    let created = create_recipe(&h, &plant, 4, "REF-4").await;
    let draft = created.versions()[0].id();
    let mut first = begin(&h).await;
    first.recipes().lock_version(draft).await.unwrap().unwrap();
    drop(first);
    let mut second = begin(&h).await;

    let error = second.recipes().discard_version(draft).await.unwrap_err();
    drop(second);

    assert_unguarded(&error);
    assert_untouched_draft(&h, draft).await;
}
