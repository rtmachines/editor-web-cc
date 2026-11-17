//! The guard compares the recorded `row_version` with the row (ADR-0030, design D6).
//!
//! PostgreSQL holds `FOR UPDATE` on a locked version, so a second transaction cannot change the
//! row before the first one ends. The recorded value cannot differ from the row there, and the
//! statement `UPDATE ... AND row_version = $locked` is only a second line of defense. The memory
//! adapter does not block. A refresh by `lock_version_shared` shows a row that another
//! transaction changed after the lock, so the case runs on `MemoryStore` only.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::VersionState;

use crate::contract::MemoryHarness;
use crate::contract::harness::{Harness, begin};
use crate::contract::images::{field, text};
use crate::contract::recipe_seed::{approve, create_recipe, plant, read_recipe};

async fn a_status_write_after_a_change_by_another_transaction_gives_e_db_008(h: &dyn Harness) {
    let plant = plant(h, 6, "M6").await;
    let created = create_recipe(h, &plant, 6, "REF-6").await;
    let draft = created.versions()[0].id();
    let mut first = begin(h).await;
    let before = first.recipes().lock_version(draft).await.unwrap().unwrap();
    assert_eq!(field(&before, "row_version").as_i64(), Some(1));
    approve(h, draft).await;
    first
        .recipes()
        .lock_version_shared(draft)
        .await
        .unwrap()
        .unwrap();

    let error = first.recipes().retire_version(draft).await.unwrap_err();
    drop(first);

    assert_eq!(error.code(), ErrorCode::DB_008, "{error:?}");
    let recipe = read_recipe(h, created.id()).await.unwrap();
    assert_eq!(recipe.versions()[0].state(), VersionState::Approved);
    let mut tx = begin(h).await;
    let image = tx.recipes().lock_version(draft).await.unwrap().unwrap();
    assert_eq!(text(&image, "status"), "approved");
    assert_eq!(field(&image, "row_version").as_i64(), Some(2));
}

#[tokio::test]
async fn a_status_write_after_a_change_by_another_transaction_gives_e_db_008_memory() {
    let harness = MemoryHarness::create().await;
    a_status_write_after_a_change_by_another_transaction_gives_e_db_008(&harness).await;
}
