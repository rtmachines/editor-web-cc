//! Tests of `RecipeReads::last_change` in memory (design D5, ADR-0043 (m)).
//!
//! The memory store keeps the commit time of each audit entry. The tests commit real commands,
//! so the row times and the entry times come from one clock reading.

use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange, StateAction,
};
use recipe_domain::recipe::VersionState;
use recipe_domain::timestamp::Timestamp;
use serde_json::json;
use time::Duration;

use super::support::{begin, seed_live, seed_user};
use crate::memory::MemoryStore;
use crate::memory::fixtures::{rid, store, text, user, uuid, vid};
use crate::ports::{Clock as _, Isolation, NonEmpty, Transaction, UnitOfWork};

const ONE_DRAFT: &[(u128, VersionState)] = &[(11, VersionState::LiveDraft)];

fn context(actor: u128) -> AuditContext {
    AuditContext {
        actor: Some(user(actor)),
        request_id: RequestId::from(uuid(actor)),
        client_ip: None,
        session_ref: None,
    }
}

async fn commit(store: &MemoryStore, tx: impl Transaction + 'static, entry: AuditEntry) {
    let unit_of_work: &dyn UnitOfWork = store;
    unit_of_work
        .commit(Box::new(tx), NonEmpty::try_from(vec![entry]).unwrap())
        .await
        .unwrap();
}

/// Rename `recipe` as user `actor`. The rename writes one `RecipeRenamed` entry.
async fn rename_as(store: &MemoryStore, (recipe, actor): (u128, u128), name: &str) {
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let after = tx
        .recipes()
        .rename_recipe(rid(recipe), &text(name))
        .await
        .unwrap();
    let entry = AuditEntry::row(
        RowAction::RecipeRenamed,
        AuditTarget::recipe(rid(recipe)),
        RowChange::Update {
            before: json!({}),
            after: after.into_value(),
        },
        context(actor),
    );
    commit(store, tx, entry).await;
}

/// Export version 11 as user `actor`. The export changes no row.
async fn export_as(store: &MemoryStore, actor: u128) {
    let tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let entry = AuditEntry::state(
        StateAction::VersionExported,
        AuditTarget::recipe_version(rid(1), vid(11)),
        json!({}),
        context(actor),
    );
    commit(store, tx, entry).await;
}

/// Return `updated_at` of the recipe row.
fn updated_at(store: &MemoryStore) -> Timestamp {
    store.state().recipes.get(&rid(1)).unwrap().updated_at
}

async fn last_change(store: &MemoryStore, at: Timestamp) -> Option<String> {
    let mut tx = begin(store, Isolation::RepeatableRead);
    let found = tx.recipe_reads().last_change(rid(1), at).await.unwrap();
    found.map(|login| login.as_str().to_owned())
}

fn later(store: &MemoryStore) {
    store.clock().advance(Duration::seconds(10));
}

#[tokio::test]
async fn last_change_returns_the_login_of_the_actor_of_the_newest_change() {
    let store = store();
    seed_user(&store, 2, "jkowalski");
    seed_user(&store, 3, "anowak");
    seed_live(&store, (1, "R-1", "One"), ONE_DRAFT);
    later(&store);
    rename_as(&store, (1, 2), "First").await;
    later(&store);
    rename_as(&store, (1, 3), "Second").await;

    let found = last_change(&store, updated_at(&store)).await;

    assert_eq!(found.as_deref(), Some("anowak"));
}

#[tokio::test]
async fn last_change_is_none_when_no_entry_belongs_to_the_time() {
    let store = store();
    seed_user(&store, 2, "jkowalski");
    seed_live(&store, (1, "R-1", "One"), ONE_DRAFT);
    later(&store);
    rename_as(&store, (1, 2), "First").await;
    let other_time = Timestamp::from(
        updated_at(&store)
            .as_offset_date_time()
            .checked_add(Duration::seconds(1))
            .unwrap(),
    );

    let found = last_change(&store, other_time).await;

    assert_eq!(found, None);
}

#[tokio::test]
async fn last_change_is_none_for_a_recipe_without_audit_entries() {
    let store = store();
    seed_live(&store, (1, "R-1", "One"), ONE_DRAFT);
    let at = updated_at(&store);

    let found = last_change(&store, at).await;

    assert_eq!(found, None);
}

#[tokio::test]
async fn last_change_ignores_a_newer_entry_of_another_recipe_at_the_same_time() {
    let store = store();
    seed_user(&store, 2, "jkowalski");
    seed_user(&store, 3, "anowak");
    seed_live(&store, (1, "R-1", "One"), ONE_DRAFT);
    seed_live(&store, (2, "R-2", "Two"), &[(21, VersionState::LiveDraft)]);
    later(&store);
    rename_as(&store, (1, 3), "First").await;
    rename_as(&store, (2, 2), "Other").await;

    let found = last_change(&store, updated_at(&store)).await;

    assert_eq!(found.as_deref(), Some("anowak"));
}

#[tokio::test]
async fn last_change_matches_an_older_time_with_the_entry_of_that_time() {
    let store = store();
    seed_user(&store, 2, "jkowalski");
    seed_user(&store, 3, "anowak");
    seed_live(&store, (1, "R-1", "One"), ONE_DRAFT);
    later(&store);
    rename_as(&store, (1, 2), "First").await;
    let first_time = updated_at(&store);
    later(&store);
    rename_as(&store, (1, 3), "Second").await;

    let found = last_change(&store, first_time).await;

    assert_eq!(found.as_deref(), Some("jkowalski"));
}

#[tokio::test]
async fn last_change_matches_an_export_entry_by_its_own_time() {
    let store = store();
    seed_user(&store, 2, "jkowalski");
    seed_user(&store, 3, "anowak");
    seed_live(&store, (1, "R-1", "One"), ONE_DRAFT);
    later(&store);
    rename_as(&store, (1, 2), "First").await;
    let rename_time = updated_at(&store);
    later(&store);
    export_as(&store, 3).await;
    let export_time = store.clock().now();

    let at_rename = last_change(&store, rename_time).await;
    let at_export = last_change(&store, export_time).await;

    assert_eq!(at_rename.as_deref(), Some("jkowalski"));
    assert_eq!(at_export.as_deref(), Some("anowak"));
}

#[tokio::test]
async fn last_change_takes_the_entry_written_last_when_two_entries_share_a_time() {
    let store = store();
    seed_user(&store, 2, "jkowalski");
    seed_user(&store, 3, "anowak");
    seed_live(&store, (1, "R-1", "One"), ONE_DRAFT);
    later(&store);
    rename_as(&store, (1, 2), "First").await;
    rename_as(&store, (1, 3), "Second").await;

    let found = last_change(&store, updated_at(&store)).await;

    assert_eq!(found.as_deref(), Some("anowak"));
}
