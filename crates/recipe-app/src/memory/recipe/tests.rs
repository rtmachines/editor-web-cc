use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::Sha256;
use recipe_domain::plant::UnitUsage;
use recipe_domain::recipe::VersionState;
use serde_json::{Value, json};

use crate::error::{RECIPE_REFERENCE_UNIQUE, RECIPE_VERSION_ONE_LIVE_DRAFT, StorageError};
use crate::memory::fixtures::{
    draft_recipe, field, lock_line, locks, long_text, mid, now_text, param, pid, pkey, reference,
    rid, store, text, uid, user, vid,
};
use crate::memory::{LockMode, MemoryStore, MemoryTransaction};
use crate::ports::{Isolation, NewVersion, Transaction};

const HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn begin(store: &MemoryStore) -> MemoryTransaction {
    store.begin_memory(Isolation::ReadCommitted).unwrap()
}

fn new_version(r: u128, v: u128, number: u32, based_on: Option<u128>) -> NewVersion {
    NewVersion {
        recipe: rid(r),
        version: vid(v),
        number,
        based_on: based_on.map(vid),
        change_note: long_text("Note"),
        creator: user(9),
    }
}

/// A store with the recipe R1 whose only version V1 is a live draft.
fn store_with_draft() -> MemoryStore {
    let store = store();
    store.seed_recipe(
        &draft_recipe((1, 1), "R-1", 11),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        user(9),
    );
    store
}

/// A store with the recipe R1: V1 approved, V2 retired is not used here.
fn store_with_approved() -> MemoryStore {
    let store = store();
    store.seed_recipe(
        &crate::memory::fixtures::recipe_with((1, 1), "R-1", &[(11, VersionState::Approved)]),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        user(9),
    );
    store
}

fn recipe_image(r: u128, name: &str, deleted_at: Value) -> Value {
    json!({
        "id": rid(r).to_string(),
        "machine_id": mid(1).to_string(),
        "plc_platform": "TwinCat3",
        "reference_number": "R-1",
        "name": name,
        "deleted_at": deleted_at,
        "created_by": user(9).to_string(),
        "created_at": now_text(),
        "updated_at": now_text(),
    })
}

#[tokio::test]
async fn lock_recipe_takes_for_update_and_returns_the_row_before_the_change() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let locked = tx.recipes().lock_recipe(rid(1)).await.unwrap().unwrap();
    assert_eq!(locked.state.id(), rid(1));
    assert_eq!(
        locked.before.as_value(),
        &recipe_image(1, "Recipe", Value::Null)
    );
    assert_eq!(
        locks(&store),
        vec![lock_line("recipe", LockMode::ForUpdate, rid(1).to_string())]
    );
}

#[tokio::test]
async fn lock_recipe_of_a_missing_recipe_gives_none_and_no_journal_line() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    assert!(tx.recipes().lock_recipe(rid(2)).await.unwrap().is_none());
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn lock_recipes_of_machine_orders_by_id_and_skips_other_machines() {
    let store = store();
    store.seed_machine(&crate::memory::fixtures::machine(2, "M2"));
    for (r, m, name, v) in [
        (3, 1, "R-3", 13),
        (1, 1, "R-1", 11),
        (2, 1, "R-2", 12),
        (4, 2, "R-4", 14),
    ] {
        store.seed_recipe(
            &draft_recipe((r, m), name, v),
            recipe_domain::plant::PlcPlatform::TwinCat3,
            user(9),
        );
    }
    let mut tx = begin(&store);
    let found = tx.recipes().lock_recipes_of_machine(mid(1)).await.unwrap();
    let ids: Vec<_> = found
        .iter()
        .map(recipe_domain::recipe::Recipe::id)
        .collect();
    assert_eq!(ids, vec![rid(1), rid(2), rid(3)]);
    assert_eq!(
        locks(&store),
        vec![
            lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
            lock_line("recipe", LockMode::ForUpdate, rid(2).to_string()),
            lock_line("recipe", LockMode::ForUpdate, rid(3).to_string()),
        ]
    );
}

#[tokio::test]
async fn lock_versions_orders_by_id_and_returns_the_images_before_the_change() {
    let store = store();
    store.seed_recipe(
        &crate::memory::fixtures::recipe_with(
            (1, 1),
            "R-1",
            &[
                (30, VersionState::Retired),
                (10, VersionState::Retired),
                (20, VersionState::LiveDraft),
            ],
        ),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        user(9),
    );
    let mut tx = begin(&store);
    let versions = tx.recipes().lock_versions(rid(1)).await.unwrap();
    let ids: Vec<_> = versions.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![vid(10), vid(20), vid(30)]);
    let numbers: Vec<Value> = versions
        .iter()
        .map(|(_, image)| field(image.as_value(), "number"))
        .collect();
    assert_eq!(numbers, vec![json!(2), json!(3), json!(1)]);
    let expected: Vec<_> = [10, 20, 30]
        .into_iter()
        .map(|v| lock_line("recipe_version", LockMode::ForUpdate, vid(v).to_string()))
        .collect();
    assert_eq!(locks(&store), expected);
}

#[tokio::test]
async fn lock_version_returns_the_row_before_the_change() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let image = tx.recipes().lock_version(vid(11)).await.unwrap().unwrap();
    let value = image.as_value();
    assert_eq!(field(value, "id"), json!(vid(11).to_string()));
    assert_eq!(field(value, "recipe_id"), json!(rid(1).to_string()));
    assert_eq!(field(value, "plc_platform"), json!("TwinCat3"));
    assert_eq!(field(value, "number"), json!(1));
    assert_eq!(field(value, "status"), json!("draft"));
    assert_eq!(field(value, "row_version"), json!(1));
    assert_eq!(field(value, "deleted_at"), Value::Null);
    assert_eq!(field(value, "content_sha256"), Value::Null);
    assert_eq!(field(value, "created_by"), json!(user(9).to_string()));
    assert_eq!(
        locks(&store),
        vec![lock_line(
            "recipe_version",
            LockMode::ForUpdate,
            vid(11).to_string()
        )]
    );
}

#[tokio::test]
async fn lock_version_of_a_missing_version_gives_none_and_no_journal_line() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    assert!(tx.recipes().lock_version(vid(99)).await.unwrap().is_none());
    assert!(
        tx.recipes()
            .lock_version_shared(vid(99))
            .await
            .unwrap()
            .is_none()
    );
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn lock_version_shared_takes_for_share_and_returns_the_stored_hash() {
    let store = store_with_approved();
    let mut tx = begin(&store);
    let shared = tx
        .recipes()
        .lock_version_shared(vid(11))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(shared.recipe, rid(1));
    assert_eq!(shared.number, 1);
    assert_eq!(shared.state, VersionState::Approved);
    assert_eq!(shared.content_sha256, Some(Sha256::from([0_u8; 32])));
    assert_eq!(
        locks(&store),
        vec![lock_line(
            "recipe_version",
            LockMode::ForShare,
            vid(11).to_string()
        )]
    );
}

#[tokio::test]
async fn a_recipe_is_found_by_reference_and_by_id_also_when_deleted() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    tx.recipes().lock_recipe(rid(1)).await.unwrap();
    tx.recipes().lock_versions(rid(1)).await.unwrap();
    tx.recipes().discard_version(vid(11)).await.unwrap();
    tx.recipes().mark_recipe_deleted(rid(1)).await.unwrap();
    let by_reference = tx
        .recipes()
        .recipe_by_reference(&reference("R-1"))
        .await
        .unwrap();
    let by_id = tx.recipes().recipe_by_id(rid(1)).await.unwrap();
    assert!(by_reference.is_some_and(|recipe| recipe.is_deleted()));
    assert!(by_id.is_some_and(|recipe| recipe.is_deleted()));
    assert!(
        tx.recipes()
            .recipe_by_reference(&reference("R-2"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(tx.recipes().recipe_by_id(rid(2)).await.unwrap().is_none());
}

#[tokio::test]
async fn insert_recipe_returns_the_images_of_the_recipe_and_of_version_1() {
    let store = store();
    let mut tx = begin(&store);
    let recipe = draft_recipe((1, 1), "R-1", 11);
    let inserted = tx
        .recipes()
        .insert_recipe(&recipe, new_version(1, 11, 1, None))
        .await
        .unwrap();
    assert_eq!(
        inserted.recipe.as_value(),
        &recipe_image(1, "Recipe", Value::Null)
    );
    let version = inserted.version.as_value();
    assert_eq!(field(version, "number"), json!(1));
    assert_eq!(field(version, "status"), json!("draft"));
    assert_eq!(field(version, "row_version"), json!(1));
    assert_eq!(field(version, "change_note"), json!("Note"));
    assert_eq!(field(version, "based_on_version_id"), Value::Null);
    assert_eq!(field(version, "created_at"), json!(now_text()));
    let read = tx
        .recipes()
        .recipe_by_reference(&reference("R-1"))
        .await
        .unwrap();
    assert_eq!(read, Some(recipe));
}

#[tokio::test]
async fn insert_recipe_with_a_known_reference_gives_recipe_reference_unique() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let error = tx
        .recipes()
        .insert_recipe(
            &draft_recipe((2, 1), "R-1", 12),
            new_version(2, 12, 1, None),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == RECIPE_REFERENCE_UNIQUE),
        "{error:?}"
    );
}

#[tokio::test]
async fn insert_recipe_for_an_unknown_machine_gives_a_missing_reference() {
    let store = store();
    let mut tx = begin(&store);
    let error = tx
        .recipes()
        .insert_recipe(
            &draft_recipe((1, 7), "R-1", 11),
            new_version(1, 11, 1, None),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, StorageError::MissingReference { .. }),
        "{error:?}"
    );
}

#[tokio::test]
async fn rename_delete_and_restore_return_the_recipe_row_after_the_change() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let renamed = tx
        .recipes()
        .rename_recipe(rid(1), &text("New"))
        .await
        .unwrap();
    assert_eq!(renamed.as_value(), &recipe_image(1, "New", Value::Null));
    let deleted = tx.recipes().mark_recipe_deleted(rid(1)).await.unwrap();
    assert_eq!(
        deleted.as_value(),
        &recipe_image(1, "New", json!(now_text()))
    );
    let restored = tx.recipes().restore_recipe(rid(1)).await.unwrap();
    assert_eq!(restored.as_value(), &recipe_image(1, "New", Value::Null));
}

#[tokio::test]
async fn a_recipe_write_for_a_missing_recipe_is_a_storage_error() {
    let store = store();
    let mut tx = begin(&store);
    assert!(
        tx.recipes()
            .rename_recipe(rid(1), &text("New"))
            .await
            .is_err()
    );
    assert!(tx.recipes().mark_recipe_deleted(rid(1)).await.is_err());
    assert!(tx.recipes().restore_recipe(rid(1)).await.is_err());
}

#[tokio::test]
async fn insert_version_copies_the_keys_limits_and_translations_with_new_parameter_ids() {
    let store = store_with_approved();
    store.seed_parameter(vid(11), &param(1, 101, "A"));
    store.seed_parameter(vid(11), &param(2, 101, "B"));
    let mut tx = begin(&store);
    let image = tx
        .recipes()
        .insert_version(new_version(1, 12, 2, Some(11)))
        .await
        .unwrap();
    assert_eq!(
        field(image.as_value(), "based_on_version_id"),
        json!(vid(11).to_string())
    );
    assert_eq!(field(image.as_value(), "number"), json!(2));
    let base = tx.recipes().version_parameters(vid(11)).await.unwrap();
    let copy = tx.recipes().version_parameters(vid(12)).await.unwrap();
    assert_eq!(copy.len(), 2);
    for (from, to) in base.iter().zip(&copy) {
        assert_eq!(from.key, to.key);
        assert_ne!(from.id, to.id);
        assert_eq!(from.tag, to.tag);
        assert_eq!(from.limits, to.limits);
        assert_eq!(from.translations, to.translations);
        assert_eq!(from.value, to.value);
    }
    assert!(base.iter().all(|p| p.id == pid(1) || p.id == pid(2)));
}

#[tokio::test]
async fn a_second_live_draft_gives_recipe_version_one_live_draft() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let error = tx
        .recipes()
        .insert_version(new_version(1, 12, 2, Some(11)))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == RECIPE_VERSION_ONE_LIVE_DRAFT),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_known_number_gives_recipe_version_number_unique() {
    let store = store_with_approved();
    let mut tx = begin(&store);
    let error = tx
        .recipes()
        .insert_version(new_version(1, 12, 1, Some(11)))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == "recipe_version_number_unique"),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_base_version_of_another_recipe_gives_a_missing_reference() {
    let store = store_with_approved();
    store.seed_recipe(
        &crate::memory::fixtures::recipe_with((2, 1), "R-2", &[(21, VersionState::Approved)]),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        user(9),
    );
    let mut tx = begin(&store);
    let error = tx
        .recipes()
        .insert_version(new_version(1, 12, 2, Some(21)))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StorageError::MissingReference { .. }),
        "{error:?}"
    );
}

#[tokio::test]
async fn discard_version_sets_deleted_at_and_adds_1_to_the_row_version() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let image = tx.recipes().discard_version(vid(11)).await.unwrap();
    let value = image.as_value();
    assert_eq!(field(value, "deleted_at"), json!(now_text()));
    assert_eq!(field(value, "status"), json!("draft"));
    assert_eq!(field(value, "row_version"), json!(2));
}

#[tokio::test]
async fn discard_version_of_an_approved_version_gives_e_rcp_033() {
    let store = store_with_approved();
    let mut tx = begin(&store);
    let error = tx.recipes().discard_version(vid(11)).await.unwrap_err();
    assert!(
        matches!(&error, StorageError::Constraint { code, .. } if *code == ErrorCode::RCP_033),
        "{error:?}"
    );
}

#[tokio::test]
async fn retire_version_sets_the_status_and_adds_1_to_the_row_version() {
    let store = store_with_approved();
    let mut tx = begin(&store);
    let image = tx.recipes().retire_version(vid(11)).await.unwrap();
    let value = image.as_value();
    assert_eq!(field(value, "status"), json!("retired"));
    assert_eq!(field(value, "retired_at"), json!(now_text()));
    assert_eq!(field(value, "row_version"), json!(2));
}

#[tokio::test]
async fn retire_version_of_a_draft_gives_e_rcp_033() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let error = tx.recipes().retire_version(vid(11)).await.unwrap_err();
    assert!(
        matches!(&error, StorageError::Constraint { code, .. } if *code == ErrorCode::RCP_033),
        "{error:?}"
    );
}

#[tokio::test]
async fn approve_version_stores_the_hash_as_64_hex_characters() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let hash = Sha256::parse(HASH).unwrap();
    let image = tx
        .recipes()
        .approve_version(vid(11), user(5), hash)
        .await
        .unwrap();
    let value = image.as_value();
    assert_eq!(field(value, "status"), json!("approved"));
    assert_eq!(field(value, "content_sha256"), json!(HASH));
    assert_eq!(field(value, "approved_by"), json!(user(5).to_string()));
    assert_eq!(field(value, "approved_at"), json!(now_text()));
    assert_eq!(field(value, "row_version"), json!(2));
}

#[tokio::test]
async fn a_second_approved_version_gives_recipe_version_one_approved() {
    let store = store();
    store.seed_recipe(
        &crate::memory::fixtures::recipe_with(
            (1, 1),
            "R-1",
            &[(11, VersionState::Approved), (12, VersionState::LiveDraft)],
        ),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        user(9),
    );
    let mut tx = begin(&store);
    let hash = Sha256::parse(HASH).unwrap();
    let error = tx
        .recipes()
        .approve_version(vid(12), user(5), hash)
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == "recipe_version_one_approved"),
        "{error:?}"
    );
}

#[tokio::test]
async fn version_parameters_are_ordered_by_tag_and_hold_limits_and_translations() {
    let store = store_with_draft();
    store.seed_parameter(vid(11), &param(1, 101, "B"));
    store.seed_parameter(vid(11), &param(2, 101, "A"));
    let mut tx = begin(&store);
    let found = tx.recipes().version_parameters(vid(11)).await.unwrap();
    let tags: Vec<String> = found.iter().map(|p| p.tag.to_string()).collect();
    assert_eq!(tags, vec!["A".to_owned(), "B".to_owned()]);
    assert_eq!(found.first().map(|p| p.key), Some(pkey(2)));
    assert_eq!(found.first().map(|p| p.limits.len()), Some(1));
    assert_eq!(found.first().map(|p| p.translations.len()), Some(1));
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn a_rollback_keeps_no_recipe_change() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    tx.recipes()
        .insert_recipe(
            &draft_recipe((2, 1), "R-2", 12),
            new_version(2, 12, 1, None),
        )
        .await
        .unwrap();
    tx.recipes()
        .rename_recipe(rid(1), &text("Gone"))
        .await
        .unwrap();
    tx.recipes().discard_version(vid(11)).await.unwrap();
    drop(tx);
    let state = store.state();
    assert!(state.recipe(rid(2)).is_none());
    assert_eq!(
        state.recipe(rid(1)).map(|r| r.name().to_string()),
        Some("Recipe".to_owned())
    );
    assert_eq!(state.version_row_version(vid(11)), Some(1));
}

async fn usage_of(store: &MemoryStore, units: &[u128]) -> UnitUsage {
    let mut tx = store.begin_memory(Isolation::ReadOnly).unwrap();
    let units: Vec<_> = units.iter().copied().map(uid).collect();
    tx.plant().unit_usage(mid(1), &units).await.unwrap()
}

#[tokio::test]
async fn unit_usage_finds_a_live_draft_and_an_approved_version() {
    let store = store();
    store.seed_recipe(
        &draft_recipe((1, 1), "R-1", 11),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        user(9),
    );
    store.seed_recipe(
        &crate::memory::fixtures::recipe_with((2, 1), "R-2", &[(21, VersionState::Approved)]),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        user(9),
    );
    store.seed_parameter(vid(11), &param(1, 101, "A"));
    store.seed_parameter(vid(21), &param(2, 102, "A"));
    let mut expected = UnitUsage::default();
    expected.add_draft(uid(101), reference("R-1"));
    expected.add_approved(uid(102), reference("R-2"));
    assert_eq!(usage_of(&store, &[101, 102, 103]).await, expected);
}

#[tokio::test]
async fn unit_usage_ignores_a_retired_version_and_a_discarded_draft() {
    let store = store();
    store.seed_recipe(
        &crate::memory::fixtures::recipe_with(
            (1, 1),
            "R-1",
            &[(11, VersionState::Retired), (12, VersionState::Discarded)],
        ),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        user(9),
    );
    store.seed_parameter(vid(11), &param(1, 101, "A"));
    store.seed_parameter(vid(12), &param(2, 102, "A"));
    assert_eq!(usage_of(&store, &[101, 102]).await, UnitUsage::default());
}

#[tokio::test]
async fn unit_usage_ignores_units_that_the_caller_did_not_ask_for() {
    let store = store_with_draft();
    store.seed_parameter(vid(11), &param(1, 101, "A"));
    assert_eq!(usage_of(&store, &[102]).await, UnitUsage::default());
}
