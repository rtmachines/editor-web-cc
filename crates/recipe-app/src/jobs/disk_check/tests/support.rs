//! Helpers that the tests of the job `disk_check` share.

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use parking_lot::Mutex;
use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;
use time::{Duration as TimeDuration, OffsetDateTime};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Event, Level, Metadata, Subscriber};
use uuid::Uuid;

use crate::error::AppError;
use crate::jobs::RunKind;
use crate::jobs::disk_check::{DiskCheck, DiskCheckSettings};
use crate::memory::{DiskFailure, DiskReading, MemoryClock, MemoryDiskSpace, MemoryReadOnly};
use crate::ports::FreeSpace;

/// The time of the clock at the start of a test, in Unix seconds.
const BASE_SECONDS: i64 = 1_000_000;
/// The blocks of the file system in most tests. One block is one tenth of one percent.
pub(super) const TOTAL: u64 = 1_000;
/// One minute in seconds.
pub(super) const MINUTE: i64 = 60;
/// One hour in seconds.
pub(super) const HOUR: i64 = 60 * MINUTE;
/// Ten minutes in seconds. This is the default `disk.warn_log_interval`.
pub(super) const TEN_MINUTES: i64 = 10 * MINUTE;
/// One day in seconds.
pub(super) const DAY: i64 = 24 * HOUR;
/// The text of a fault of the port that is not `Unsupported`.
pub(super) const UNREADABLE_TEXT: &str = "The free disk space cannot be read.";
/// The text of the fault `Unsupported`.
pub(super) const UNSUPPORTED_TEXT: &str = "The free disk space cannot be read on this system.";

/// A job on the in-memory adapters, with the handles that a test needs.
pub(super) struct Rig {
    /// The adapter that holds the reading of each path. A test reads its calls.
    pub disk: MemoryDiskSpace,
    /// The switch that the job sets. A test reads its flag and its history.
    pub switch: MemoryReadOnly,
    /// The clock of the job.
    pub clock: MemoryClock,
    /// The job under test.
    pub job: DiskCheck,
}

/// The path of the volume `pgdata` in the tests.
pub(super) fn pgdata() -> PathBuf {
    PathBuf::from("/srv/pgdata")
}

/// The instant `seconds` after the start of the test.
pub(super) fn at(seconds: i64) -> Timestamp {
    let base = OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap();
    Timestamp::from(base.checked_add(TimeDuration::seconds(seconds)).unwrap())
}

/// Return `seconds` plus `by`.
pub(super) fn after(seconds: i64, by: i64) -> i64 {
    seconds.checked_add(by).unwrap()
}

/// Build a job with the default settings. The switch starts off, and `pgdata` has 50 % free.
pub(super) fn rig() -> Rig {
    rig_with(DiskCheckSettings::default(), false)
}

/// Build a job with `settings`. The switch starts as `mode_on`, and `pgdata` has 50 % free.
pub(super) fn rig_with(settings: DiskCheckSettings, mode_on: bool) -> Rig {
    let disk = MemoryDiskSpace::default();
    let switch = MemoryReadOnly::new(mode_on);
    let clock = MemoryClock::new(OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap());
    let job = DiskCheck::new(
        Arc::new(disk.clone()),
        Arc::new(switch.clone()),
        Arc::new(clock.clone()),
        pgdata(),
        settings,
    );
    let rig = Rig {
        disk,
        switch,
        clock,
        job,
    };
    set_available(&rig, TOTAL / 2);
    rig
}

/// Set the reading of `pgdata` to `available` blocks of [`TOTAL`].
pub(super) fn set_available(rig: &Rig, available: u64) {
    set_space(rig, available, TOTAL);
}

/// Set the reading of `pgdata` to `available` blocks of `total`.
pub(super) fn set_space(rig: &Rig, available: u64, total: u64) {
    let space = FreeSpace {
        available_blocks: available,
        total_blocks: total,
    };
    rig.disk.set(&pgdata(), DiskReading::Space(space));
}

/// Make the reading of `pgdata` fail with `failure`.
pub(super) fn set_failure(rig: &Rig, failure: DiskFailure) {
    rig.disk.set(&pgdata(), DiskReading::Failure(failure));
}

/// Move the clock of the rig to `seconds` after the start of the test.
pub(super) fn set_time(rig: &Rig, seconds: i64) {
    rig.clock.set(at(seconds));
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
