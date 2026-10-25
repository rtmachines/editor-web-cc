//! Tests of the job `heartbeat` (task 3.7, design D6 and D14 row T14, ADR-0060 (q)).
//!
//! They run the job against the in-memory adapters. The job clock differs from the clock of the
//! store, so a test sees which clock the job reads.

use std::sync::{Arc, LazyLock};

use parking_lot::Mutex;
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Event, Metadata, Subscriber};
use uuid::Uuid;

use super::Heartbeat;
use crate::error::AppError;
use crate::jobs::RunKind;
use crate::memory::{
    Fault, MaintenanceMethod, MemoryClock, MemoryMaintenance, MemoryOperations, MemoryStore,
};
use crate::ports::{OperationsReads, TimeStateKey};

/// The time of the clock of the store, in Unix seconds.
const STORE_SECONDS: i64 = 1_000_000;
/// The time of the clock of the job at the start of a test, in Unix seconds.
const JOB_SECONDS: i64 = 2_000_000;
/// The time of the clock of the job after a test moves it, in Unix seconds.
const LATER_SECONDS: i64 = 2_000_300;

/// A job on the in-memory adapters, with the handles that a test needs.
struct Rig {
    /// The store that holds the rows and the audit entries.
    store: MemoryStore,
    /// The adapter that the job writes through. A test reads its call counters.
    maintenance: MemoryMaintenance,
    /// The adapter that a test reads the state key with.
    operations: MemoryOperations,
    /// The clock of the job. It differs from the clock of the store.
    clock: MemoryClock,
    /// The job under test.
    job: Heartbeat,
}

fn instant(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn rig() -> Rig {
    let store = MemoryStore::with_clock(MemoryClock::new(
        OffsetDateTime::from_unix_timestamp(STORE_SECONDS).unwrap(),
    ));
    let clock = MemoryClock::new(OffsetDateTime::from_unix_timestamp(JOB_SECONDS).unwrap());
    let maintenance = MemoryMaintenance::new(store.clone());
    let operations = MemoryOperations::new(store.clone());
    let job = Heartbeat::new(Arc::new(maintenance.clone()), Arc::new(clock.clone()));
    Rig {
        store,
        maintenance,
        operations,
        clock,
        job,
    }
}

async fn run_kind(rig: &Rig, kind: RunKind) -> Result<(), AppError> {
    rig.job
        .run_once(RequestId::from(Uuid::from_u128(1)), kind)
        .await
}

async fn run(rig: &Rig) -> Result<(), AppError> {
    run_kind(rig, RunKind::Scheduled).await
}

async fn stored(rig: &Rig) -> Option<Timestamp> {
    rig.operations
        .state(TimeStateKey::HeartbeatLastAt)
        .await
        .unwrap()
}

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first. A test thread
/// without a subscriber then caches `Interest::never` for all threads, and a capture misses a line.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

/// A subscriber that counts the events of its thread.
///
/// It solves one problem: `recipe-app` has no log subscriber, and a test must not install a global
/// one. The guard of [`LogCount::install`] limits the count to the thread of the test.
#[derive(Clone, Default)]
struct LogCount {
    /// The number of events seen.
    events: Arc<Mutex<usize>>,
}

impl LogCount {
    /// Count the events of this thread until the guard drops.
    fn install() -> (Self, DefaultGuard) {
        LazyLock::force(&KEEPALIVE);
        let count = Self::default();
        let guard = tracing::subscriber::set_default(count.clone());
        (count, guard)
    }

    /// Return the number of events seen so far.
    fn count(&self) -> usize {
        *self.events.lock()
    }
}

impl Subscriber for LogCount {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        let mut seen = self.events.lock();
        *seen = seen.saturating_add(1);
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

#[tokio::test]
async fn a_run_records_the_time_of_the_job_clock_in_the_state_key() {
    let rig = rig();
    assert_eq!(stored(&rig).await, None);
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig).await, Some(instant(JOB_SECONDS)));
}

#[tokio::test]
async fn a_later_run_replaces_the_recorded_time() {
    let rig = rig();
    run(&rig).await.unwrap();
    rig.clock.set(instant(LATER_SECONDS));
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig).await, Some(instant(LATER_SECONDS)));
}

#[tokio::test]
async fn a_run_makes_one_write_and_only_the_heartbeat_row() {
    let rig = rig();
    run(&rig).await.unwrap();
    assert_eq!(rig.maintenance.calls(MaintenanceMethod::RecordState), 1);
    let state = rig.store.state();
    let keys: Vec<&str> = state.settings.keys().map(|key| key.as_str()).collect();
    assert_eq!(keys, ["heartbeat.last_at"]);
}

#[tokio::test]
async fn a_run_writes_no_audit_entry_and_no_security_event() {
    let rig = rig();
    run(&rig).await.unwrap();
    run(&rig).await.unwrap();
    assert!(rig.store.audit_entries().is_empty());
    assert!(rig.store.sink().recorded().is_empty());
}

#[tokio::test]
async fn both_run_kinds_record_the_time() {
    let rig = rig();
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert_eq!(stored(&rig).await, Some(instant(JOB_SECONDS)));
    rig.clock.set(instant(LATER_SECONDS));
    run_kind(&rig, RunKind::Scheduled).await.unwrap();
    assert_eq!(stored(&rig).await, Some(instant(LATER_SECONDS)));
}

#[tokio::test]
async fn a_fault_of_the_store_returns_the_storage_error() {
    let rig = rig();
    rig.maintenance
        .fail_on_call(MaintenanceMethod::RecordState, 1, Fault::Unavailable);
    let error = run(&rig).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)));
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert_eq!(stored(&rig).await, None);
}

#[tokio::test]
async fn the_log_capture_sees_an_event_that_the_test_emits() {
    let (logged, _guard) = LogCount::install();
    assert_eq!(logged.count(), 0);
    tracing::info!("control event of the test");
    assert_eq!(logged.count(), 1);
}

#[tokio::test]
async fn a_successful_run_writes_no_log_line() {
    let (logged, _guard) = LogCount::install();
    let rig = rig();
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig).await, Some(instant(JOB_SECONDS)));
    assert_eq!(logged.count(), 0);
}

#[tokio::test]
async fn a_fault_of_the_store_writes_no_log_line() {
    let (logged, _guard) = LogCount::install();
    let rig = rig();
    rig.maintenance
        .fail_on_call(MaintenanceMethod::RecordState, 1, Fault::Unavailable);
    let error = run(&rig).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)));
    assert_eq!(logged.count(), 0);
}

#[tokio::test]
async fn the_run_after_a_fault_records_the_time() {
    let rig = rig();
    rig.maintenance
        .fail_on_call(MaintenanceMethod::RecordState, 1, Fault::StatementTimeout);
    assert_eq!(run(&rig).await.unwrap_err().code(), ErrorCode::DB_002);
    run(&rig).await.unwrap();
    assert_eq!(stored(&rig).await, Some(instant(JOB_SECONDS)));
}
