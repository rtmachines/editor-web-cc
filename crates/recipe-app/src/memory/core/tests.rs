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
async fn fail_begin_after_commit_fails_the_first_begin_after_a_commit() {
    let store = MemoryStore::default();
    store.fail_begin_after_commit(Fault::Unavailable);
    let tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    commit(&store, Box::new(tx), &[1]).await.unwrap();
    let unit_of_work: &dyn UnitOfWork = &store;
    let error = unit_of_work.begin(Isolation::ReadOnly).await.err().unwrap();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert!(store.begin_memory(Isolation::ReadOnly).is_ok());
}

#[tokio::test]
async fn fail_begin_after_commit_lets_the_begin_before_the_commit_pass() {
    let store = MemoryStore::default();
    store.fail_begin_after_commit(Fault::Unavailable);
    assert!(store.begin_memory(Isolation::ReadOnly).is_ok());
    assert!(store.begin_memory(Isolation::ReadCommitted).is_ok());
}

#[tokio::test]
async fn clear_failures_removes_the_fault_of_fail_begin_after_commit() {
    let store = MemoryStore::default();
    store.fail_begin_after_commit(Fault::Unavailable);
    store.clear_failures();
    let tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    commit(&store, Box::new(tx), &[1]).await.unwrap();
    assert!(store.begin_memory(Isolation::ReadOnly).is_ok());
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
async fn a_commit_in_a_read_only_transaction_stops_at_the_audit_write_and_keeps_the_state() {
    let store = MemoryStore::default();
    let tx = store.begin_memory(Isolation::ReadOnly).unwrap();
    let error = commit(&store, Box::new(tx), &[1]).await.unwrap_err();
    // The audit write comes before `finish` and gives 25006 in this level (design D7).
    assert!(
        matches!(error, AppError::AuditWriteFailed { .. }),
        "{error:?}"
    );
    assert!(markers(&store).is_empty());
    assert!(store.audit_entries().is_empty());
}

/// A write in a read-only level gives SQLSTATE `25006` and `E-DB-008`, as PostgreSQL does.
async fn assert_write_refused_at(isolation: Isolation) {
    let store = MemoryStore::default();
    let mut tx = store.begin_memory(isolation).unwrap();

    let error = tx.state_for_write().map(|_| ()).unwrap_err();
    assert!(
        matches!(&error, StorageError::Other { sqlstate: Some(state), .. } if state == "25006"),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_008);

    let error = tx.append_audit(&entries(&[1])).await.unwrap_err();
    assert!(
        matches!(&error, StorageError::Other { sqlstate: Some(state), .. } if state == "25006"),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_008);
}

#[tokio::test]
async fn a_write_in_a_read_only_transaction_gives_25006() {
    assert_write_refused_at(Isolation::ReadOnly).await;
}

#[tokio::test]
async fn a_write_in_a_repeatable_read_transaction_gives_25006() {
    assert_write_refused_at(Isolation::RepeatableRead).await;
}

#[test]
fn the_transaction_and_the_unit_of_work_are_send() {
    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send::<Box<dyn Transaction>>();
    assert_send_sync::<Arc<dyn UnitOfWork>>();
    assert_send_sync::<MemoryStore>();
}

/// A store with the machine M1 and the recipe R1 whose version V11 is a live draft.
fn store_with_draft() -> MemoryStore {
    let store = crate::memory::fixtures::store();
    store.seed_recipe(
        &crate::memory::fixtures::draft_recipe((1, 1), "R-1", 11),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        crate::memory::fixtures::user(9),
    );
    store
}

#[tokio::test]
async fn the_hook_on_lock_recipe_gives_none_once_and_then_the_row() {
    use crate::memory::MissingRow;
    use crate::memory::fixtures::rid;

    let store = store_with_draft();
    store.miss_next(MissingRow::Recipe);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    assert!(tx.recipes().lock_recipe(rid(1)).await.unwrap().is_none());
    assert!(store.markers().is_empty(), "a missed lock leaves no line");
    let locked = tx.recipes().lock_recipe(rid(1)).await.unwrap();
    assert_eq!(locked.unwrap().state.id(), rid(1));
}

#[tokio::test]
async fn the_hook_on_lock_machine_shared_gives_none_once_and_then_the_row() {
    use crate::memory::MissingRow;
    use crate::memory::fixtures::mid;

    let store = store_with_draft();
    store.miss_next(MissingRow::MachineShared);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    assert!(
        tx.plant()
            .lock_machine_shared(mid(1))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        tx.plant()
            .lock_machine_shared(mid(1))
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn the_hook_on_lock_draft_gives_none_once_and_then_the_row() {
    use crate::memory::MissingRow;
    use crate::memory::fixtures::vid;

    let store = store_with_draft();
    store.miss_next(MissingRow::Draft);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    assert!(tx.drafts().lock_draft(vid(11)).await.unwrap().is_none());
    assert!(tx.drafts().lock_draft(vid(11)).await.unwrap().is_some());
}

#[tokio::test]
async fn the_hook_on_live_units_gives_none_once_and_then_the_units() {
    use crate::memory::MissingRow;
    use crate::memory::fixtures::mid;

    let store = store_with_draft();
    store.miss_next(MissingRow::LiveUnits);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    assert!(tx.units().live_units(mid(1)).await.unwrap().is_none());
    assert!(tx.units().live_units(mid(1)).await.unwrap().is_some());
}

#[tokio::test]
async fn a_hook_of_one_lookup_does_not_touch_another_lookup() {
    use crate::memory::MissingRow;
    use crate::memory::fixtures::{mid, rid};

    let store = store_with_draft();
    store.miss_next(MissingRow::Recipe);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    assert!(
        tx.plant()
            .lock_machine_shared(mid(1))
            .await
            .unwrap()
            .is_some()
    );
    assert!(tx.recipes().lock_recipe(rid(1)).await.unwrap().is_none());
}

/// Count the rows of `rows` with the identifier `id`.
fn count_rows(
    rows: &[recipe_domain::plant::PlcType],
    id: recipe_domain::plant::PlcTypeId,
) -> usize {
    rows.iter().filter(|row| row.id() == id).count()
}

#[tokio::test]
async fn the_hook_on_type_rows_adds_a_duplicate_to_the_first_call_only() {
    use crate::memory::fixtures::{mid, store};
    use crate::memory::seed_type_id;
    use recipe_domain::plant::Primitive;

    let store = store();
    let int = seed_type_id(Primitive::Int, None).unwrap();
    store.duplicate_type_row_on_call(1, int);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let first = tx.types().type_rows(mid(1)).await.unwrap();
    assert_eq!(count_rows(&first, int), 2);
    let second = tx.types().type_rows(mid(1)).await.unwrap();
    assert_eq!(count_rows(&second, int), 1);
}

#[tokio::test]
async fn the_hook_on_type_rows_adds_a_duplicate_to_the_second_call_only() {
    use crate::memory::fixtures::{mid, store};
    use crate::memory::seed_type_id;
    use recipe_domain::plant::Primitive;

    let store = store();
    let int = seed_type_id(Primitive::Int, None).unwrap();
    store.duplicate_type_row_on_call(2, int);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let first = tx.types().type_rows(mid(1)).await.unwrap();
    let second = tx.types().type_rows(mid(1)).await.unwrap();
    let third = tx.types().type_rows(mid(1)).await.unwrap();
    assert_eq!(count_rows(&first, int), 1);
    assert_eq!(count_rows(&second, int), 2);
    assert_eq!(count_rows(&third, int), 1);
}

#[tokio::test]
async fn the_hook_on_type_rows_with_call_zero_never_fires() {
    use crate::memory::fixtures::{mid, store};
    use crate::memory::seed_type_id;
    use recipe_domain::plant::Primitive;

    let store = store();
    let int = seed_type_id(Primitive::Int, None).unwrap();
    store.duplicate_type_row_on_call(0, int);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let first = tx.types().type_rows(mid(1)).await.unwrap();
    let second = tx.types().type_rows(mid(1)).await.unwrap();
    assert_eq!(count_rows(&first, int), 1);
    assert_eq!(count_rows(&second, int), 1);
}

#[tokio::test]
async fn clear_failures_clears_a_pending_type_row_duplicate() {
    use crate::memory::fixtures::{mid, store};
    use crate::memory::seed_type_id;
    use recipe_domain::plant::Primitive;

    let store = store();
    let int = seed_type_id(Primitive::Int, None).unwrap();
    store.duplicate_type_row_on_call(1, int);
    store.clear_failures();
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    assert_eq!(count_rows(&rows, int), 1);
}

#[tokio::test]
async fn clear_failures_clears_a_pending_miss() {
    use crate::memory::MissingRow;
    use crate::memory::fixtures::rid;

    let store = store_with_draft();
    store.miss_next(MissingRow::Recipe);
    store.clear_failures();
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    assert!(tx.recipes().lock_recipe(rid(1)).await.unwrap().is_some());
}
