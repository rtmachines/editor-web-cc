//! Contract cases of `discard_version`, `retire_version`, and `approve_version`.

use recipe_app::error::StorageError;
use recipe_domain::recipe::{RecipeVersionId, VersionState};

use crate::contract::harness::{Harness, begin, commit, fixture_hash};
use crate::contract::images::{Stage, assert_shape, field, text, version_shape};
use crate::contract::recipe_seed::{
    approve, assert_illegal_transition, create_recipe, create_version, plant, read_recipe, retire,
    version_id,
};

/// A recipe with a live draft.
async fn draft_recipe(
    h: &dyn Harness,
    n: u128,
) -> (crate::contract::recipe_seed::Plant, RecipeVersionId) {
    let plant = plant(h, n, &format!("M{n}")).await;
    let created = create_recipe(h, &plant, n, &format!("REF-{n}")).await;
    (plant, created.versions()[0].id())
}

async fn discard_version_gives_the_image_with_the_deletion_mark(h: &dyn Harness) {
    let (plant, draft) = draft_recipe(h, 1).await;
    let mut tx = begin(h).await;
    let before = tx.recipes().lock_version(draft).await.unwrap().unwrap();

    let image = tx.recipes().discard_version(draft).await.unwrap();
    commit(h, tx, plant.machine).await;

    assert_shape(&image, &version_shape(false, Stage::Discarded));
    assert_eq!(text(&image, "status"), "draft");
    assert_eq!(
        field(&image, "row_version").as_i64(),
        field(&before, "row_version")
            .as_i64()
            .and_then(|v| v.checked_add(1))
    );
    let recipe = read_recipe(h, crate::contract::recipe_seed::recipe_id(1))
        .await
        .unwrap();
    assert_eq!(recipe.versions()[0].state(), VersionState::Discarded);
}

async fn approve_version_of_a_locked_draft_adds_one_to_the_row_version(h: &dyn Harness) {
    let (plant, draft) = draft_recipe(h, 2).await;
    let mut tx = begin(h).await;
    let before = tx.recipes().lock_version(draft).await.unwrap().unwrap();

    let image = tx
        .recipes()
        .approve_version(draft, h.user(), fixture_hash())
        .await
        .unwrap();
    commit(h, tx, plant.machine).await;

    assert_shape(&image, &version_shape(false, Stage::Approved));
    assert_eq!(field(&before, "row_version").as_i64(), Some(1));
    assert_eq!(field(&image, "row_version").as_i64(), Some(2));
    assert_eq!(text(&image, "status"), "approved");
    assert_eq!(text(&image, "approved_by"), h.user().to_string());
    assert_eq!(text(&image, "content_sha256"), fixture_hash().to_string());
    let mut tx = begin(h).await;
    let shared = tx
        .recipes()
        .lock_version_shared(draft)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(shared.state, VersionState::Approved);
    assert_eq!(shared.approved_by, Some(h.user()));
    assert_eq!(shared.content_sha256, Some(fixture_hash()));
}

async fn retire_version_gives_the_retired_image(h: &dyn Harness) {
    let (plant, version) = draft_recipe(h, 3).await;
    approve(h, version).await;
    let mut tx = begin(h).await;
    let before = tx.recipes().lock_version(version).await.unwrap().unwrap();

    let image = tx.recipes().retire_version(version).await.unwrap();
    commit(h, tx, plant.machine).await;

    assert_shape(&image, &version_shape(false, Stage::Retired));
    assert_eq!(text(&image, "status"), "retired");
    assert_eq!(field(&before, "row_version").as_i64(), Some(2));
    assert_eq!(field(&image, "row_version").as_i64(), Some(3));
    assert_eq!(text(&image, "approved_by"), h.user().to_string());
}

async fn approval_retires_the_approved_version_first_and_keeps_one_approved(h: &dyn Harness) {
    let (plant, first) = draft_recipe(h, 4).await;
    approve(h, first).await;
    let second = version_id(1);
    create_version(
        h,
        (crate::contract::recipe_seed::recipe_id(4), second),
        2,
        Some(first),
    )
    .await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(second).await.unwrap().unwrap();
    tx.recipes().lock_version(first).await.unwrap().unwrap();

    tx.recipes().retire_version(first).await.unwrap();
    let approved = tx
        .recipes()
        .approve_version(second, h.user(), fixture_hash())
        .await
        .unwrap();
    commit(h, tx, plant.machine).await;

    assert_eq!(text(&approved, "status"), "approved");
    let recipe = read_recipe(h, crate::contract::recipe_seed::recipe_id(4))
        .await
        .unwrap();
    let states: Vec<VersionState> = recipe.versions().iter().map(|v| v.state()).collect();
    assert_eq!(states, vec![VersionState::Retired, VersionState::Approved]);
}

async fn a_second_approved_version_gives_the_constraint_of_the_approved_index(h: &dyn Harness) {
    let (_plant, first) = draft_recipe(h, 5).await;
    approve(h, first).await;
    let second = version_id(1);
    create_version(
        h,
        (crate::contract::recipe_seed::recipe_id(5), second),
        2,
        Some(first),
    )
    .await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(second).await.unwrap().unwrap();

    let error = tx
        .recipes()
        .approve_version(second, h.user(), fixture_hash())
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint }
            if constraint == "recipe_version_one_approved"),
        "{error:?}"
    );
}

async fn the_discard_of_an_approved_version_gives_e_rcp_033(h: &dyn Harness) {
    let (_plant, version) = draft_recipe(h, 6).await;
    approve(h, version).await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(version).await.unwrap().unwrap();

    let error = tx.recipes().discard_version(version).await.unwrap_err();

    assert_illegal_transition(&error);
}

async fn the_discard_of_a_retired_version_gives_e_rcp_033(h: &dyn Harness) {
    let (_plant, version) = draft_recipe(h, 7).await;
    approve(h, version).await;
    retire(h, version).await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(version).await.unwrap().unwrap();

    let error = tx.recipes().discard_version(version).await.unwrap_err();

    assert_illegal_transition(&error);
}

async fn the_retirement_of_a_draft_gives_e_rcp_033(h: &dyn Harness) {
    let (_plant, version) = draft_recipe(h, 8).await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(version).await.unwrap().unwrap();

    let error = tx.recipes().retire_version(version).await.unwrap_err();

    assert_illegal_transition(&error);
}

async fn the_approval_of_an_approved_version_gives_e_rcp_033(h: &dyn Harness) {
    let (_plant, version) = draft_recipe(h, 9).await;
    approve(h, version).await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(version).await.unwrap().unwrap();

    let error = tx
        .recipes()
        .approve_version(version, h.user(), fixture_hash())
        .await
        .unwrap_err();

    assert_illegal_transition(&error);
}

async fn the_approval_of_a_discarded_draft_gives_e_rcp_033(h: &dyn Harness) {
    let (plant, version) = draft_recipe(h, 10).await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(version).await.unwrap().unwrap();
    tx.recipes().discard_version(version).await.unwrap();
    commit(h, tx, plant.machine).await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(version).await.unwrap().unwrap();

    let error = tx
        .recipes()
        .approve_version(version, h.user(), fixture_hash())
        .await
        .unwrap_err();

    assert_illegal_transition(&error);
}

async fn two_status_writes_after_one_lock_both_pass_the_guard(h: &dyn Harness) {
    let (plant, version) = draft_recipe(h, 11).await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(version).await.unwrap().unwrap();

    tx.recipes()
        .approve_version(version, h.user(), fixture_hash())
        .await
        .unwrap();
    let retired = tx.recipes().retire_version(version).await.unwrap();
    commit(h, tx, plant.machine).await;

    assert_shape(&retired, &version_shape(false, Stage::Retired));
    assert_eq!(field(&retired, "row_version").as_i64(), Some(3));
}

contract_case!(
    discard_version_gives_the_image_with_the_deletion_mark,
    discard_version_gives_the_image_with_the_deletion_mark_memory,
    discard_version_gives_the_image_with_the_deletion_mark_postgres
);
contract_case!(
    approve_version_of_a_locked_draft_adds_one_to_the_row_version,
    approve_version_of_a_locked_draft_adds_one_to_the_row_version_memory,
    approve_version_of_a_locked_draft_adds_one_to_the_row_version_postgres
);
contract_case!(
    retire_version_gives_the_retired_image,
    retire_version_gives_the_retired_image_memory,
    retire_version_gives_the_retired_image_postgres
);
contract_case!(
    approval_retires_the_approved_version_first_and_keeps_one_approved,
    approval_retires_the_approved_version_first_and_keeps_one_approved_memory,
    approval_retires_the_approved_version_first_and_keeps_one_approved_postgres
);
contract_case!(
    a_second_approved_version_gives_the_constraint_of_the_approved_index,
    a_second_approved_version_gives_the_constraint_of_the_approved_index_memory,
    a_second_approved_version_gives_the_constraint_of_the_approved_index_postgres
);
contract_case!(
    the_discard_of_an_approved_version_gives_e_rcp_033,
    the_discard_of_an_approved_version_gives_e_rcp_033_memory,
    the_discard_of_an_approved_version_gives_e_rcp_033_postgres
);
contract_case!(
    the_discard_of_a_retired_version_gives_e_rcp_033,
    the_discard_of_a_retired_version_gives_e_rcp_033_memory,
    the_discard_of_a_retired_version_gives_e_rcp_033_postgres
);
contract_case!(
    the_retirement_of_a_draft_gives_e_rcp_033,
    the_retirement_of_a_draft_gives_e_rcp_033_memory,
    the_retirement_of_a_draft_gives_e_rcp_033_postgres
);
contract_case!(
    the_approval_of_an_approved_version_gives_e_rcp_033,
    the_approval_of_an_approved_version_gives_e_rcp_033_memory,
    the_approval_of_an_approved_version_gives_e_rcp_033_postgres
);
contract_case!(
    the_approval_of_a_discarded_draft_gives_e_rcp_033,
    the_approval_of_a_discarded_draft_gives_e_rcp_033_memory,
    the_approval_of_a_discarded_draft_gives_e_rcp_033_postgres
);
contract_case!(
    two_status_writes_after_one_lock_both_pass_the_guard,
    two_status_writes_after_one_lock_both_pass_the_guard_memory,
    two_status_writes_after_one_lock_both_pass_the_guard_postgres
);
