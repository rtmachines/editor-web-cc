//! Job `audit_chain_check`: verify the hash chain of the audit log (design D6).
//!
//! Purpose: find a changed or removed audit row.
//!
//! Schedule: daily at `audit.check_time`, and once after `audit.catchup_delay`.
//! The nightly run and the catch-up never overlap. The catch-up skips after a newer check.
//!
//! A run reads the head and applies the rules of step 1 (ADR-0086 (c)). Then it reads the chain
//! in batches from the first row. Each batch is one short transaction through the port, so the
//! check never delays VACUUM. After the last batch the run compares its end with the head
//! (ADR-0086 (d)). A break writes the security event, records the state keys, and returns
//! `E-AUD-002`. The supervisor logs that error once, so the job writes no log line.

use std::sync::Arc;

use recipe_domain::audit::{RequestId, SecurityEvent};
use recipe_domain::timestamp::Timestamp;

use self::rules::Verdict;
use super::RunKind;
use crate::error::{AppError, AuditError, StorageError};
use crate::ports::{
    AuditChainReads, BatchOutcome, ChainHead, ChainPosition, ChainSeq, Clock, MaintenanceStore,
    OperationsReads, SecurityEventSink, StateValue, TimeStateKey,
};

mod rules;

/// Default of `audit.check_batch` in rows (`reliability.md` section 7.11).
const DEFAULT_BATCH_ROWS: u32 = 10_000;
/// Default of `audit.check_batch_bytes`: 8 MiB (`reliability.md` section 7.11).
const DEFAULT_BATCH_BYTES: u64 = 8 * 1024 * 1024;

/// The numeric settings that the job `audit_chain_check` reads.
///
/// The type solves one problem: a job must not read a configuration file. `recipe-server` builds
/// this value and passes it down. `Default` equals the table of `reliability.md` section 7.11, and
/// a test compares both. The statement timeout belongs to the adapter of the port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuditChainCheckSettings {
    /// The most rows of one batch (`audit.check_batch`).
    pub batch_rows: u32,
    /// The most bytes of `before_state` and `after_state` in one batch
    /// (`audit.check_batch_bytes`).
    pub batch_bytes: u64,
}

impl Default for AuditChainCheckSettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            batch_rows: DEFAULT_BATCH_ROWS,
            batch_bytes: DEFAULT_BATCH_BYTES,
        }
    }
}

/// The ports that the job `audit_chain_check` uses.
///
/// The type solves one problem: the job needs five ports, and a long argument list hides which
/// one is which. The server wires the adapters of `recipe-db`. A test wires the memory adapters.
pub struct AuditChainCheckPorts {
    /// The reads of the chain: the head and the batches.
    pub chain: Arc<dyn AuditChainReads>,
    /// The writes of the two state keys.
    pub store: Arc<dyn MaintenanceStore>,
    /// The read of `audit.last_check_at` for the catch-up.
    pub operations: Arc<dyn OperationsReads>,
    /// The sink of the security event `audit_chain_broken`.
    pub sink: Arc<dyn SecurityEventSink>,
    /// The source of the time of `audit.last_check_at`.
    pub clock: Arc<dyn Clock>,
}

/// The supervised job that verifies the hash chain of `audit_log`.
///
/// The type solves one problem: a changed or removed audit row must raise an alert within one
/// night, and the check must not hold a long transaction on the largest table. The job keeps the
/// value of `audit.last_check_at` that the start read, so the catch-up knows what "newer" means.
pub struct AuditChainCheck {
    /// The ports of the job.
    ports: AuditChainCheckPorts,
    /// The batch limits.
    settings: AuditChainCheckSettings,
    /// The value of `audit.last_check_at` that the start read (ADR-0060 (v)).
    checked_at_start: Option<Timestamp>,
}

impl AuditChainCheck {
    /// Build the job.
    ///
    /// `checked_at_start` is the value of `audit.last_check_at` that the start read, or `None`
    /// when no check completed. A `CatchUp` run skips when a newer value exists (ADR-0088 (b)).
    #[must_use]
    pub fn new(
        ports: AuditChainCheckPorts,
        settings: AuditChainCheckSettings,
        checked_at_start: Option<Timestamp>,
    ) -> Self {
        Self {
            ports,
            settings,
            checked_at_start,
        }
    }

    /// Check the whole chain once.
    ///
    /// A `CatchUp` run reads `audit.last_check_at` first and skips when a check completed after
    /// the start. A `Scheduled` run never skips. The check starts at the first row and stops at
    /// the first break (ADR-0073 (a)). A check without a break records the head as the anchor.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Audit(AuditError::ChainBroken)` (`E-AUD-002`) after the security event
    /// and the two state keys. It returns `AppError::Storage` when a read or a write fails. A fault
    /// of the event write records no state key.
    pub async fn run_once(&self, request_id: RequestId, kind: RunKind) -> Result<(), AppError> {
        if kind == RunKind::CatchUp && self.newer_check_exists().await? {
            return Ok(());
        }
        let head = self.ports.chain.head().await?;
        let verdict = match rules::step_one(&head)? {
            Some(verdict) => Some(verdict),
            None => self.check_batches(&head).await?,
        };
        match verdict {
            None => Ok(self.record_state(head.position.seq).await?),
            Some(verdict) => self.report_break(request_id, verdict).await,
        }
    }

    /// Return true when a check completed after the value that the start read.
    async fn newer_check_exists(&self) -> Result<bool, StorageError> {
        let current = self
            .ports
            .operations
            .state(TimeStateKey::AuditLastCheckAt)
            .await?;
        Ok(match (current, self.checked_at_start) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(current), Some(start)) => current > start,
        })
    }

    /// Read the chain in batches up to the head, then compare the end with the head.
    async fn check_batches(&self, head: &ChainHead) -> Result<Option<Verdict>, StorageError> {
        let mut end = ChainPosition::START;
        while end.seq < head.position.seq {
            let outcome = self
                .ports
                .chain
                .check_batch(
                    &end,
                    head.position.seq,
                    self.settings.batch_rows,
                    self.settings.batch_bytes,
                )
                .await?;
            match outcome {
                BatchOutcome::Broken {
                    first_break,
                    last_clean,
                } => {
                    return Ok(Some(rules::batch_break(
                        first_break,
                        last_clean,
                        head.anchor,
                    )));
                }
                BatchOutcome::Clean { last: None, .. } => break,
                BatchOutcome::Clean {
                    last: Some(next), ..
                } => end = advance(&end, next, head.position.seq)?,
            }
        }
        rules::end_check(&end, &head.position, head.anchor)
    }

    /// Write the event and the state keys of a break, then return the error.
    async fn report_break(&self, request_id: RequestId, verdict: Verdict) -> Result<(), AppError> {
        self.ports
            .sink
            .record(SecurityEvent::audit_chain_broken(request_id))
            .await?;
        self.record_state(verdict.anchor).await?;
        Err(AuditError::ChainBroken {
            chain_seq: verdict.chain_seq.get(),
        }
        .into())
    }

    /// Record the anchor, then the time of the completed check (ADR-0086 (e)).
    ///
    /// The order matters. The job never announces a check before its anchor persists.
    async fn record_state(&self, anchor: ChainSeq) -> Result<(), StorageError> {
        self.ports
            .store
            .record_state(&StateValue::AuditLastCheckSeq(anchor))
            .await?;
        self.ports
            .store
            .record_state(&StateValue::AuditLastCheckAt(self.ports.clock.now()))
            .await
    }
}

/// Return the end after a clean batch, or a fault when the batch broke its contract.
///
/// A batch must move the end forward and must not pass the head. A batch that does not move the
/// end forward would loop for ever. A batch that passes the head would check rows that the head
/// does not cover.
fn advance(
    end: &ChainPosition,
    next: ChainPosition,
    head: ChainSeq,
) -> Result<ChainPosition, StorageError> {
    if next.seq <= end.seq || next.seq > head {
        return Err(StorageError::invariant(
            "a clean batch ended outside the range after its start and up to the head",
        ));
    }
    Ok(next)
}

#[cfg(test)]
mod tests;
