//! Tests of `RecipeReads::exportable_recipes` in memory (design D4, ADR-0109 (j)).

use recipe_domain::export::Sha256;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::VersionState;
use recipe_domain::timestamp::Timestamp;
use time::Duration;

use super::support::{Seed, begin, code, seed_live, seed_recipe, seed_user};
use crate::memory::MemoryStore;
use crate::memory::fixtures::{instant, machine, rid, store, timestamp, vid};
use crate::ports::{ExportableRecipes, Isolation, Transaction};

const APPROVED: &[(u128, VersionState)] = &[(31, VersionState::Approved)];

async fn exportable(store: &MemoryStore, wanted: &str) -> Option<ExportableRecipes> {
    let mut tx = begin(store, Isolation::ReadOnly);
    tx.recipe_reads()
        .exportable_recipes(code(wanted))
        .await
        .unwrap()
}

fn references(found: &ExportableRecipes) -> Vec<String> {
    found
        .recipes
        .iter()
        .map(|row| row.reference.to_string())
        .collect()
}

/// Store the archived machine `n` with the code `wanted`, archived `seconds` after the clock.
fn seed_archived(store: &MemoryStore, (n, wanted): (u128, &str), seconds: i64) {
    let mut archived = machine(n, wanted);
    let later = instant().checked_add(Duration::seconds(seconds)).unwrap();
    let at = Timestamp::from(later);
    archived.archive(at, None).unwrap();
    store.seed_machine(&archived);
}

/// Store a recipe of machine `machine` whose only version is the approved version `version`.
fn seed_approved(
    store: &MemoryStore,
    (recipe, machine): (u128, u128),
    (reference, version): (&str, u128),
) {
    seed_recipe(
        store,
        &Seed {
            recipe,
            machine,
            reference,
            name: "Alpha",
            deleted_at: None,
            versions: &[(version, VersionState::Approved)],
        },
    );
}

#[tokio::test]
async fn exportable_recipes_gives_none_for_a_code_without_a_machine() {
    let store = store();

    assert_eq!(exportable(&store, "M9").await, None);
}

#[tokio::test]
async fn exportable_recipes_prefers_the_live_machine_over_an_archived_machine_with_its_code() {
    let store = store();
    seed_archived(&store, (2, "M1"), 10);
    seed_user(&store, 9, "creator");
    seed_approved(&store, (1, 1), ("R-LIVE", 11));
    seed_approved(&store, (2, 2), ("R-OLD", 21));

    let found = exportable(&store, "M1").await.unwrap();

    assert!(!found.archived);
    assert_eq!(references(&found), ["R-LIVE"]);
}

#[tokio::test]
async fn exportable_recipes_gives_the_recipes_of_an_archived_machine_without_a_live_machine() {
    let store = store();
    seed_archived(&store, (2, "M7"), 10);
    seed_user(&store, 9, "creator");
    seed_approved(&store, (2, 2), ("R-OLD", 21));

    let found = exportable(&store, "M7").await.unwrap();

    assert!(found.archived);
    assert_eq!(found.code, code("M7"));
    assert_eq!(found.platform, PlcPlatform::TwinCat3);
    assert_eq!(references(&found), ["R-OLD"]);
}

#[tokio::test]
async fn exportable_recipes_takes_the_machine_archived_last_when_none_is_live() {
    let store = store();
    seed_archived(&store, (2, "M7"), 10);
    seed_archived(&store, (3, "M7"), 20);
    seed_archived(&store, (4, "M7"), 5);
    seed_user(&store, 9, "creator");
    seed_approved(&store, (2, 2), ("R-TWO", 21));
    seed_approved(&store, (3, 3), ("R-THREE", 31));
    seed_approved(&store, (4, 4), ("R-FOUR", 41));

    let found = exportable(&store, "M7").await.unwrap();

    assert_eq!(references(&found), ["R-THREE"]);
}

#[tokio::test]
async fn exportable_recipes_keeps_only_undeleted_recipes_with_an_approved_version() {
    let store = store();
    seed_user(&store, 9, "creator");
    seed_live(
        &store,
        (1, "R-DRAFT", "Draft"),
        &[(11, VersionState::LiveDraft)],
    );
    seed_live(
        &store,
        (2, "R-RETIRED", "Retired"),
        &[(21, VersionState::Retired)],
    );
    seed_live(&store, (3, "R-DELETED", "Deleted"), APPROVED);
    store.update_state(|state, now| {
        if let Some(row) = state.recipes.get_mut(&rid(3)) {
            row.deleted_at = Some(now);
        }
    });
    seed_live(&store, (4, "R-OK", "Kept"), &[(41, VersionState::Approved)]);

    let found = exportable(&store, "M1").await.unwrap();

    assert_eq!(references(&found), ["R-OK"]);
}

#[tokio::test]
async fn exportable_recipes_returns_the_approved_number_and_the_stored_hash_in_byte_order() {
    let store = store();
    seed_user(&store, 9, "creator");
    seed_live(&store, (1, "R-2", "Two"), &[(11, VersionState::Approved)]);
    seed_live(
        &store,
        (2, "R-10", "Ten"),
        &[(21, VersionState::Retired), (22, VersionState::Approved)],
    );
    seed_live(&store, (3, "A-1", "One"), &[(31, VersionState::Approved)]);
    store.update_state(|state, _| {
        if let Some(row) = state.versions.get_mut(&vid(22)) {
            row.content_sha256 = Some(Sha256::from([7_u8; 32]));
        }
    });

    let found = exportable(&store, "M1").await.unwrap();

    assert_eq!(references(&found), ["A-1", "R-10", "R-2"]);
    let ten = &found.recipes[1];
    assert_eq!(ten.name.as_str(), "Ten");
    assert_eq!(ten.version_number, 2);
    assert_eq!(ten.approved_at, timestamp());
    assert_eq!(ten.content_sha256, Sha256::from([7_u8; 32]));
    assert_eq!(found.recipes[0].content_sha256, Sha256::from([0_u8; 32]));
}

#[tokio::test]
async fn exportable_recipes_gives_an_empty_list_for_a_machine_without_approved_versions() {
    let store = store();

    let found = exportable(&store, "M1").await.unwrap();

    assert_eq!(found.code, code("M1"));
    assert!(found.recipes.is_empty());
}
