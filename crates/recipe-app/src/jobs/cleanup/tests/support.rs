//! Helpers that the tests of the job `cleanup` share.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use recipe_domain::audit::RequestId;
use recipe_domain::identity::{CsrfToken, SessionId, SessionToken, TypedLogin, UserId};
use recipe_domain::timestamp::Timestamp;
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;

use crate::error::AppError;
use crate::jobs::RunKind;
use crate::jobs::cleanup::{Cleanup, CleanupSettings};
use crate::memory::{MaintenanceMethod, MemoryClock, MemoryMaintenance, MemoryStore};
use crate::ports::{Clock, NewSession};

/// The time of the clock at the start of a test, in Unix seconds.
const BASE_SECONDS: i64 = 1_000_000;
/// The idle timeout of a seeded session.
const IDLE: Duration = Duration::from_secs(30 * 60);
/// The absolute session timeout that a default rig passes to the job: 12 hours.
const ABSOLUTE_TIMEOUT: Duration = Duration::from_secs(12 * 60 * 60);
/// One minute in seconds.
pub(super) const MINUTE: i64 = 60;
/// One hour in seconds.
pub(super) const HOUR: i64 = 60 * MINUTE;
/// Ten minutes in seconds. This is the default cleanup interval.
pub(super) const TEN_MINUTES: i64 = 10 * MINUTE;
/// Five hours in seconds.
pub(super) const FIVE_HOURS: i64 = 5 * HOUR;
/// One day in seconds.
pub(super) const DAY: i64 = 24 * HOUR;
/// Two days in seconds. This is above every default retention.
const TWO_DAYS: i64 = 2 * DAY;

/// A job on the in-memory adapters, with the handles that a test needs.
pub(super) struct Rig {
    /// The store that holds the rows. Its clock is the clock of the job.
    pub store: MemoryStore,
    /// The adapter that the job writes through. A test reads its call counters.
    pub maintenance: MemoryMaintenance,
    /// The job under test.
    pub job: Cleanup,
}

/// The instant `seconds` after the start of the test. A negative value is before it.
pub(super) fn at(seconds: i64) -> Timestamp {
    let base = OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap();
    Timestamp::from(base.checked_add(TimeDuration::seconds(seconds)).unwrap())
}

/// Return `seconds` minus `by`.
pub(super) fn before(seconds: i64, by: i64) -> i64 {
    seconds.checked_sub(by).unwrap()
}

/// Return `seconds` plus `by`.
pub(super) fn after(seconds: i64, by: i64) -> i64 {
    seconds.checked_add(by).unwrap()
}

/// Build a job with `settings` and the default absolute timeout of 12 hours.
pub(super) fn rig(settings: CleanupSettings) -> Rig {
    rig_with_timeout(settings, ABSOLUTE_TIMEOUT)
}

/// Build a job with `settings` and `timeout` on a store whose clock starts at the test start.
pub(super) fn rig_with_timeout(settings: CleanupSettings, timeout: Duration) -> Rig {
    let store = new_store();
    let clock = Arc::new(store.clock());
    build_rig(store, clock, settings, timeout)
}

/// Build a job whose own clock moves forward by `step` at each read.
///
/// The store keeps the plain clock. Only the reads of the job move the time, so the time that
/// the job records shows which of its reads it used.
pub(super) fn rig_stepping(settings: CleanupSettings, step: Duration) -> Rig {
    let store = new_store();
    let clock = Arc::new(SteppingClock {
        inner: store.clock(),
        step: TimeDuration::try_from(step).unwrap(),
    });
    build_rig(store, clock, settings, ABSOLUTE_TIMEOUT)
}

/// A clock that returns the time of the inner clock and then moves it forward by `step`.
///
/// It solves one problem: the memory clock never moves during a run, so no test could tell the
/// start of a run from its end.
struct SteppingClock {
    /// The clock that holds the time. A test sets it through the store.
    inner: MemoryClock,
    /// The distance that each read adds to the inner clock.
    step: TimeDuration,
}

impl Clock for SteppingClock {
    fn now(&self) -> Timestamp {
        let current = self.inner.now();
        self.inner.advance(self.step);
        current
    }
}

fn new_store() -> MemoryStore {
    MemoryStore::with_clock(MemoryClock::new(
        OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap(),
    ))
}

fn build_rig(
    store: MemoryStore,
    clock: Arc<dyn Clock>,
    settings: CleanupSettings,
    timeout: Duration,
) -> Rig {
    let maintenance = MemoryMaintenance::new(store.clone());
    let job = Cleanup::new(Arc::new(maintenance.clone()), clock, timeout, settings);
    Rig {
        store,
        maintenance,
        job,
    }
}

/// Move the clock of the rig to `seconds` after the start of the test.
pub(super) fn set_time(rig: &Rig, seconds: i64) {
    rig.store.clock().set(at(seconds));
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

/// Return the number of calls that `method` of the store received.
pub(super) fn calls(rig: &Rig, method: MaintenanceMethod) -> u64 {
    rig.maintenance.calls(method)
}

/// Seed a session whose idle expiry lies 30 minutes before `now_seconds`, then set the clock.
pub(super) fn seed_expired_session(rig: &Rig, n: u8, now_seconds: i64) {
    set_time(rig, before(now_seconds, HOUR));
    rig.store.seed_session(&new_session(n));
    set_time(rig, now_seconds);
}

/// Seed a session that is created `age` seconds before `now_seconds` and idle for a short time.
pub(super) fn seed_session_aged(rig: &Rig, n: u8, now_seconds: i64, age: i64) {
    set_time(rig, before(now_seconds, age));
    rig.store.seed_session(&new_session(n));
    set_time(rig, now_seconds);
}

/// Seed `count` login attempts that are two days old at `now_seconds`.
pub(super) fn seed_old_attempts(rig: &Rig, count: u32, now_seconds: i64) {
    for _ in 0..count {
        seed_attempt_aged(rig, now_seconds, TWO_DAYS);
    }
}

/// Seed one login attempt that is `age` seconds old at `now_seconds`.
pub(super) fn seed_attempt_aged(rig: &Rig, now_seconds: i64, age: i64) {
    let login = TypedLogin::parse("anna").unwrap();
    let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
    rig.store
        .seed_attempt(&login, ip, at(before(now_seconds, age)), false);
}

fn new_session(n: u8) -> NewSession {
    NewSession {
        id: SessionId::of(&SessionToken::from_bytes([n; 16])),
        user_id: UserId::from(Uuid::from_u128(1)),
        epoch: 0,
        csrf_token: CsrfToken::from_bytes([n; 32]),
        idle_timeout: IDLE,
    }
}
