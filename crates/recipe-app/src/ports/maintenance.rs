//! Port of the maintenance writes of the supervised jobs (ADR-0060, `data-model.md` section 6).

use std::time::Duration;

use async_trait::async_trait;

use super::StateValue;
use crate::error::StorageError;

/// Store of the writes that the jobs `cleanup`, `audit_chain_check`, and `heartbeat` make.
///
/// The port solves one problem: these writes belong to no command and have no audit entry
/// (`domain-model.md` section 5). They must still go through a port. A test runs the jobs
/// against an in-memory adapter. Each method runs one statement in its own short transaction,
/// so a failed batch keeps the batches before it.
#[async_trait]
pub trait MaintenanceStore: Send + Sync {
    /// Delete the sessions that are expired or older than `absolute_timeout`.
    ///
    /// Returns the number of deleted rows.
    async fn delete_expired_sessions(
        &self,
        absolute_timeout: Duration,
    ) -> Result<u64, StorageError>;

    /// Delete at most `batch` login attempts that are older than `retention`.
    ///
    /// Returns the number of deleted rows. The job calls it again while the result equals `batch`.
    async fn purge_login_attempts(
        &self,
        retention: Duration,
        batch: u32,
    ) -> Result<u64, StorageError>;

    /// Delete one batch of old security events through `security_event_purge()`.
    ///
    /// The retention and the batch size are constants of the function
    /// (`data-model.md` section 4.12).
    /// Returns the number of deleted rows. The job calls it again while the batch is full.
    async fn purge_security_events(&self) -> Result<u64, StorageError>;

    /// Write one state key of `app_setting` with no audit entry and no `updated_by`.
    ///
    /// The three keys are the variants of [`StateValue`] (`data-model.md` section 3.17).
    /// The call inserts the row when it does not exist.
    async fn record_state(&self, value: &StateValue) -> Result<(), StorageError>;
}
