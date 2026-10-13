//! Port of the security event log.

use async_trait::async_trait;
use recipe_domain::audit::SecurityEvent;

use crate::error::StorageError;

/// Sink of security events.
///
/// The port solves one problem: a security event must survive the rollback of the
/// request that caused it. `record` therefore writes in its own short transaction
/// (`domain-model.md` section 5, ADR-0015).
#[async_trait]
pub trait SecurityEventSink: Send + Sync {
    /// Write one event in its own transaction.
    async fn record(&self, event: SecurityEvent) -> Result<(), StorageError>;
}
