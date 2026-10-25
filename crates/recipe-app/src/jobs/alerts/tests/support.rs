//! Helpers that the tests of the job `alerts` share.

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;
use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::sync::Notify;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Event, Level, Metadata, Subscriber};
use uuid::Uuid;

use crate::error::{AppError, StorageError};
use crate::jobs::RunKind;
use crate::jobs::alerts::{AlertPorts, AlertSettings, AlertSources, AlertVolumes, Alerts};
use crate::memory::{DiskReading, MemoryBackupDirectory, MemoryDiskSpace, MemoryWebhook};
use crate::operations::{AlertCounters, ReadinessState};
use crate::ports::{
    AlertCondition, AlertWebhook, OperationsReads, OperationsSnapshot, ReplicationState, RoleCheck,
    SnapshotWindows, TimeStateKey, WebhookFault,
};

/// The time of the database at the start of a test, in Unix seconds.
const BASE_SECONDS: i64 = 10_000_000;
/// The blocks of each file system in the tests. One block is one tenth of one percent.
pub(super) const TOTAL: u64 = 1_000;
/// One minute in seconds.
pub(super) const MINUTE: i64 = 60;
/// One hour in seconds.
pub(super) const HOUR: i64 = 60 * MINUTE;
/// 26 hours in seconds: `audit.max_check_age` and `backup.max_age_alert`.
pub(super) const WINDOW_26H: i64 = 26 * HOUR;
/// One day in seconds.
pub(super) const DAY: i64 = 24 * HOUR;
/// The text of a fault of the disk port that is not `Unsupported`.
pub(super) const UNREADABLE_TEXT: &str = "The free disk space cannot be read.";
/// The text of the fault of the backup directory.
pub(super) const BACKUP_TEXT: &str = "The backup directory cannot be read.";

/// The instant `seconds` after the start of the test. A negative value is before it.
pub(super) fn at(seconds: i64) -> Timestamp {
    let base = OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap();
    Timestamp::from(base.checked_add(TimeDuration::seconds(seconds)).unwrap())
}

/// The instant `seconds` before the start of the test.
pub(super) fn earlier(seconds: i64) -> Timestamp {
    at(seconds.checked_neg().unwrap())
}

/// The calls that the job made on the stub of `OperationsReads`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Calls {
    /// The windows of each call of `snapshot`.
    pub snapshots: Vec<SnapshotWindows>,
    /// The number of calls of any other method.
    pub others: u64,
}

/// The values that the stub holds.
struct StubInner {
    /// The snapshot that the stub returns.
    snapshot: OperationsSnapshot,
    /// True when `snapshot` fails.
    failing: bool,
    /// The calls so far.
    calls: Calls,
    /// The gate that holds a call of `snapshot`, if a test asked for one.
    gate: Option<Gate>,
}

/// A stub of `OperationsReads` that returns a snapshot that the test builds.
///
/// It solves one problem: the memory adapter derives its snapshot from rows, and a test of a
/// condition needs to set each field of the snapshot directly, also a time in the future.
#[derive(Clone)]
pub(super) struct StubOperations {
    /// The values of the stub.
    inner: Arc<Mutex<StubInner>>,
}

impl StubOperations {
    /// Build a stub with the snapshot of a healthy system at the start time of the test.
    fn healthy() -> Self {
        let snapshot = OperationsSnapshot {
            database_now: at(0),
            replication: ReplicationState {
                connected: true,
                lag_bytes: Some(0),
            },
            failed_logins: 0,
            security_alerts: 0,
            audit_chain_broken_at: None,
            audit_last_check_at: Some(at(-HOUR)),
            first_setting_at: Some(earlier(times(30, DAY))),
        };
        Self {
            inner: Arc::new(Mutex::new(StubInner {
                snapshot,
                failing: false,
                calls: Calls::default(),
                gate: None,
            })),
        }
    }

    /// Give `snapshot` a gate and return it. The gate is open until the test arms it.
    pub fn gate(&self) -> Gate {
        let gate = Gate::default();
        self.inner.lock().gate = Some(gate.clone());
        gate
    }

    /// Change the snapshot that the stub returns.
    pub fn edit(&self, change: impl FnOnce(&mut OperationsSnapshot)) {
        change(&mut self.inner.lock().snapshot);
    }

    /// Make every later call of `snapshot` fail, or succeed again.
    pub fn set_failing(&self, failing: bool) {
        self.inner.lock().failing = failing;
    }

    /// Return the calls so far.
    pub fn calls(&self) -> Calls {
        self.inner.lock().calls.clone()
    }

    /// Count a call of a method that the job must not use.
    fn other(&self) -> Result<(), StorageError> {
        let mut inner = self.inner.lock();
        inner.calls.others = inner.calls.others.checked_add(1).unwrap();
        Err(StorageError::Unavailable)
    }
}

#[async_trait]
impl OperationsReads for StubOperations {
    async fn snapshot(
        &self,
        windows: &SnapshotWindows,
    ) -> Result<OperationsSnapshot, StorageError> {
        let held = self.inner.lock().gate.clone();
        if let Some(gate) = held {
            gate.hold().await;
        }
        let mut inner = self.inner.lock();
        inner.calls.snapshots.push(*windows);
        if inner.failing {
            return Err(StorageError::Unavailable);
        }
        Ok(inner.snapshot.clone())
    }

    async fn database_now(&self) -> Result<Timestamp, StorageError> {
        self.other().map(|()| at(0))
    }

    async fn role_check(&self) -> Result<RoleCheck, StorageError> {
        self.other().map(|()| RoleCheck {
            current_user: String::new(),
            is_superuser: false,
            owns_relations: false,
        })
    }

    async fn state(&self, _key: TimeStateKey) -> Result<Option<Timestamp>, StorageError> {
        self.other().map(|()| None)
    }
}

/// A job on stubs and in-memory adapters, with the handles that a test needs.
pub(super) struct Rig {
    /// The stub that returns the snapshot.
    pub operations: StubOperations,
    /// The adapter that records the calls of the webhook.
    pub webhook: MemoryWebhook,
    /// The adapter that holds the reading of each volume.
    pub disk: MemoryDiskSpace,
    /// The adapter that holds the newest set of the backup directory.
    pub backups: MemoryBackupDirectory,
    /// The counters that the job reads.
    pub counters: AlertCounters,
    /// The readiness state that the job reads.
    pub readiness: ReadinessState,
    /// The job under test.
    pub job: Alerts,
}

/// The path of the volume `pgdata` in the tests.
pub(super) fn pgdata() -> PathBuf {
    PathBuf::from("/srv/pgdata")
}

/// The path of the volume `backups` in the tests.
pub(super) fn backups_path() -> PathBuf {
    PathBuf::from("/srv/backups")
}

/// The path of the volume `walarchive` in the tests.
pub(super) fn walarchive() -> PathBuf {
    PathBuf::from("/srv/walarchive")
}

/// The three volumes in the order of the job: name and path.
pub(super) fn volumes() -> [(&'static str, PathBuf); 3] {
    [
        ("pgdata", pgdata()),
        ("backups", backups_path()),
        ("walarchive", walarchive()),
    ]
}

/// Build a healthy job with the default settings.
///
/// The snapshot is clean, the server is ready, the newest set is one hour old, and each volume
/// has 50 % free.
pub(super) fn rig() -> Rig {
    rig_with(AlertSettings::default())
}

/// Build a healthy job with `settings`.
pub(super) fn rig_with(settings: AlertSettings) -> Rig {
    let webhook = MemoryWebhook::default();
    rig_on(settings, webhook.clone(), Arc::new(webhook))
}

/// Build a healthy job with `settings`, where the job calls `port` and `webhook` records the calls.
///
/// The caller passes a port that wraps `webhook`, for example a [`GatedWebhook`].
pub(super) fn rig_on(
    settings: AlertSettings,
    webhook: MemoryWebhook,
    port: Arc<dyn AlertWebhook>,
) -> Rig {
    let operations = StubOperations::healthy();
    let disk = MemoryDiskSpace::default();
    let backups = MemoryBackupDirectory::default();
    let counters = AlertCounters::default();
    let readiness = ReadinessState::new(at(-DAY));
    readiness.mark_ready();
    backups.set_newest(Some(at(-HOUR)));
    for (_, path) in volumes() {
        disk.set(&path, space(TOTAL / 2));
    }
    let job = Alerts::new(
        AlertPorts {
            operations: Arc::new(operations.clone()),
            webhook: port,
            disk: Arc::new(disk.clone()),
            backups: Arc::new(backups.clone()),
        },
        AlertSources {
            counters: counters.clone(),
            readiness: readiness.clone(),
        },
        AlertVolumes {
            pgdata: pgdata(),
            backups: backups_path(),
            walarchive: walarchive(),
        },
        settings,
    );
    Rig {
        operations,
        webhook,
        disk,
        backups,
        counters,
        readiness,
        job,
    }
}

/// The signals of a gate that holds one awaited port call of the job.
///
/// The type solves one problem: a test must stop the job at an awaited call, change the world, or
/// drop the run there. A new gate is open. [`Gate::arm`] holds the next call only.
#[derive(Clone, Default)]
pub(super) struct Gate {
    /// The port notifies it when a call has arrived.
    reached: Arc<Notify>,
    /// The test notifies it to let the call go on.
    release: Arc<Notify>,
    /// True until the next call has arrived.
    armed: Arc<AtomicBool>,
}

impl Gate {
    /// Hold the next call until the test releases it.
    pub fn arm(&self) {
        self.armed.store(true, Ordering::Relaxed);
    }

    /// Wait until the port holds a call.
    pub async fn wait_reached(&self) {
        self.reached.notified().await;
    }

    /// Let the held call go on.
    pub fn release(&self) {
        self.release.notify_one();
    }

    /// Hold the call of the port when the gate is armed. Else return at once.
    async fn hold(&self) {
        if self.armed.swap(false, Ordering::Relaxed) {
            self.reached.notify_one();
            self.release.notified().await;
        }
    }
}

/// An `AlertWebhook` that holds its first call until the test releases it.
///
/// It solves one problem: a test must change a counter while the job waits for an awaited port
/// call. The adapter records the call after the release. The later calls pass at once, until the
/// test arms the gate again.
pub(super) struct GatedWebhook {
    /// The adapter that records the calls.
    inner: MemoryWebhook,
    /// The gate of the call.
    gate: Gate,
}

impl GatedWebhook {
    /// Build the adapter over `inner` and return it with the handle of its armed gate.
    pub fn new(inner: MemoryWebhook) -> (Self, Gate) {
        let gate = Gate::default();
        gate.arm();
        let webhook = Self {
            inner,
            gate: gate.clone(),
        };
        (webhook, gate)
    }
}

#[async_trait]
impl AlertWebhook for GatedWebhook {
    async fn send_all(&self, conditions: &[AlertCondition]) -> Vec<Result<(), WebhookFault>> {
        self.gate.hold().await;
        self.inner.send_all(conditions).await
    }
}

/// Build a healthy job whose webhook holds its first call. Return the rig and the gate.
pub(super) fn gated_rig() -> (Rig, Gate) {
    let webhook = MemoryWebhook::default();
    let (gated, gate) = GatedWebhook::new(webhook.clone());
    (
        rig_on(AlertSettings::default(), webhook, Arc::new(gated)),
        gate,
    )
}

/// The reading of a file system with `available` free blocks of [`TOTAL`].
pub(super) fn space(available: u64) -> DiskReading {
    DiskReading::Space(crate::ports::FreeSpace {
        available_blocks: available,
        total_blocks: TOTAL,
    })
}

/// Move the database time of the snapshot to `seconds` after the start of the test.
pub(super) fn set_time(rig: &Rig, seconds: i64) {
    rig.operations
        .edit(|snapshot| snapshot.database_now = at(seconds));
}

/// Run the job once as a scheduled run.
pub(super) async fn run(rig: &Rig) -> Result<(), AppError> {
    run_kind(rig, RunKind::Scheduled).await
}

/// Run the job once with the run kind `kind`.
pub(super) async fn run_kind(rig: &Rig, kind: RunKind) -> Result<(), AppError> {
    let request_id = RequestId::from(Uuid::from_u128(1));
    rig.job.run_once(request_id, kind).await
}

/// Return `count` times `unit`.
pub(super) fn times(count: i64, unit: i64) -> i64 {
    count.checked_mul(unit).unwrap()
}

/// Return the conditions of the last call of the webhook, or an empty list when no call came.
pub(super) fn last_sent(rig: &Rig) -> Vec<AlertCondition> {
    rig.webhook.calls().pop().unwrap_or_default()
}

/// Run the job once and return the conditions that it sent to the webhook in that run.
pub(super) async fn sent_by_one_run(rig: &Rig) -> Vec<AlertCondition> {
    let before = rig.webhook.calls().len();
    run(rig).await.unwrap();
    let calls = rig.webhook.calls();
    assert!(calls.len() <= before.checked_add(1).unwrap());
    if calls.len() == before {
        Vec::new()
    } else {
        last_sent(rig)
    }
}

/// One event that the job wrote.
#[derive(Clone, Debug)]
pub(super) struct Logged {
    /// The level of the event.
    pub level: Level,
    /// The message of the event.
    pub message: String,
    /// The other fields of the event, as name and `Debug` text.
    pub fields: Vec<(String, String)>,
}

impl Logged {
    /// Return the `Debug` text of the field `name`, if the event has it.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first. A test thread
/// without a subscriber then caches `Interest::never` for all threads, and a capture misses a line.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

/// A subscriber that keeps every event of its thread.
///
/// It solves one problem: `recipe-app` has no log subscriber, and a test must not install a global
/// one. The guard of [`LogCapture::install`] limits the capture to the thread of the test.
#[derive(Clone, Default)]
pub(super) struct LogCapture {
    /// The events in write order.
    events: Arc<Mutex<Vec<Logged>>>,
}

impl LogCapture {
    /// Capture the events of this thread until the guard drops.
    pub fn install() -> (Self, DefaultGuard) {
        LazyLock::force(&KEEPALIVE);
        let capture = Self::default();
        let guard = tracing::subscriber::set_default(capture.clone());
        (capture, guard)
    }

    /// Return the captured events in write order.
    pub fn events(&self) -> Vec<Logged> {
        self.events.lock().clone()
    }

    /// Return the number of captured events.
    pub fn count(&self) -> usize {
        self.events.lock().len()
    }

    /// Return the captured events that have the field `alert`, in write order.
    pub fn alert_lines(&self) -> Vec<Logged> {
        self.events()
            .into_iter()
            .filter(|event| event.field("alert").is_some())
            .collect()
    }

    /// Return the unquoted values of the field `alert`, in write order.
    pub fn alert_names(&self) -> Vec<String> {
        self.alert_lines()
            .iter()
            .filter_map(|event| event.field("alert"))
            .map(|text| text.trim_matches('"').to_owned())
            .collect()
    }
}

/// A visitor that copies the fields of one event.
#[derive(Default)]
struct FieldCopy {
    /// The text of the field `message`.
    message: String,
    /// The other fields.
    fields: Vec<(String, String)>,
}

impl Visit for FieldCopy {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        let text = format!("{value:?}");
        if field.name() == "message" {
            self.message = text;
        } else {
            self.fields.push((field.name().to_owned(), text));
        }
    }
}

impl Subscriber for LogCapture {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut copy = FieldCopy::default();
        event.record(&mut copy);
        self.events.lock().push(Logged {
            level: *event.metadata().level(),
            message: copy.message,
            fields: copy.fields,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}
