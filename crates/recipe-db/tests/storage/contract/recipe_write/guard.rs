//! The guard of a version status write (ADR-0030, delta requirement "Guard of a version status
//! write").
//!
//! `MemoryStore` and `Database` both record the `row_version` of each locked version. A status
//! write without that record gives `E-DB-008` and writes nothing. These cases run on both
//! adapters with the same assertions (design D6).

use recipe_app::error::StorageError;
use recipe_app::ports::Transaction;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::{RecipeVersionId, VersionState};

use crate::contract::harness::{Harness, begin, commit, fixture_hash};
use crate::contract::images::{field, text};
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

/// Assert, inside the open transaction, that a refused write did not change the version row.
///
/// The read takes no lock, so it does not open the guard of the next write. A drop of the
/// transaction would roll a change back and hide it.
async fn assert_unchanged_in_transaction(tx: &mut dyn Transaction, version: RecipeVersionId) {
    let detail = tx
        .recipe_reads()
        .version_detail(version)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(detail.state, VersionState::LiveDraft);
    assert!(detail.approved_by.is_none());
    assert!(detail.approved_at.is_none());
    assert!(detail.content_sha256.is_none());
    assert_eq!(detail.row_version, 1);
}

async fn a_status_write_without_its_lock_gives_e_db_008_and_writes_nothing(h: &dyn Harness) {
    let plant = plant(h, 1, "M1").await;
    let created = create_recipe(h, &plant, 1, "REF-1").await;
    let draft = created.versions()[0].id();
    let mut tx = begin(h).await;

    let discard = tx.recipes().discard_version(draft).await.unwrap_err();
    assert_unchanged_in_transaction(&mut *tx, draft).await;
    let retire = tx.recipes().retire_version(draft).await.unwrap_err();
    assert_unchanged_in_transaction(&mut *tx, draft).await;
    let approve = tx
        .recipes()
        .approve_version(draft, h.user(), fixture_hash())
        .await
        .unwrap_err();
    assert_unchanged_in_transaction(&mut *tx, draft).await;
    drop(tx);

    for error in [&discard, &retire, &approve] {
        assert_unguarded(error);
    }
    assert_untouched_draft(h, draft).await;
}

async fn the_lock_of_another_version_does_not_open_the_guard(h: &dyn Harness) {
    let plant = plant(h, 2, "M2").await;
    let created = create_recipe(h, &plant, 2, "REF-2").await;
    let first = created.versions()[0].id();
    approve(h, first).await;
    create_version(h, (created.id(), version_id(1)), 2, Some(first)).await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(first).await.unwrap().unwrap();

    let error = tx
        .recipes()
        .discard_version(version_id(1))
        .await
        .unwrap_err();
    assert_unchanged_in_transaction(&mut *tx, version_id(1)).await;
    drop(tx);

    assert_unguarded(&error);
    assert_untouched_draft(h, version_id(1)).await;
}

async fn a_lock_of_all_versions_opens_the_guard_of_each_version(h: &dyn Harness) {
    let plant = plant(h, 3, "M3").await;
    let created = create_recipe(h, &plant, 3, "REF-3").await;
    let first = created.versions()[0].id();
    approve(h, first).await;
    create_version(h, (created.id(), version_id(1)), 2, Some(first)).await;
    let draft = version_id(1);
    let mut tx = begin(h).await;
    tx.recipes().lock_versions(created.id()).await.unwrap();

    let retired = tx.recipes().retire_version(first).await.unwrap();
    let discarded = tx.recipes().discard_version(draft).await.unwrap();
    commit(h, tx, plant.machine).await;

    assert_eq!(text(&retired, "status"), "retired");
    assert!(!field(&discarded, "deleted_at").is_null());
    assert_eq!(field(&discarded, "row_version").as_i64(), Some(2));
}

async fn a_lock_does_not_outlive_its_transaction(h: &dyn Harness) {
    let plant = plant(h, 4, "M4").await;
    let created = create_recipe(h, &plant, 4, "REF-4").await;
    let draft = created.versions()[0].id();
    let mut first = begin(h).await;
    first.recipes().lock_version(draft).await.unwrap().unwrap();
    drop(first);
    let mut second = begin(h).await;

    let error = second.recipes().discard_version(draft).await.unwrap_err();
    assert_unchanged_in_transaction(&mut *second, draft).await;
    drop(second);

    assert_unguarded(&error);
    assert_untouched_draft(h, draft).await;
}

async fn a_status_write_after_lock_draft_passes_the_guard(h: &dyn Harness) {
    let plant = plant(h, 5, "M5").await;
    let created = create_recipe(h, &plant, 5, "REF-5").await;
    let draft = created.versions()[0].id();
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(draft).await.unwrap().unwrap();

    let image = tx.recipes().discard_version(draft).await.unwrap();
    commit(h, tx, plant.machine).await;

    assert!(!field(&image, "deleted_at").is_null());
    assert_eq!(field(&image, "row_version").as_i64(), Some(2));
}

contract_case!(
    a_status_write_without_its_lock_gives_e_db_008_and_writes_nothing,
    a_status_write_without_its_lock_gives_e_db_008_and_writes_nothing_memory,
    a_status_write_without_its_lock_gives_e_db_008_and_writes_nothing_postgres
);
contract_case!(
    the_lock_of_another_version_does_not_open_the_guard,
    the_lock_of_another_version_does_not_open_the_guard_memory,
    the_lock_of_another_version_does_not_open_the_guard_postgres
);
contract_case!(
    a_lock_of_all_versions_opens_the_guard_of_each_version,
    a_lock_of_all_versions_opens_the_guard_of_each_version_memory,
    a_lock_of_all_versions_opens_the_guard_of_each_version_postgres
);
contract_case!(
    a_lock_does_not_outlive_its_transaction,
    a_lock_does_not_outlive_its_transaction_memory,
    a_lock_does_not_outlive_its_transaction_postgres
);
contract_case!(
    a_status_write_after_lock_draft_passes_the_guard,
    a_status_write_after_lock_draft_passes_the_guard_memory,
    a_status_write_after_lock_draft_passes_the_guard_postgres
);
