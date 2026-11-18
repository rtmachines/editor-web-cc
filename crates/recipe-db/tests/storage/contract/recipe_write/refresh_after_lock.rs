//! A row lock in `READ COMMITTED` reads the latest committed row (ADR-0030, Decision).
//!
//! Transaction A begins. Transaction B approves the draft and commits. Then A takes its lock.
//! PostgreSQL reads the row that B committed. No wait happens, because B has committed already.
//! The memory adapter gives the same rows. These cases run on both adapters.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::{RecipeVersionId, VersionState};

use crate::contract::harness::{Harness, begin};
use crate::contract::images::{field, text};
use crate::contract::recipe_seed::{approve, create_recipe, plant, recipe_id};

/// A recipe with a live draft. Transaction A begins, and transaction B approves the draft.
async fn draft_approved_after_begin(
    h: &dyn Harness,
    n: u128,
) -> (Box<dyn recipe_app::ports::Transaction>, RecipeVersionId) {
    let plant = plant(h, n, &format!("M{n}")).await;
    let created = create_recipe(h, &plant, n, &format!("REF-{n}")).await;
    let draft = created.versions()[0].id();
    let tx = begin(h).await;
    approve(h, draft).await;
    (tx, draft)
}

async fn lock_version_reads_the_row_that_another_transaction_committed(h: &dyn Harness) {
    let (mut tx, draft) = draft_approved_after_begin(h, 31).await;

    let image = tx.recipes().lock_version(draft).await.unwrap().unwrap();
    let error = tx.recipes().discard_version(draft).await.unwrap_err();

    assert_eq!(text(&image, "status"), "approved");
    assert_eq!(field(&image, "row_version").as_i64(), Some(2));
    assert!(
        matches!(&error, recipe_app::error::StorageError::Constraint { code, .. } if *code == ErrorCode::RCP_033),
        "{error:?}"
    );
}

async fn lock_draft_reads_the_row_that_another_transaction_committed(h: &dyn Harness) {
    let (mut tx, draft) = draft_approved_after_begin(h, 32).await;

    let stored = tx.drafts().lock_draft(draft).await.unwrap().unwrap();

    assert_ne!(stored.state, VersionState::LiveDraft);
    assert_eq!(stored.row_version, 2);
}

async fn lock_versions_reads_the_rows_that_another_transaction_committed(h: &dyn Harness) {
    let (mut tx, _draft) = draft_approved_after_begin(h, 33).await;

    let images = tx.recipes().lock_versions(recipe_id(33)).await.unwrap();

    assert_eq!(images.len(), 1);
    assert_eq!(text(&images[0].1, "status"), "approved");
    assert_eq!(field(&images[0].1, "row_version").as_i64(), Some(2));
}

async fn lock_recipe_reads_the_rows_that_another_transaction_committed(h: &dyn Harness) {
    let (mut tx, _draft) = draft_approved_after_begin(h, 34).await;

    let locked = tx
        .recipes()
        .lock_recipe(recipe_id(34))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(locked.state.versions()[0].state(), VersionState::Approved);
}

async fn lock_recipes_of_machine_reads_the_rows_that_another_transaction_committed(
    h: &dyn Harness,
) {
    let plant = plant(h, 35, "M35").await;
    let created = create_recipe(h, &plant, 35, "REF-35").await;
    let mut tx = begin(h).await;
    approve(h, created.versions()[0].id()).await;

    let recipes = tx
        .recipes()
        .lock_recipes_of_machine(plant.machine)
        .await
        .unwrap();

    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].versions()[0].state(), VersionState::Approved);
}

contract_case!(
    lock_version_reads_the_row_that_another_transaction_committed,
    lock_version_reads_the_row_that_another_transaction_committed_memory,
    lock_version_reads_the_row_that_another_transaction_committed_postgres
);
contract_case!(
    lock_draft_reads_the_row_that_another_transaction_committed,
    lock_draft_reads_the_row_that_another_transaction_committed_memory,
    lock_draft_reads_the_row_that_another_transaction_committed_postgres
);
contract_case!(
    lock_versions_reads_the_rows_that_another_transaction_committed,
    lock_versions_reads_the_rows_that_another_transaction_committed_memory,
    lock_versions_reads_the_rows_that_another_transaction_committed_postgres
);
contract_case!(
    lock_recipe_reads_the_rows_that_another_transaction_committed,
    lock_recipe_reads_the_rows_that_another_transaction_committed_memory,
    lock_recipe_reads_the_rows_that_another_transaction_committed_postgres
);
contract_case!(
    lock_recipes_of_machine_reads_the_rows_that_another_transaction_committed,
    lock_recipes_of_machine_reads_the_rows_that_another_transaction_committed_memory,
    lock_recipes_of_machine_reads_the_rows_that_another_transaction_committed_postgres
);
