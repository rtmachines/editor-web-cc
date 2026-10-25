//! Helpers that the tests of the job `clock_check` share.

use std::fmt::Debug;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use parking_lot::Mutex;
use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;
use time::{OffsetDateTime, PrimitiveDateTime};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Event, Level, Metadata, Subscriber};
use uuid::Uuid;

use crate::error::AppError;
use crate::jobs::RunKind;
use crate::jobs::clock_check::{ClockCheck, ClockCheckSettings};
use crate::memory::{MemoryClock, MemoryOperations, MemoryStore};

/// The time of the clock of the database at the start of a test, in Unix seconds.
const BASE_SECONDS: i64 = 1_000_000;
/// Nanoseconds in one second.
pub(super) const SECOND: i128 = 1_000_000_000;

/// A job on the in-memory adapters, with the handles that a test needs.
pub(super) struct Rig {
    /// The adapter that reads the time of the database. A test reads its call counters.
    pub operations: MemoryOperations,
    /// The clock of the database. The adapter `operations` reads it.
    pub database: MemoryClock,
    /// The clock of the host. It is the clock of the job.
    pub host: MemoryClock,
    /// The job under test.
    pub job: ClockCheck,
}

/// Build a job with `settings`. Both clocks show the same time.
pub(super) fn rig_with(settings: ClockCheckSettings) -> Rig {
    let start = OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap();
    let database = MemoryClock::new(start);
    let host = MemoryClock::new(start);
    let store = MemoryStore::with_clock(database.clone());
    let operations = MemoryOperations::new(store);
    let job = ClockCheck::new(
        Arc::new(operations.clone()),
        Arc::new(host.clone()),
        settings,
    );
    Rig {
        operations,
        database,
        host,
        job,
    }
}

/// Build a job with the default settings.
pub(super) fn rig() -> Rig {
    rig_with(ClockCheckSettings::default())
}

/// Move the clock of the host to `nanos` nanoseconds after the clock of the database.
///
/// A negative value puts the host behind the database.
pub(super) fn set_host_offset(rig: &Rig, nanos: i128) {
    let base = i128::from(BASE_SECONDS).checked_mul(SECOND).unwrap();
    let host = OffsetDateTime::from_unix_timestamp_nanos(base.checked_add(nanos).unwrap());
    rig.host.set(Timestamp::from(host.unwrap()));
}

/// Set the clock of the database to `database` and the clock of the host to `host`.
pub(super) fn set_clocks(rig: &Rig, database: OffsetDateTime, host: OffsetDateTime) {
    rig.database.set(Timestamp::from(database));
    rig.host.set(Timestamp::from(host));
}

/// Return `seconds` seconds in nanoseconds.
pub(super) fn seconds(seconds: i128) -> i128 {
    seconds.checked_mul(SECOND).unwrap()
}

/// Return the negative of `nanos`. A host that is behind uses a negative offset.
pub(super) fn behind(nanos: i128) -> i128 {
    nanos.checked_neg().unwrap()
}

/// Return the instant that is `nanos` nanoseconds after the Unix epoch.
///
/// A negative value gives an instant before the epoch.
pub(super) fn at_nanos(nanos: i128) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(nanos).unwrap()
}

/// Return the earliest instant that `OffsetDateTime` holds, in UTC.
///
/// The instant type of the ports is `Timestamp`, and it wraps `OffsetDateTime`.
pub(super) fn earliest() -> OffsetDateTime {
    PrimitiveDateTime::MIN.assume_utc()
}

/// Return the latest instant that `OffsetDateTime` holds, in UTC.
pub(super) fn latest() -> OffsetDateTime {
    PrimitiveDateTime::MAX.assume_utc()
}

/// Return the instant that is `nanos` nanoseconds after `instant`.
///
/// A negative value gives an earlier instant.
pub(super) fn shifted(instant: OffsetDateTime, nanos: i128) -> OffsetDateTime {
    instant
        .checked_add(time::Duration::nanoseconds_i128(nanos))
        .unwrap()
}

/// Return the skew of two instants as a `Duration`, computed with `i128` nanoseconds.
///
/// The way differs from the way of the job on purpose, so a test does not repeat the job.
pub(super) fn exact_skew(first: OffsetDateTime, second: OffsetDateTime) -> Duration {
    let nanos = first
        .unix_timestamp_nanos()
        .abs_diff(second.unix_timestamp_nanos());
    let second_nanos = u128::try_from(SECOND).unwrap();
    let whole = u64::try_from(nanos.checked_div(second_nanos).unwrap()).unwrap();
    let fraction = u32::try_from(nanos.checked_rem(second_nanos).unwrap()).unwrap();
    Duration::new(whole, fraction)
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

/// One event that the job wrote.
#[derive(Clone, Debug)]
pub(super) struct Logged {
    /// The level of the event.
    pub level: Level,
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
    /// The fields in write order.
    fields: Vec<(String, String)>,
}

impl Visit for FieldCopy {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.fields
            .push((field.name().to_owned(), format!("{value:?}")));
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
            fields: copy.fields,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}
