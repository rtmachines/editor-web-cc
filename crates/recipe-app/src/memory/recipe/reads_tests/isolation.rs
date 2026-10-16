//! Tests of `Isolation::RepeatableRead` in the in-memory transaction (ADR-0042 (e)).

use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::recipe::VersionState;
use serde_json::json;

use super::support::{begin, seed_live};
use crate::error::AppError;
use crate::memory::JournalEntry;
use crate::memory::fixtures::{rid, store, text, uuid};
use crate::ports::{Isolation, NonEmpty, Transaction, UnitOfWork};

#[tokio::test]
async fn begin_through_the_port_records_the_repeatable_read_isolation() {
    let store = store();
    let unit_of_work: &dyn UnitOfWork = &store;

    let tx = unit_of_work.begin(Isolation::RepeatableRead).await.unwrap();
    drop(tx);

    assert_eq!(
        store.journal(),
        vec![JournalEntry::Begin(Isolation::RepeatableRead)]
    );
}

#[tokio::test]
async fn a_repeatable_read_transaction_reads_like_a_read_only_one() {
    let store = store();
    seed_live(&store, (1, "R-1", "One"), &[(11, VersionState::LiveDraft)]);
    let mut snapshot = begin(&store, Isolation::RepeatableRead);
    let mut read_only = begin(&store, Isolation::ReadOnly);

    let from_snapshot = snapshot
        .recipe_reads()
        .version_page(rid(1), 0, 10)
        .await
        .unwrap();
    let from_read_only = read_only
        .recipe_reads()
        .version_page(rid(1), 0, 10)
        .await
        .unwrap();

    assert_eq!(from_snapshot.len(), 1);
    assert_eq!(from_snapshot, from_read_only);
}

#[tokio::test]
async fn a_repeatable_read_transaction_cannot_finish_and_keeps_the_state() {
    let store = store();
    seed_live(&store, (1, "R-1", "One"), &[(11, VersionState::LiveDraft)]);
    let mut tx = begin(&store, Isolation::RepeatableRead);
    let after = tx
        .recipes()
        .rename_recipe(rid(1), &text("Renamed"))
        .await
        .unwrap();
    let entry = AuditEntry::row(
        RowAction::RecipeRenamed,
        AuditTarget::recipe(rid(1)),
        RowChange::Update {
            before: json!({}),
            after: after.into_value(),
        },
        AuditContext {
            actor: None,
            request_id: RequestId::from(uuid(1)),
            client_ip: None,
            session_ref: None,
        },
    );
    let unit_of_work: &dyn UnitOfWork = &store;

    let error = unit_of_work
        .commit(Box::new(tx), NonEmpty::try_from(vec![entry]).unwrap())
        .await
        .unwrap_err();

    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    assert_eq!(
        store.state().recipes.get(&rid(1)).unwrap().name,
        text("One")
    );
    assert!(store.audit_entries().is_empty());
}
