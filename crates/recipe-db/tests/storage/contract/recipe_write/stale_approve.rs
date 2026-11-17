//! A stale record wins over the uniqueness check of an approval (ADR-0030, design D6).
//!
//! PostgreSQL holds `FOR UPDATE` on a locked version, so a second transaction cannot change the
//! row before the first one ends. The recorded `row_version` cannot differ from the row there.
//! The memory adapter does not block, so a commit of another transaction can make the record
//! stale after a `lock_version_shared` refresh. The case runs on `MemoryStore` only. It pins the order of the memory checks: a record
//! that differs from the row gives `E-DB-008`, also when an approved sibling exists. The
//! guarded `UPDATE ... AND row_version = $locked` of PostgreSQL matches zero rows and gives the
//! same code.

use recipe_domain::error_codes::ErrorCode;

use crate::contract::MemoryHarness;
use crate::contract::harness::{Harness, begin, commit, fixture_hash};
use crate::contract::images::{field, text};
use crate::contract::recipe_seed::{approve, create_recipe, create_version, plant, version_id};

async fn a_stale_record_with_an_approved_sibling_gives_e_db_008_and_not_a_duplicate(
    h: &dyn Harness,
) {
    let plant = plant(h, 7, "M7").await;
    let created = create_recipe(h, &plant, 7, "REF-7").await;
    let first = created.versions()[0].id();
    approve(h, first).await;
    let draft = version_id(1);
    create_version(h, (created.id(), draft), 7, Some(first)).await;
    let mut tx = begin(h).await;
    let before = tx.recipes().lock_version(draft).await.unwrap().unwrap();
    assert_eq!(field(&before, "row_version").as_i64(), Some(1));
    let mut other = begin(h).await;
    other.drafts().touch_draft(draft, h.user()).await.unwrap();
    commit(h, other, plant.machine).await;
    tx.recipes()
        .lock_version_shared(draft)
        .await
        .unwrap()
        .unwrap();

    let error = tx
        .recipes()
        .approve_version(draft, h.user(), fixture_hash())
        .await
        .unwrap_err();
    drop(tx);

    assert_eq!(error.code(), ErrorCode::DB_008, "{error:?}");
    let mut check = begin(h).await;
    let image = check.recipes().lock_version(draft).await.unwrap().unwrap();
    assert_eq!(text(&image, "status"), "draft");
}

#[tokio::test]
async fn a_stale_record_with_an_approved_sibling_gives_e_db_008_and_not_a_duplicate_memory() {
    let harness = MemoryHarness::create().await;
    a_stale_record_with_an_approved_sibling_gives_e_db_008_and_not_a_duplicate(&harness).await;
}
