use std::sync::Arc;

use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::MachineId;
use serde_json::json;
use uuid::Uuid;

use super::{JournalEntry, Lock, LockMode, MemoryStore};
use crate::error::{AppError, StorageError};
use crate::memory::Fault;
use crate::ports::{Isolation, NonEmpty, Transaction, UnitOfWork};

fn entry(n: u128) -> AuditEntry {
    AuditEntry::row(
        RowAction::MachineCreated,
        AuditTarget::machine(MachineId::from(Uuid::from_u128(n))),
        RowChange::Create {
            after: json!({ "n": n.to_string() }),
        },
        AuditContext {
            actor: None,
            request_id: RequestId::from(Uuid::from_u128(n)),
            client_ip: None,
            session_ref: None,
        },
    )
}

fn entries(numbers: &[u128]) -> NonEmpty<AuditEntry> {
    NonEmpty::try_from(numbers.iter().copied().map(entry).collect::<Vec<_>>()).unwrap()
}

fn lock(table: &'static str, mode: LockMode, key: &str) -> JournalEntry {
    JournalEntry::Lock(Lock {
        table,
        mode,
        key: key.to_owned(),
    })
}

fn markers(store: &MemoryStore) -> Vec<String> {
    store.markers()
}

async fn commit(
    store: &MemoryStore,
    tx: Box<dyn Transaction>,
    numbers: &[u128],
) -> Result<(), AppError> {
    let unit_of_work: &dyn UnitOfWork = store;
    unit_of_work.commit(tx, entries(numbers)).await
}

#[tokio::test]
async fn a_commit_keeps_the_writes_and_the_entries() {
    let store = MemoryStore::default();
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    tx.mark("a");
    commit(&store, Box::new(tx), &[1, 2]).await.unwrap();
    assert_eq!(markers(&store), vec!["a".to_owned()]);
    assert_eq!(store.audit_entries(), vec![entry(1), entry(2)]);
}

#[tokio::test]
async fn two_commits_keep_the_entries_in_order() {
    let store = MemoryStore::default();
    for number in [1, 2] {
        let tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
        commit(&store, Box::new(tx), &[number]).await.unwrap();
    }
    assert_eq!(store.audit_entries(), vec![entry(1), entry(2)]);
}

#[tokio::test]
async fn a_dropped_transaction_keeps_nothing() {
    let store = MemoryStore::default();
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    tx.mark("a");
    drop(tx);
    assert!(markers(&store).is_empty());
    assert!(store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_write_is_invisible_before_the_commit() {
    let store = MemoryStore::default();
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    tx.mark("a");
    assert!(markers(&store).is_empty());
}

#[tokio::test]
async fn a_failed_audit_write_gives_e_aud_001_and_keeps_nothing() {
    let store = MemoryStore::default();
    store.fail_append_audit(Fault::Unavailable);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    tx.mark("a");
    let error = commit(&store, Box::new(tx), &[1]).await.unwrap_err();
    assert!(
        matches!(
            error,
            AppError::AuditWriteFailed {
                cause: StorageError::Unavailable
            }
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::AUD_001);
    assert!(markers(&store).is_empty());
    assert!(store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_failed_finish_gives_its_storage_code_and_keeps_nothing() {
    let store = MemoryStore::default();
    store.fail_finish(Fault::Deadlock);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    tx.mark("a");
    let error = commit(&store, Box::new(tx), &[1]).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert!(markers(&store).is_empty());
    assert!(store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_failed_begin_returns_its_storage_error_and_opens_nothing() {
    let store = MemoryStore::default();
    store.fail_begin(Fault::PoolTimeout);
    let unit_of_work: &dyn UnitOfWork = &store;
    let error = unit_of_work
        .begin(Isolation::ReadCommitted)
        .await
        .err()
        .unwrap();
    assert_eq!(error.code(), ErrorCode::DB_004);
    assert!(store.journal().is_empty());
}

#[tokio::test]
async fn clear_failures_lets_the_next_commit_pass() {
    let store = MemoryStore::default();
    store.fail_finish(Fault::Deadlock);
    store.clear_failures();
    let tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    commit(&store, Box::new(tx), &[1]).await.unwrap();
}

#[tokio::test]
async fn the_journal_lists_begin_and_the_locks_in_call_order() {
    let store = MemoryStore::default();
    let tx = store.begin_memory(Isolation::Serializable).unwrap();
    tx.lock("machine", LockMode::ForShare, "m1");
    tx.lock("recipe", LockMode::ForUpdate, "r1");
    tx.lock("unit", LockMode::Advisory, "2:struct");
    assert_eq!(
        store.journal(),
        vec![
            JournalEntry::Begin(Isolation::Serializable),
            lock("machine", LockMode::ForShare, "m1"),
            lock("recipe", LockMode::ForUpdate, "r1"),
            lock("unit", LockMode::Advisory, "2:struct"),
        ]
    );
}

#[tokio::test]
async fn the_journal_keeps_the_locks_after_a_rollback() {
    let store = MemoryStore::default();
    let tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    tx.lock("machine", LockMode::ForShare, "m1");
    drop(tx);
    assert_eq!(
        store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock("machine", LockMode::ForShare, "m1"),
        ]
    );
}

#[tokio::test]
async fn begin_through_the_port_records_its_isolation() {
    let store = MemoryStore::default();
    let unit_of_work: &dyn UnitOfWork = &store;
    let tx = unit_of_work.begin(Isolation::ReadOnly).await.unwrap();
    drop(tx);
    assert_eq!(
        store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
}

#[tokio::test]
async fn a_read_only_transaction_cannot_finish_and_keeps_the_state() {
    let store = MemoryStore::default();
    let mut tx = store.begin_memory(Isolation::ReadOnly).unwrap();
    tx.mark("a");
    let error = commit(&store, Box::new(tx), &[1]).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    assert!(markers(&store).is_empty());
    assert!(store.audit_entries().is_empty());
}

#[test]
fn the_transaction_and_the_unit_of_work_are_send() {
    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send::<Box<dyn Transaction>>();
    assert_send_sync::<Arc<dyn UnitOfWork>>();
    assert_send_sync::<MemoryStore>();
}
