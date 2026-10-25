//! Job `cleanup`: delete expired sessions and old rows (design D6, ADR-0088 (c)).
//!
//! Purpose: keep the tables `session`, `login_attempt`, and `security_event` small.
//!
//! Schedule: the sessions every `session.cleanup_interval`, the login attempts every
//! `login.attempt_cleanup_interval`. The security event purge runs when
//! `security_event.purge_interval` has passed since its last run.
//!
//! One loop of the server calls [`Cleanup::run_once`] at the shorter of the first two intervals.
//! The job keeps the start of the last run of each cadence. A cadence is due when it never ran
//! in this process, or when its interval has passed. Each statement runs in its own short
//! transaction through the port, so a run never holds a long lock. The job writes no audit entry
//! and no log line.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;

use super::RunKind;
use crate::error::{AppError, StorageError};
use crate::ports::{Clock, MaintenanceStore};

/// The rows that one call of `security_event_purge()` deletes at most.
///
/// The value equals `security_event.purge_batch` in `reliability.md` section 7.11. The function
/// holds the number as a constant, so the job only compares it with the result of a call.
pub const SECURITY_EVENT_PURGE_BATCH: u64 = 5_000;

/// Value of a cadence atomic before its first run. No instant has this value.
const NEVER: i64 = i64::MIN;
/// Default of `session.cleanup_interval` in seconds: 10 minutes.
const DEFAULT_SESSION_INTERVAL_SECONDS: u64 = 10 * 60;
/// Default of `login.attempt_cleanup_interval` in seconds: 10 minutes.
const DEFAULT_ATTEMPT_INTERVAL_SECONDS: u64 = 10 * 60;
/// Default of `login.attempt_retention` in seconds: 24 hours.
const DEFAULT_ATTEMPT_RETENTION_SECONDS: u64 = 24 * 60 * 60;
/// Default of `security_event.purge_interval` in seconds: 24 hours.
const DEFAULT_SECURITY_INTERVAL_SECONDS: u64 = 24 * 60 * 60;
/// Default of `login_attempt.purge_batch`.
const DEFAULT_ATTEMPT_BATCH: u32 = 5_000;

/// The numeric settings that the job `cleanup` reads.
///
/// The type solves one problem: a job must not read a configuration file. `recipe-server` builds
/// this value and passes it down. `Default` equals the table of `reliability.md` section 7.11,
/// and a test compares both. The type has no `session.absolute_timeout`, because
/// `IdentitySettings` owns that key. [`Cleanup::new`] takes it as a separate argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CleanupSettings {
    /// The time between two session purges (`session.cleanup_interval`).
    pub session_cleanup_interval: Duration,
    /// The time between two login attempt purges (`login.attempt_cleanup_interval`).
    pub login_attempt_cleanup_interval: Duration,
    /// The age after which a login attempt is old (`login.attempt_retention`).
    pub login_attempt_retention: Duration,
    /// The time between two security event purges (`security_event.purge_interval`).
    pub security_event_purge_interval: Duration,
    /// The rows of one login attempt statement (`login_attempt.purge_batch`).
    pub login_attempt_purge_batch: u32,
}

impl CleanupSettings {
    /// Return the interval of the loop that calls the job.
    ///
    /// It is the shorter of the two cleanup intervals (ADR-0088 (c)). The security event purge
    /// has its own due time and needs no faster loop.
    #[must_use]
    pub fn loop_interval(&self) -> Duration {
        self.session_cleanup_interval
            .min(self.login_attempt_cleanup_interval)
    }
}

impl Default for CleanupSettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            session_cleanup_interval: Duration::from_secs(DEFAULT_SESSION_INTERVAL_SECONDS),
            login_attempt_cleanup_interval: Duration::from_secs(DEFAULT_ATTEMPT_INTERVAL_SECONDS),
            login_attempt_retention: Duration::from_secs(DEFAULT_ATTEMPT_RETENTION_SECONDS),
            security_event_purge_interval: Duration::from_secs(DEFAULT_SECURITY_INTERVAL_SECONDS),
            login_attempt_purge_batch: DEFAULT_ATTEMPT_BATCH,
        }
    }
}

/// The supervised job that purges old rows of three tables.
///
/// The type solves one problem: the tables `session`, `login_attempt`, and `security_event` must
/// not grow without bound, and one run must not hold a long lock. The job has two cadences in
/// one loop (ADR-0088 (c)). It keeps the start of the last run of each cadence as an atomic
/// (`reliability.md` section 8). A cadence records its start only after it ended without an
/// error. A failed cadence therefore stays due, and the next run continues it.
pub struct Cleanup {
    /// The port that runs the purge statements.
    store: Arc<dyn MaintenanceStore>,
    /// The source of the time of a run.
    clock: Arc<dyn Clock>,
    /// The age after which a session ends (`session.absolute_timeout`).
    session_absolute_timeout: Duration,
    /// The intervals, the retention, and the batch size.
    settings: CleanupSettings,
    /// Epoch seconds of the start of the last run that ended the session purge without an error, or [`NEVER`].
    sessions_ran_at: AtomicI64,
    /// Epoch seconds of the start of the last run that ended the login attempt purge without an error, or [`NEVER`].
    attempts_ran_at: AtomicI64,
    /// Epoch seconds of the start of the last run that ended the security event purge without an error, or [`NEVER`].
    security_events_ran_at: AtomicI64,
}

impl Cleanup {
    /// Build the job. No cadence has run yet, so the first run executes all three.
    ///
    /// The caller passes `IdentitySettings::session_absolute_timeout` as the timeout.
    /// The job has no copy of this value in its settings, so the two can never differ.
    #[must_use]
    pub fn new(
        store: Arc<dyn MaintenanceStore>,
        clock: Arc<dyn Clock>,
        session_absolute_timeout: Duration,
        settings: CleanupSettings,
    ) -> Self {
        Self {
            store,
            clock,
            session_absolute_timeout,
            settings,
            sessions_ran_at: AtomicI64::new(NEVER),
            attempts_ran_at: AtomicI64::new(NEVER),
            security_events_ran_at: AtomicI64::new(NEVER),
        }
    }

    /// Run every cadence that is due, in the order sessions, login attempts, security events.
    ///
    /// The run kind has no effect, because the job has one trigger (ADR-0088 (b)). The run
    /// writes no row with the request id. The attempt and security event purges repeat their
    /// statement while a batch is full.
    ///
    /// # Errors
    ///
    /// Returns the `StorageError` of the first failed statement as `AppError::Storage`. The
    /// batches before it stay deleted, and its cadence stays due for the next run.
    pub async fn run_once(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        let started = epoch_seconds(self.clock.now());
        let settings = &self.settings;
        if is_due(
            &self.sessions_ran_at,
            started,
            settings.session_cleanup_interval,
        ) {
            self.store
                .delete_expired_sessions(self.session_absolute_timeout)
                .await?;
            self.sessions_ran_at.store(started, Ordering::Relaxed);
        }
        if is_due(
            &self.attempts_ran_at,
            started,
            settings.login_attempt_cleanup_interval,
        ) {
            self.purge_login_attempts().await?;
            self.attempts_ran_at.store(started, Ordering::Relaxed);
        }
        if is_due(
            &self.security_events_ran_at,
            started,
            settings.security_event_purge_interval,
        ) {
            self.purge_security_events().await?;
            self.security_events_ran_at
                .store(started, Ordering::Relaxed);
        }
        Ok(())
    }

    /// Delete old login attempts in batches until a batch is not full.
    async fn purge_login_attempts(&self) -> Result<(), StorageError> {
        let batch = self.settings.login_attempt_purge_batch;
        loop {
            let deleted = self
                .store
                .purge_login_attempts(self.settings.login_attempt_retention, batch)
                .await?;
            if deleted == 0 || deleted < u64::from(batch) {
                return Ok(());
            }
        }
    }

    /// Delete old security events in batches until a batch is not full.
    async fn purge_security_events(&self) -> Result<(), StorageError> {
        loop {
            let deleted = self.store.purge_security_events().await?;
            if deleted < SECURITY_EVENT_PURGE_BATCH {
                return Ok(());
            }
        }
    }
}

/// Return the whole seconds since the Unix epoch of an instant.
fn epoch_seconds(instant: Timestamp) -> i64 {
    instant.as_offset_date_time().unix_timestamp()
}

/// Return true when a cadence never ran, or when `interval` has passed since `ran_at`.
///
/// A clock that moved back gives false, so the cadence waits for the interval from its last
/// start. The atomic publishes no other memory, so every access uses `Ordering::Relaxed`.
fn is_due(ran_at: &AtomicI64, now: i64, interval: Duration) -> bool {
    let last = ran_at.load(Ordering::Relaxed);
    if last == NEVER {
        return true;
    }
    now.checked_sub(last)
        .and_then(|elapsed| u64::try_from(elapsed).ok())
        .is_some_and(|elapsed| Duration::from_secs(elapsed) >= interval)
}

#[cfg(test)]
mod tests;
