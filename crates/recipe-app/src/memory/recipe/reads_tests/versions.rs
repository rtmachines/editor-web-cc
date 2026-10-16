//! Tests of `RecipeReads::version_page` and `version_detail` in memory (design D5).

use recipe_domain::recipe::VersionState;

use super::support::{begin, seed_live, seed_user};
use crate::error::StorageError;
use crate::memory::fixtures::{long_text, rid, store, timestamp, vid};
use crate::ports::{Isolation, Transaction};

const FOUR_STATES: &[(u128, VersionState)] = &[
    (11, VersionState::Retired),
    (12, VersionState::Approved),
    (13, VersionState::Discarded),
    (14, VersionState::LiveDraft),
];

#[tokio::test]
async fn version_page_returns_the_newest_number_first_with_its_state_and_times() {
    let store = store();
    seed_live(&store, (1, "R-1", "One"), FOUR_STATES);
    seed_live(&store, (2, "R-2", "Two"), &[(21, VersionState::LiveDraft)]);
    let mut tx = begin(&store, Isolation::ReadOnly);

    let rows = tx.recipe_reads().version_page(rid(1), 0, 10).await.unwrap();

    let seen: Vec<_> = rows.iter().map(|row| (row.number, row.state)).collect();
    assert_eq!(
        seen,
        [
            (4, VersionState::LiveDraft),
            (3, VersionState::Discarded),
            (2, VersionState::Approved),
            (1, VersionState::Retired),
        ]
    );
    let approved = rows.get(2).unwrap();
    assert_eq!(approved.change_note, long_text("Seeded version"));
    assert_eq!(approved.created_at, timestamp());
    assert_eq!(approved.approved_at, Some(timestamp()));
    assert_eq!(rows.first().unwrap().approved_at, None);
}

#[tokio::test]
async fn version_page_returns_the_page_that_offset_and_limit_name() {
    let store = store();
    seed_live(&store, (1, "R-1", "One"), FOUR_STATES);
    let mut tx = begin(&store, Isolation::ReadOnly);

    let page = tx.recipe_reads().version_page(rid(1), 1, 2).await.unwrap();
    let past_the_end = tx.recipe_reads().version_page(rid(1), 9, 2).await.unwrap();

    assert_eq!(
        page.iter().map(|row| row.number).collect::<Vec<_>>(),
        [3, 2]
    );
    assert!(past_the_end.is_empty());
}

#[tokio::test]
async fn version_page_of_an_unknown_recipe_is_empty() {
    let store = store();
    let mut tx = begin(&store, Isolation::ReadOnly);

    let rows = tx.recipe_reads().version_page(rid(7), 0, 10).await.unwrap();

    assert!(rows.is_empty());
}

#[tokio::test]
async fn version_page_with_an_offset_above_i64_max_is_a_storage_error() {
    let store = store();
    seed_live(&store, (1, "R-1", "One"), FOUR_STATES);
    let mut tx = begin(&store, Isolation::ReadOnly);
    let beyond = u64::try_from(i64::MAX).unwrap().checked_add(1).unwrap();

    let error = tx.recipe_reads().version_page(rid(1), beyond, 10).await;

    assert!(matches!(
        error.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}

#[tokio::test]
async fn version_detail_returns_the_header_the_logins_and_the_times() {
    let store = store();
    seed_user(&store, 9, "jkowalski");
    seed_live(&store, (1, "R-1", "One"), FOUR_STATES);
    let mut tx = begin(&store, Isolation::RepeatableRead);

    let approved = tx
        .recipe_reads()
        .version_detail(vid(12))
        .await
        .unwrap()
        .unwrap();
    let draft = tx
        .recipe_reads()
        .version_detail(vid(14))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(approved.id, vid(12));
    assert_eq!(approved.recipe, rid(1));
    assert_eq!(approved.number, 2);
    assert_eq!(approved.state, VersionState::Approved);
    assert_eq!(approved.change_note, long_text("Seeded version"));
    assert_eq!(approved.created_by.as_str(), "jkowalski");
    assert_eq!(
        approved.approved_by.as_ref().map(|login| login.as_str()),
        Some("jkowalski")
    );
    assert_eq!(approved.created_at, timestamp());
    assert_eq!(approved.approved_at, Some(timestamp()));
    assert_eq!(approved.updated_at, timestamp());
    assert_eq!(approved.row_version, 1);
    assert_eq!(draft.state, VersionState::LiveDraft);
    assert_eq!(draft.approved_by, None);
    assert_eq!(draft.approved_at, None);
}

#[tokio::test]
async fn version_detail_of_an_unknown_version_is_none() {
    let store = store();
    let mut tx = begin(&store, Isolation::ReadOnly);

    let found = tx.recipe_reads().version_detail(vid(99)).await.unwrap();

    assert!(found.is_none());
}

#[tokio::test]
async fn version_detail_with_a_creator_that_has_no_user_row_is_a_storage_error() {
    let store = store();
    seed_live(&store, (1, "R-1", "One"), FOUR_STATES);
    let mut tx = begin(&store, Isolation::ReadOnly);

    let error = tx.recipe_reads().version_detail(vid(14)).await;

    assert!(matches!(
        error.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}
