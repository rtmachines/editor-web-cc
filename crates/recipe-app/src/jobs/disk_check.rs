//! Job `disk_check`: read the free disk space and set the read-only mode (design D6).
//!
//! Purpose: stop writes before a full disk can damage the data.
//!
//! Schedule: every `health.disk_check_interval`, and once at start after the database check.
//! The volume `pgdata` sets the mode. The volumes `backups` and `walarchive` give a warning.
//!
//! One run reads the free space of `pgdata` and compares it with three thresholds. Below
//! `disk.readonly_percent` the job turns the mode on. Above `disk.resume_percent` it turns the mode
//! off. Between both it leaves the mode unchanged (reliability.md section 7.8). A failed
//! measurement turns the mode on, and only a measurement that succeeds can turn it off again
//! (ADR-0073 (d)). A reading with more free blocks than total blocks is invalid, so the job treats
//! it as a failed measurement. The job writes its `WARN` lines itself and returns `Ok` after a fault of the
//! disk, because a fault is a handled fault (ADR-0089 (d)). The job `alerts` reads the volumes
//! `backups` and `walarchive`, so this job never reads them.

use std::cmp::Ordering;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering as AtomicOrdering};
use std::time::Duration;

use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;

use super::RunKind;
use crate::error::AppError;
use crate::ports::{Clock, DiskFault, DiskSpace, FreeSpace, ReadOnlySwitch};

/// The name of the only volume that this job reads. The `WARN` lines hold it in the field `volume`.
const VOLUME: &str = "pgdata";
/// Value of a time atomic before its first line. No instant has this value.
const NEVER: i64 = i64::MIN;
/// Percent is a count of hundredths, so the compare multiplies blocks by this factor.
const PERCENT_SCALE: u128 = 100;
/// Default of `disk.warn_percent`.
const DEFAULT_WARN_PERCENT: u8 = 15;
/// Default of `disk.readonly_percent`.
const DEFAULT_READONLY_PERCENT: u8 = 5;
/// Default of `disk.resume_percent`.
const DEFAULT_RESUME_PERCENT: u8 = 10;
/// Default of `disk.warn_log_interval` in seconds: 10 minutes.
const DEFAULT_WARN_LOG_INTERVAL_SECONDS: u64 = 10 * 60;

/// The numeric settings that the job `disk_check` reads.
///
/// The type solves one problem: a job must not read a configuration file. `recipe-server` builds
/// this value and passes it down. `Default` equals the table of `reliability.md` section 7.11, and
/// a test compares both. The loop interval `health.disk_check_interval` belongs to the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskCheckSettings {
    /// The free percent below which the job writes a `WARN` line (`disk.warn_percent`).
    pub warn_percent: u8,
    /// The free percent below which the job turns the mode on (`disk.readonly_percent`).
    pub readonly_percent: u8,
    /// The free percent above which the job turns the mode off (`disk.resume_percent`).
    pub resume_percent: u8,
    /// The time between two `WARN` lines of the same kind (`disk.warn_log_interval`).
    pub warn_log_interval: Duration,
}

impl Default for DiskCheckSettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            warn_percent: DEFAULT_WARN_PERCENT,
            readonly_percent: DEFAULT_READONLY_PERCENT,
            resume_percent: DEFAULT_RESUME_PERCENT,
            warn_log_interval: Duration::from_secs(DEFAULT_WARN_LOG_INTERVAL_SECONDS),
        }
    }
}

/// How one reading of the free space relates to the three thresholds.
#[derive(Clone, Copy, Debug)]
struct Assessment {
    /// The free space is below `disk.warn_percent`.
    below_warn: bool,
    /// The free space is below `disk.readonly_percent`.
    below_readonly: bool,
    /// The free space is above `disk.resume_percent`.
    above_resume: bool,
}

/// The supervised job that sets the read-only mode from the free space of `pgdata`.
///
/// The type solves one problem: no command may write while the free space of `pgdata` is low or
/// unknown, so PostgreSQL never fills the disk unseen (A24). The job keeps its run state as
/// atomics (`reliability.md` section 8, ADR-0060 (aa)): the time of the last low-disk line, the
/// time of the last read-fault line, and the flag of the `Unsupported` line (ADR-0073 (d), (e)).
pub struct DiskCheck {
    /// The port that reads the free space.
    disk: Arc<dyn DiskSpace>,
    /// The switch that the gate of the web crate reads.
    switch: Arc<dyn ReadOnlySwitch>,
    /// The source of the time of a run.
    clock: Arc<dyn Clock>,
    /// The mount point of the volume `pgdata`.
    pgdata: PathBuf,
    /// The thresholds and the interval of the lines.
    settings: DiskCheckSettings,
    /// Epoch seconds of the last low-disk `WARN` line, or [`NEVER`].
    low_warned_at: AtomicI64,
    /// Epoch seconds of the last read-fault `WARN` line, or [`NEVER`].
    fault_warned_at: AtomicI64,
    /// True after the one `WARN` line of the fault `Unsupported`.
    unsupported_warned: AtomicBool,
}

impl DiskCheck {
    /// Build the job. No line has been written yet, so the first matching run writes one.
    ///
    /// `pgdata` is the mount point of the read-only mount of the data volume. The server passes it.
    #[must_use]
    pub fn new(
        disk: Arc<dyn DiskSpace>,
        switch: Arc<dyn ReadOnlySwitch>,
        clock: Arc<dyn Clock>,
        pgdata: PathBuf,
        settings: DiskCheckSettings,
    ) -> Self {
        Self {
            disk,
            switch,
            clock,
            pgdata,
            settings,
            low_warned_at: AtomicI64::new(NEVER),
            fault_warned_at: AtomicI64::new(NEVER),
            unsupported_warned: AtomicBool::new(false),
        }
    }

    /// Measure the free space of `pgdata` once and set the mode.
    ///
    /// The run kind has no effect, because the job has one trigger. The run writes no row with the
    /// request id. A fault of the disk is a handled fault: the run logs it once at `WARN`.
    ///
    /// # Errors
    ///
    /// Never returns `Err` at present. The signature follows the run interface of all jobs
    /// (ADR-0088 (d)).
    pub async fn run_once(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        let now = epoch_seconds(self.clock.now());
        match self.measure().await {
            Ok((space, assessment)) => self.apply(space, assessment, now),
            Err(DiskFault::Unsupported) => self.note_unsupported(),
            Err(fault) => self.note_unreadable(&fault, now),
        }
        Ok(())
    }

    /// Read the free space of `pgdata` and compare it with the three thresholds.
    ///
    /// A file system without blocks has no percent. The job treats it as a failed measurement.
    /// A reading with more free blocks than total blocks is invalid and fails the same way.
    async fn measure(&self) -> Result<(FreeSpace, Assessment), DiskFault> {
        let space = self.disk.free_space(&self.pgdata).await?;
        let usable = space.total_blocks > 0 && space.available_blocks <= space.total_blocks;
        match assess(space, &self.settings) {
            Some(assessment) if usable => Ok((space, assessment)),
            _ => Err(DiskFault::Unreadable(io::Error::new(
                io::ErrorKind::InvalidData,
                "the file system reports no usable block count",
            ))),
        }
    }

    /// Set the mode from a reading that succeeded, then write the low-disk line when it is due.
    ///
    /// Between the hard threshold and the resume threshold the job does not call the switch.
    fn apply(&self, space: FreeSpace, assessment: Assessment, now: i64) {
        if assessment.below_readonly {
            self.switch.set(true);
        } else if assessment.above_resume {
            self.switch.set(false);
        }
        if assessment.below_warn && claim(&self.low_warned_at, now, self.settings.warn_log_interval)
        {
            tracing::warn!(
                volume = VOLUME,
                available_blocks = space.available_blocks,
                total_blocks = space.total_blocks,
                warn_percent = self.settings.warn_percent,
                "The free disk space is below the warning threshold."
            );
        }
    }

    /// Write the one line of the fault `Unsupported`. The mode stays as it is (ADR-0073 (e)).
    fn note_unsupported(&self) {
        if !self.unsupported_warned.swap(true, AtomicOrdering::Relaxed) {
            tracing::warn!(volume = VOLUME, "{}", DiskFault::Unsupported);
        }
    }

    /// Turn the mode on and write the fault line when it is due (ADR-0073 (d)).
    ///
    /// The line holds the `Display` text of the fault. That text names no path and no cause.
    fn note_unreadable(&self, fault: &DiskFault, now: i64) {
        self.switch.set(true);
        if claim(&self.fault_warned_at, now, self.settings.warn_log_interval) {
            tracing::warn!(volume = VOLUME, "{fault}");
        }
    }
}

/// Compare a reading with the three thresholds, or return `None` when the compare overflows.
fn assess(space: FreeSpace, settings: &DiskCheckSettings) -> Option<Assessment> {
    Some(Assessment {
        below_warn: compare(space, settings.warn_percent)?.is_lt(),
        below_readonly: compare(space, settings.readonly_percent)?.is_lt(),
        above_resume: compare(space, settings.resume_percent)?.is_gt(),
    })
}

/// Compare the free space with `percent` of the file system, with integers only.
///
/// The free blocks times 100 are compared with the total blocks times `percent`. This avoids a
/// division and a float. The product uses `u128`, so no real file system overflows it.
fn compare(space: FreeSpace, percent: u8) -> Option<Ordering> {
    let free = u128::from(space.available_blocks).checked_mul(PERCENT_SCALE)?;
    let limit = u128::from(space.total_blocks).checked_mul(u128::from(percent))?;
    Some(free.cmp(&limit))
}

/// Return the whole seconds since the Unix epoch of an instant.
fn epoch_seconds(instant: Timestamp) -> i64 {
    instant.as_offset_date_time().unix_timestamp()
}

/// Take the right to write a line when the line never ran or when `interval` has passed.
///
/// A clock that moved back gives false, so the line waits for the interval from its last time.
/// The atomic publishes no other memory, so every access uses `Ordering::Relaxed`.
fn claim(last: &AtomicI64, now: i64, interval: Duration) -> bool {
    let previous = last.load(AtomicOrdering::Relaxed);
    let due = previous == NEVER
        || now
            .checked_sub(previous)
            .and_then(|elapsed| u64::try_from(elapsed).ok())
            .is_some_and(|elapsed| Duration::from_secs(elapsed) >= interval);
    if due {
        last.store(now, AtomicOrdering::Relaxed);
    }
    due
}

#[cfg(test)]
mod tests;
