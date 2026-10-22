//! Tests of the state that `read_draft` reads at each isolation level (task 6.3, ADR-0042).

use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::VersionState;

use crate::memory::fixtures::{draft_recipe, recipe_with, store, user, vid};
use crate::ports::{Isolation, Transaction};

/// Begin at `isolation`, commit an approval of the draft outside the transaction, and read the draft.
async fn state_after_a_later_commit(isolation: Isolation) -> VersionState {
    let store = store();
    let draft = draft_recipe((1, 1), "R-1", 11);
    store.seed_recipe(&draft, PlcPlatform::TwinCat3, user(9));
    let mut tx = store.begin_memory(isolation).unwrap();
    let approved = recipe_with((1, 1), "R-1", &[(11, VersionState::Approved)]);
    store.seed_recipe(&approved, PlcPlatform::TwinCat3, user(9));

    let read = tx.drafts().read_draft(vid(11)).await.unwrap().unwrap();
    read.state
}

#[tokio::test]
async fn a_repeatable_read_transaction_reads_the_draft_of_its_snapshot() {
    let state = state_after_a_later_commit(Isolation::RepeatableRead).await;
    assert_eq!(state, VersionState::LiveDraft);
}

#[tokio::test]
async fn the_other_levels_read_the_latest_committed_draft() {
    for isolation in [
        Isolation::ReadCommitted,
        Isolation::Serializable,
        Isolation::ReadOnly,
    ] {
        let state = state_after_a_later_commit(isolation).await;
        assert_eq!(state, VersionState::Approved, "{isolation:?}");
    }
}
