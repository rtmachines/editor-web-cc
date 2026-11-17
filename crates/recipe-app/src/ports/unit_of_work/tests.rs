//! Tests of the commit token against the memory adapter (ADR-0015, ADR-0042 (e)).

use recipe_domain::error_codes::ErrorCode;

use super::{AuditWritten, Isolation, Transaction};
use crate::error::StorageError;
use crate::memory::MemoryStore;

/// `finish` refuses a read-only level on its own, so the guard holds when the audit write is skipped.
async fn assert_finish_refused_at(isolation: Isolation) {
    let store = MemoryStore::default();
    let tx = store.begin_memory(isolation).unwrap();

    let error = Box::new(tx).finish(AuditWritten::new()).await.unwrap_err();

    assert!(
        matches!(&error, StorageError::Other { sqlstate: None, .. }),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(store.audit_entries().is_empty());
}

#[tokio::test]
async fn finish_refuses_a_read_only_transaction() {
    assert_finish_refused_at(Isolation::ReadOnly).await;
}

#[tokio::test]
async fn finish_refuses_a_repeatable_read_transaction() {
    assert_finish_refused_at(Isolation::RepeatableRead).await;
}
