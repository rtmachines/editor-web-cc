//! Job `clock_check`: compare the clock of the host with the clock of the database (design D6).
//!
//! Purpose: warn when the clocks of `app` and `db` differ by more than `clock.skew_warn`
//! (`reliability.md` section 7.9).
//!
//! Schedule: once at start, then every `clock.check_interval`. The interval belongs to the loop of
//! the server.
//!
//! A run reads the time of the database with one call of the port, then reads the clock of the
//! host. The difference in both directions counts. A skew above `clock.skew_warn` gives one `WARN`
//! line with the fields `skew` and `limit`. Both fields show a full `Duration`. A skew equal to the limit gives no
//! line. A fault of the read is a storage error. The run returns it and the supervisor logs it once
//! at `ERROR` (ADR-0060 (q), ADR-0089 (d)).

use std::sync::Arc;
use std::time::Duration;

use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;

use super::RunKind;
use crate::error::AppError;
use crate::ports::{Clock, OperationsReads};

/// Default of `clock.skew_warn` in seconds.
const DEFAULT_SKEW_WARN_SECONDS: u64 = 5;

/// The numeric settings that the job `clock_check` reads.
///
/// The type solves one problem: a job must not read a configuration file. `recipe-server` builds
/// this value and passes it down. `Default` equals the table of `reliability.md` section 7.11, and
/// a test compares both. The loop interval `clock.check_interval` belongs to the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockCheckSettings {
    /// The skew above which the job writes a `WARN` line (`clock.skew_warn`).
    pub skew_warn: Duration,
}

impl Default for ClockCheckSettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            skew_warn: Duration::from_secs(DEFAULT_SKEW_WARN_SECONDS),
        }
    }
}

/// The supervised job that compares the clock of the host with the clock of the database.
///
/// The type solves one problem: the audit log, the sessions, and the retention rules all use
/// instants. A skew between `app` and `db` makes them disagree, so the operator must see it
/// before it harms them (`reliability.md` section 7.9).
pub struct ClockCheck {
    /// The port that reads the time of the database.
    reads: Arc<dyn OperationsReads>,
    /// The clock of the host.
    clock: Arc<dyn Clock>,
    /// The limit of the skew.
    settings: ClockCheckSettings,
}

impl ClockCheck {
    /// Build the job.
    #[must_use]
    pub fn new(
        reads: Arc<dyn OperationsReads>,
        clock: Arc<dyn Clock>,
        settings: ClockCheckSettings,
    ) -> Self {
        Self {
            reads,
            clock,
            settings,
        }
    }

    /// Compare both clocks once and write a `WARN` line when the skew is above the limit.
    ///
    /// The run kind has no effect, because the job has one trigger (ADR-0088 (b)). The run writes
    /// no row with the request id.
    ///
    /// # Errors
    ///
    /// Returns the `StorageError` of the read as `AppError::Storage`. The run then compares nothing.
    pub async fn run_once(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        let database = self.reads.database_now().await?;
        let host = self.clock.now();
        let skew = absolute_skew(host, database);
        if skew > self.settings.skew_warn {
            tracing::warn!(
                ?skew,
                limit = ?self.settings.skew_warn,
                "The clocks of the host and the database differ by more than the limit."
            );
        }
        Ok(())
    }
}

/// Return the absolute difference of two instants.
///
/// The difference holds in both directions. The result keeps the full precision, so no
/// rounding remains. The code orders the instants, builds the later one as a `Duration` from the
/// earlier whole second, and takes `Duration::abs_diff` of the earlier fraction. This path has no
/// arithmetic operator, no failed conversion, and no panic.
fn absolute_skew(host: Timestamp, database: Timestamp) -> Duration {
    let host = whole_and_fraction(host);
    let database = whole_and_fraction(database);
    let (later, earlier) = if host >= database {
        (host, database)
    } else {
        (database, host)
    };
    let from_earlier_second = Duration::new(later.0.abs_diff(earlier.0), later.1);
    from_earlier_second.abs_diff(Duration::from_nanos(u64::from(earlier.1)))
}

/// Return the Unix seconds and the nanoseconds of the last second of an instant.
///
/// The pair orders like the instant does.
fn whole_and_fraction(instant: Timestamp) -> (i64, u32) {
    let instant = instant.as_offset_date_time();
    (instant.unix_timestamp(), instant.nanosecond())
}

#[cfg(test)]
mod tests;
