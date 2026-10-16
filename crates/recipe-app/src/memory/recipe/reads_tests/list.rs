//! Tests of `RecipeReads::list_recipes` in memory (design D5).

use recipe_domain::recipe::VersionState;
use recipe_domain::timestamp::Timestamp;

use super::support::{Seed, begin, code, seed_live, seed_recipe};
use crate::error::StorageError;
use crate::memory::MemoryStore;
use crate::memory::fixtures::{machine, store, text, timestamp};
use crate::ports::{Isolation, RecipeFilter, RecipeSummary, Transaction};

const DRAFT: &[(u128, VersionState)] = &[(11, VersionState::LiveDraft)];

async fn list(
    store: &MemoryStore,
    filter: &RecipeFilter,
    (offset, limit): (u64, u32),
) -> Vec<RecipeSummary> {
    let mut tx = begin(store, Isolation::ReadOnly);
    tx.recipe_reads()
        .list_recipes(filter, offset, limit)
        .await
        .unwrap()
}

fn references(rows: &[RecipeSummary]) -> Vec<String> {
    rows.iter().map(|row| row.reference.to_string()).collect()
}

#[tokio::test]
async fn list_recipes_returns_the_live_recipes_in_the_byte_order_of_the_reference_number() {
    let store = store();
    seed_live(&store, (1, "R-2", "Two"), DRAFT);
    seed_live(&store, (2, "R-10", "Ten"), &[(21, VersionState::LiveDraft)]);
    seed_live(&store, (3, "A-1", "One"), &[(31, VersionState::LiveDraft)]);

    let rows = list(&store, &RecipeFilter::default(), (0, 10)).await;

    assert_eq!(references(&rows), ["A-1", "R-10", "R-2"]);
}

#[tokio::test]
async fn list_recipes_matches_the_text_in_the_reference_or_the_name_without_case() {
    let store = store();
    seed_live(&store, (1, "R-1", "Press Alpha"), DRAFT);
    seed_live(
        &store,
        (2, "ZEBRA", "beta"),
        &[(21, VersionState::LiveDraft)],
    );
    seed_live(
        &store,
        (3, "Q-9", "ALPHA line"),
        &[(31, VersionState::LiveDraft)],
    );
    let by = |needle: &str| RecipeFilter {
        text: Some(text(needle)),
        ..RecipeFilter::default()
    };

    let in_name = list(&store, &by("aLpHa"), (0, 10)).await;
    let in_reference = list(&store, &by("EB"), (0, 10)).await;
    let nowhere = list(&store, &by("gamma"), (0, 10)).await;

    assert_eq!(references(&in_name), ["Q-9", "R-1"]);
    assert_eq!(references(&in_reference), ["ZEBRA"]);
    assert!(nowhere.is_empty());
}

#[tokio::test]
async fn list_recipes_folds_the_case_of_polish_letters() {
    let store = store();
    seed_live(&store, (1, "R-1", "Zażółć gęślą jaźń"), DRAFT);
    let filter = RecipeFilter {
        text: Some(text("ŻÓŁĆ")),
        ..RecipeFilter::default()
    };

    let rows = list(&store, &filter, (0, 10)).await;

    assert_eq!(references(&rows), ["R-1"]);
}

#[tokio::test]
async fn list_recipes_filters_by_the_code_of_the_machine() {
    let store = store();
    store.seed_machine(&machine(2, "M2"));
    seed_live(&store, (1, "R-1", "On M1"), DRAFT);
    seed_recipe(
        &store,
        &Seed {
            recipe: 2,
            machine: 2,
            reference: "R-2",
            name: "On M2",
            deleted_at: None,
            versions: &[(21, VersionState::LiveDraft)],
        },
    );
    let filter = RecipeFilter {
        machine: Some(code("M2")),
        ..RecipeFilter::default()
    };

    let rows = list(&store, &filter, (0, 10)).await;

    assert_eq!(references(&rows), ["R-2"]);
    assert_eq!(rows.first().unwrap().machine_code, code("M2"));
}

#[tokio::test]
async fn list_recipes_keeps_a_recipe_with_a_version_in_the_requested_state() {
    let store = store();
    seed_live(&store, (1, "A-DRAFT", "a"), DRAFT);
    seed_live(
        &store,
        (2, "B-APPROVED", "b"),
        &[(21, VersionState::Approved)],
    );
    seed_live(
        &store,
        (3, "C-RETIRED", "c"),
        &[(31, VersionState::Retired)],
    );
    seed_live(
        &store,
        (4, "D-BOTH", "d"),
        &[
            (41, VersionState::Retired),
            (42, VersionState::Approved),
            (43, VersionState::LiveDraft),
        ],
    );
    seed_live(
        &store,
        (5, "E-DISCARDED", "e"),
        &[(51, VersionState::Discarded)],
    );
    let in_state = |state| RecipeFilter {
        status: Some(state),
        ..RecipeFilter::default()
    };

    let drafts = list(&store, &in_state(VersionState::LiveDraft), (0, 10)).await;
    let approved = list(&store, &in_state(VersionState::Approved), (0, 10)).await;
    let retired = list(&store, &in_state(VersionState::Retired), (0, 10)).await;

    assert_eq!(references(&drafts), ["A-DRAFT", "D-BOTH"]);
    assert_eq!(references(&approved), ["B-APPROVED", "D-BOTH"]);
    assert_eq!(references(&retired), ["C-RETIRED", "D-BOTH"]);
}

#[tokio::test]
async fn list_recipes_hides_a_deleted_recipe_unless_the_filter_includes_it() {
    let store = store();
    seed_live(&store, (1, "A-LIVE", "live"), DRAFT);
    let deleted_at: Option<Timestamp> = Some(timestamp());
    seed_recipe(
        &store,
        &Seed {
            recipe: 2,
            machine: 1,
            reference: "B-GONE",
            name: "gone",
            deleted_at,
            versions: &[(21, VersionState::Discarded)],
        },
    );
    let with_deleted = RecipeFilter {
        include_deleted: true,
        ..RecipeFilter::default()
    };

    let hidden = list(&store, &RecipeFilter::default(), (0, 10)).await;
    let shown = list(&store, &with_deleted, (0, 10)).await;

    assert_eq!(references(&hidden), ["A-LIVE"]);
    assert_eq!(references(&shown), ["A-LIVE", "B-GONE"]);
    assert_eq!(
        shown.iter().map(|row| row.deleted).collect::<Vec<_>>(),
        [false, true]
    );
}

#[tokio::test]
async fn list_recipes_reports_the_numbers_of_the_approved_version_and_the_live_draft() {
    let store = store();
    seed_live(
        &store,
        (1, "A-FULL", "full"),
        &[
            (11, VersionState::Retired),
            (12, VersionState::Approved),
            (13, VersionState::LiveDraft),
        ],
    );
    seed_live(
        &store,
        (2, "B-OLD", "old"),
        &[(21, VersionState::Retired), (22, VersionState::Discarded)],
    );

    let rows = list(&store, &RecipeFilter::default(), (0, 10)).await;

    let numbers: Vec<_> = rows.iter().map(|row| (row.approved, row.draft)).collect();
    assert_eq!(numbers, [(Some(2), Some(3)), (None, None)]);
}

#[tokio::test]
async fn list_recipes_returns_the_page_that_offset_and_limit_name() {
    let store = store();
    for (n, name) in [(1, "R-1"), (2, "R-2"), (3, "R-3"), (4, "R-4")] {
        seed_live(
            &store,
            (n, name, "n"),
            &[(n.checked_mul(10).unwrap(), VersionState::LiveDraft)],
        );
    }

    let second_page = list(&store, &RecipeFilter::default(), (2, 2)).await;
    let past_the_end = list(&store, &RecipeFilter::default(), (9, 2)).await;
    let no_rows = list(&store, &RecipeFilter::default(), (0, 0)).await;

    assert_eq!(references(&second_page), ["R-3", "R-4"]);
    assert!(past_the_end.is_empty());
    assert!(no_rows.is_empty());
}

#[tokio::test]
async fn list_recipes_with_an_offset_above_i64_max_is_a_storage_error_as_in_the_sql_adapter() {
    let store = store();
    seed_live(&store, (1, "R-1", "One"), DRAFT);
    let mut tx = begin(&store, Isolation::ReadOnly);
    let largest = u64::try_from(i64::MAX).unwrap();

    let at_limit = tx
        .recipe_reads()
        .list_recipes(&RecipeFilter::default(), largest, 10)
        .await;
    let beyond = tx
        .recipe_reads()
        .list_recipes(
            &RecipeFilter::default(),
            largest.checked_add(1).unwrap(),
            10,
        )
        .await;

    assert!(at_limit.unwrap().is_empty());
    assert!(matches!(
        beyond.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}
