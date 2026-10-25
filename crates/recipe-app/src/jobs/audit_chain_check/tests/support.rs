//! Helpers that the tests of the job `audit_chain_check` share.

use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::{RequestId, SecurityEvent};
use recipe_domain::setting::{SettingKey, SettingValue};
use recipe_domain::timestamp::Timestamp;
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;

use crate::error::{AppError, AuditError, StorageError};
use crate::jobs::RunKind;
use crate::jobs::audit_chain_check::{
    AuditChainCheck, AuditChainCheckPorts, AuditChainCheckSettings,
};
use crate::memory::{
    MemoryAuditChain, MemoryClock, MemoryMaintenance, MemoryOperations, MemorySink, MemoryStore,
};
use crate::ports::{AuditChainReads, BatchOutcome, ChainHead, ChainPosition, ChainSeq, StateValue};

/// The time of the clock at the start of a test, in Unix seconds.
pub(super) const BASE_SECONDS: i64 = 1_000_000;
/// The bytes of `before_state` and `after_state` of a row that a test appends.
pub(super) const ROW_BYTES: u64 = 10;
/// The key of the anchor row.
pub(super) const SEQ_KEY: &str = "audit.last_check_seq";
/// The key of the time of the last completed check.
pub(super) const AT_KEY: &str = "audit.last_check_at";

/// A job on the in-memory adapters, with the handles that a test needs.
pub(super) struct Rig {
    /// The store that holds the rows and the clock.
    pub store: MemoryStore,
    /// The chain that the job reads. A scripted rig does not use it.
    pub chain: MemoryAuditChain,
    /// The port that records the state keys. A test reads its call counters.
    pub maintenance: MemoryMaintenance,
    /// The port that reads `audit.last_check_at`. A test reads its call counters.
    pub operations: MemoryOperations,
    /// The sink that holds the security events.
    pub sink: MemorySink,
    /// The job under test.
    pub job: AuditChainCheck,
}

/// The request id of the run of every test.
pub(super) fn request_id() -> RequestId {
    RequestId::from(Uuid::from_u128(7))
}

/// The instant `seconds` after the start of the test. A negative value is before it.
pub(super) fn at(seconds: i64) -> Timestamp {
    let base = OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap();
    Timestamp::from(base.checked_add(TimeDuration::seconds(seconds)).unwrap())
}

/// Return the sequence number `n`.
pub(super) fn seq(n: i64) -> ChainSeq {
    ChainSeq::from(n)
}

/// Build a rig with `rows` correct rows, the default settings, and no check at the start.
pub(super) fn rig(rows: u32) -> Rig {
    rig_with(rows, AuditChainCheckSettings::default(), None)
}

/// Build a rig with `rows` correct rows, `settings`, and the time of the last check at the start.
pub(super) fn rig_with(
    rows: u32,
    settings: AuditChainCheckSettings,
    checked_at_start: Option<Timestamp>,
) -> Rig {
    let store = new_store();
    let chain = MemoryAuditChain::new(store.clone());
    for _ in 0..rows {
        chain.append_row(ROW_BYTES);
    }
    assemble(
        store,
        chain.clone(),
        Arc::new(chain),
        settings,
        checked_at_start,
    )
}

/// Build a rig whose job reads `reads` and not the memory chain.
pub(super) fn rig_over(reads: Arc<dyn AuditChainReads>) -> Rig {
    let store = new_store();
    let chain = MemoryAuditChain::new(store.clone());
    assemble(
        store,
        chain,
        reads,
        AuditChainCheckSettings::default(),
        None,
    )
}

/// Build a rig with `rows` correct rows whose clock moves `step_seconds` in each batch read.
///
/// The clock starts at `BASE_SECONDS`. A test reads the end time of a run from the later instant.
pub(super) fn rig_with_moving_clock(rows: u32, step_seconds: i64) -> Rig {
    let store = new_store();
    let chain = MemoryAuditChain::new(store.clone());
    for _ in 0..rows {
        chain.append_row(ROW_BYTES);
    }
    let reads = ClockMovingChain {
        inner: chain.clone(),
        clock: store.clock(),
        step: TimeDuration::seconds(step_seconds),
    };
    assemble(
        store,
        chain,
        Arc::new(reads),
        AuditChainCheckSettings::default(),
        None,
    )
}

fn new_store() -> MemoryStore {
    MemoryStore::with_clock(MemoryClock::new(
        OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap(),
    ))
}

fn assemble(
    store: MemoryStore,
    chain: MemoryAuditChain,
    reads: Arc<dyn AuditChainReads>,
    settings: AuditChainCheckSettings,
    checked_at_start: Option<Timestamp>,
) -> Rig {
    let maintenance = MemoryMaintenance::new(store.clone());
    let operations = MemoryOperations::new(store.clone());
    let sink = store.sink();
    let ports = AuditChainCheckPorts {
        chain: reads,
        store: Arc::new(maintenance.clone()),
        operations: Arc::new(operations.clone()),
        sink: Arc::new(sink.clone()),
        clock: Arc::new(store.clock()),
    };
    let job = AuditChainCheck::new(ports, settings, checked_at_start);
    Rig {
        store,
        chain,
        maintenance,
        operations,
        sink,
        job,
    }
}

/// Run the job once as a scheduled run.
pub(super) async fn run(rig: &Rig) -> Result<(), AppError> {
    run_kind(rig, RunKind::Scheduled).await
}

/// Run the job once with the run kind `kind`.
pub(super) async fn run_kind(rig: &Rig, kind: RunKind) -> Result<(), AppError> {
    rig.job.run_once(request_id(), kind).await
}

/// Return the `chain_seq` of the break that a run reported, or `None` for another error.
///
/// The function panics when the run succeeded, so a missing break fails the test.
pub(super) fn chain_seq_of(result: Result<(), AppError>) -> Option<i64> {
    match result.unwrap_err() {
        AppError::Audit(AuditError::ChainBroken { chain_seq }) => Some(chain_seq),
        _other => None,
    }
}

/// Return the stored text of the `app_setting` row `key`, or `None` when the row is missing.
pub(super) fn stored(rig: &Rig, key: &str) -> Option<String> {
    let key = SettingKey::parse(key).unwrap();
    rig.store
        .state()
        .settings
        .get(&key)
        .map(|row| row.value.as_str().to_owned())
}

/// Write the anchor `audit.last_check_seq` without a call of the port `MaintenanceStore`.
pub(super) fn set_anchor(rig: &Rig, n: i64) {
    let text = StateValue::AuditLastCheckSeq(seq(n)).to_text().unwrap();
    write_state(rig, SEQ_KEY, &text);
}

/// Write the time `audit.last_check_at` without a call of the port `MaintenanceStore`.
pub(super) fn set_checked_at(rig: &Rig, at: Timestamp) {
    let text = StateValue::AuditLastCheckAt(at).to_text().unwrap();
    write_state(rig, AT_KEY, &text);
}

fn write_state(rig: &Rig, key: &str, text: &str) {
    let key = SettingKey::parse(key).unwrap();
    let value = SettingValue::parse(text).unwrap();
    rig.store
        .update_state(|state, now| state.upsert_setting(key, value, now))
        .unwrap();
}

/// Return the text that a time gets in `app_setting`.
pub(super) fn time_text(at: Timestamp) -> String {
    StateValue::AuditLastCheckAt(at).to_text().unwrap()
}

/// Return the security events that the sink holds.
pub(super) fn events(rig: &Rig) -> Vec<SecurityEvent> {
    rig.sink.events()
}

/// A chain reader that moves the clock forward in each batch read.
///
/// It solves one problem: `MemoryClock` stands still during a run, so a test cannot tell the
/// start of a run from its end. This reader makes the end later than the start.
pub(super) struct ClockMovingChain {
    /// The memory chain that answers each read.
    inner: MemoryAuditChain,
    /// The clock that the job also reads.
    clock: MemoryClock,
    /// The time that each batch read adds to the clock.
    step: TimeDuration,
}

#[async_trait]
impl AuditChainReads for ClockMovingChain {
    async fn head(&self) -> Result<ChainHead, StorageError> {
        self.inner.head().await
    }

    async fn check_batch(
        &self,
        after: &ChainPosition,
        head: ChainSeq,
        batch: u32,
        bytes: u64,
    ) -> Result<BatchOutcome, StorageError> {
        self.clock.advance(self.step);
        self.inner.check_batch(after, head, batch, bytes).await
    }
}

/// A chain reader that returns a scripted head and scripted batches.
///
/// It solves one problem: the end check of ADR-0086 (d) needs a clean batch that ends below the
/// head, and the memory chain cannot give one without a concurrent change between two calls.
pub(super) struct ScriptedChain {
    /// The head that step 1 returns.
    head: ChainHead,
    /// The outcomes of the batches, in the order of the calls.
    batches: Mutex<VecDeque<BatchOutcome>>,
}

impl ScriptedChain {
    /// Build a reader that returns `head` and then `batches`.
    pub(super) fn new(head: ChainHead, batches: Vec<BatchOutcome>) -> Arc<Self> {
        Arc::new(Self {
            head,
            batches: Mutex::new(batches.into()),
        })
    }
}

#[async_trait]
impl AuditChainReads for ScriptedChain {
    async fn head(&self) -> Result<ChainHead, StorageError> {
        Ok(self.head)
    }

    async fn check_batch(
        &self,
        _after: &ChainPosition,
        _head: ChainSeq,
        _batch: u32,
        _bytes: u64,
    ) -> Result<BatchOutcome, StorageError> {
        Ok(self
            .batches
            .lock()
            .pop_front()
            .unwrap_or(BatchOutcome::Clean {
                rows: 0,
                last: None,
            }))
    }
}
