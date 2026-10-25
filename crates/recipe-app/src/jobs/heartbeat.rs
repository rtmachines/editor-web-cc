//! Job `heartbeat`: write the state key `heartbeat.last_at` (design D6).
//!
//! Purpose: write one small row every interval, so the WAL stream never stays idle for long
//! (`reliability.md` section 7.7).
//!
//! Schedule: every `heartbeat.interval`. The interval belongs to the loop of the server, because
//! the job reads no setting. Each run records the state key `heartbeat.last_at` with the time of
//! the clock. The job writes no audit entry (`domain-model.md` section 5) and no log line. A
//! fault of the store is a storage error, so the run returns it and the supervisor logs it once
//! at `ERROR` (ADR-0060 (q), ADR-0089 (d)).

use std::sync::Arc;

use recipe_domain::audit::RequestId;

use super::RunKind;
use crate::error::AppError;
use crate::ports::{Clock, MaintenanceStore, StateValue};

/// The supervised job that writes one row every interval.
///
/// The type solves one problem: a quiet night writes no WAL, so the stream of `walarchive` can
/// stay idle for hours. One small write per interval keeps the stream active and shows that the
/// database accepts writes (`reliability.md` section 7.7).
pub struct Heartbeat {
    /// The port that writes the state key.
    store: Arc<dyn MaintenanceStore>,
    /// The source of the time that the row holds.
    clock: Arc<dyn Clock>,
}

impl Heartbeat {
    /// Build the job.
    #[must_use]
    pub fn new(store: Arc<dyn MaintenanceStore>, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// Record the time of the clock in the state key `heartbeat.last_at`.
    ///
    /// The run kind has no effect, because the job has one trigger (ADR-0088 (b)). The run writes
    /// no row with the request id.
    ///
    /// # Errors
    ///
    /// Returns the `StorageError` of the write as `AppError::Storage`. The row keeps its old value.
    pub async fn run_once(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        let now = self.clock.now();
        self.store
            .record_state(&StateValue::HeartbeatLastAt(now))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
