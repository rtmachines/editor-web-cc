//! Job `alerts`: test the ten alert conditions and call the webhook (design D6).
//!
//! Purpose: tell the operator about a fault that needs action.
//!
//! Schedule: every `alerts.interval`. One database snapshot serves each run.
//! The webhook calls of one run go out at the same time under `alerts.webhook_timeout`.
//!
//! One run reads the snapshot first. A failed read ends the run with the error, and the run
//! writes no line and sends no call (ADR-0089 (d)). Then the run reads the backup directory and
//! the three volumes, and it tests the conditions in the order of [`AlertCondition::ALL`]. Each
//! active condition writes one `WARN` line with the field `alert` (ADR-0075 (c)). The run sends
//! all active conditions in one call of the webhook port. With no active condition it makes no
//! call.
//!
//! The job takes every time from `snapshot.database_now`, because all containers use the host
//! clock (`reliability.md` section 7.9). A stored time later than that time by at most
//! `alerts.interval` has the age zero. A stored time later by more than `alerts.interval` is
//! untrusted, and its condition stays active (ADR-0093 (a)).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;

use super::RunKind;
use crate::error::AppError;
use crate::operations::{AlertCounters, ReadinessState};
use crate::ports::{
    AlertCondition, AlertWebhook, BackupDirectory, DiskFault, DiskSpace, FreeSpace,
    OperationsReads, OperationsSnapshot, SnapshotWindows,
};

/// Value of a slot before its first panic. No instant has this value.
const NEVER: i64 = i64::MIN;
/// Percent is a count of hundredths, so the compare multiplies blocks by this factor.
const PERCENT_SCALE: u128 = 100;
/// The message of the `WARN` line of an active condition.
const ACTIVE_MESSAGE: &str = "An alert condition is active.";
/// Default of `alerts.interval` in seconds.
const DEFAULT_INTERVAL_SECONDS: u64 = 60;
/// Default of `alerts.panic_count`.
const DEFAULT_PANIC_COUNT: u16 = 3;
/// Default of `alerts.panic_window` in seconds: 10 minutes.
const DEFAULT_PANIC_WINDOW_SECONDS: u64 = 10 * 60;
/// Default of `alerts.not_ready_for` in seconds.
const DEFAULT_NOT_READY_SECONDS: u64 = 60;
/// Default of `alerts.failed_login_count`.
const DEFAULT_FAILED_LOGIN_COUNT: u64 = 50;
/// Default of `alerts.failed_login_window` and `alerts.security_window` in seconds: 15 minutes.
const DEFAULT_SECURITY_WINDOW_SECONDS: u64 = 15 * 60;
/// Default of `backup.max_age_alert` and `audit.max_check_age` in seconds: 26 hours.
const DEFAULT_MAX_AGE_SECONDS: u64 = 26 * 60 * 60;
/// Default of `wal.max_lag_bytes`: 16 MiB.
const DEFAULT_WAL_MAX_LAG_BYTES: u64 = 16 * 1024 * 1024;
/// Default of `disk.warn_percent`.
const DEFAULT_DISK_WARN_PERCENT: u8 = 15;

/// The numeric settings that the job `alerts` reads.
///
/// The type solves one problem: a job must not read a configuration file. `recipe-server` builds
/// this value and passes it down. `Default` equals the table of `reliability.md` section 7.11, and
/// a test compares both. The setting `alerts.webhook_timeout` belongs to the webhook adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlertSettings {
    /// The time between two runs (`alerts.interval`).
    /// Also the largest skew of a stored time that the job trusts (ADR-0093 (a)).
    pub interval: Duration,
    /// A run raises `panics` above this count (`alerts.panic_count`). The job keeps one more slot.
    /// The type `u16` limits the job to 65536 slots.
    pub panic_count: u16,
    /// The time in which the panics count (`alerts.panic_window`).
    pub panic_window: Duration,
    /// The time of not ready above which `not_ready` is active (`alerts.not_ready_for`).
    pub not_ready_for: Duration,
    /// A run raises `failed_logins` above this count (`alerts.failed_login_count`).
    pub failed_login_count: u64,
    /// The window of the failed logins (`alerts.failed_login_window`).
    pub failed_login_window: Duration,
    /// The window of `security_events` (`alerts.security_window`).
    pub security_window: Duration,
    /// The age of the newest dump above which `backup_age` is active (`backup.max_age_alert`).
    pub backup_max_age_alert: Duration,
    /// The age of the audit check above which `audit_check_age` is active (`audit.max_check_age`).
    pub audit_max_check_age: Duration,
    /// The lag at or above which `replication` is active (`wal.max_lag_bytes`).
    pub wal_max_lag_bytes: u64,
    /// The free percent below which `low_disk` is active (`disk.warn_percent`).
    pub disk_warn_percent: u8,
}

impl Default for AlertSettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(DEFAULT_INTERVAL_SECONDS),
            panic_count: DEFAULT_PANIC_COUNT,
            panic_window: Duration::from_secs(DEFAULT_PANIC_WINDOW_SECONDS),
            not_ready_for: Duration::from_secs(DEFAULT_NOT_READY_SECONDS),
            failed_login_count: DEFAULT_FAILED_LOGIN_COUNT,
            failed_login_window: Duration::from_secs(DEFAULT_SECURITY_WINDOW_SECONDS),
            security_window: Duration::from_secs(DEFAULT_SECURITY_WINDOW_SECONDS),
            backup_max_age_alert: Duration::from_secs(DEFAULT_MAX_AGE_SECONDS),
            audit_max_check_age: Duration::from_secs(DEFAULT_MAX_AGE_SECONDS),
            wal_max_lag_bytes: DEFAULT_WAL_MAX_LAG_BYTES,
            disk_warn_percent: DEFAULT_DISK_WARN_PERCENT,
        }
    }
}

/// The ports that the job calls.
///
/// The type solves one problem: the constructor of the job would need too many arguments
/// without a group. The four ports have one owner, the server wiring.
pub struct AlertPorts {
    /// The port that reads the database snapshot of one run.
    pub operations: Arc<dyn OperationsReads>,
    /// The port that sends the alerts to the webhook.
    pub webhook: Arc<dyn AlertWebhook>,
    /// The port that reads the free space of a volume.
    pub disk: Arc<dyn DiskSpace>,
    /// The port that reads the newest complete backup set.
    pub backups: Arc<dyn BackupDirectory>,
}

/// The in-process sources of the job.
///
/// The type solves one problem: two conditions have no database source (3.7). The panic hook and
/// `ExportVersion` write the counters, and the readiness check writes the state.
pub struct AlertSources {
    /// The counters of panics and export hash mismatches.
    pub counters: AlertCounters,
    /// The state of `/readyz`.
    pub readiness: ReadinessState,
}

/// The mount points of the three volumes that the job reads.
///
/// The type solves one problem: the three paths would be three bare `PathBuf` arguments that
/// a caller can swap by mistake.
pub struct AlertVolumes {
    /// The mount point of the volume `pgdata`.
    pub pgdata: PathBuf,
    /// The mount point of the volume `backups`.
    pub backups: PathBuf,
    /// The mount point of the volume `walarchive`.
    pub walarchive: PathBuf,
}

impl AlertVolumes {
    /// Return the name and the mount point of each volume, in the order of the reads.
    fn named(&self) -> [(&'static str, &Path); 3] {
        [
            ("pgdata", self.pgdata.as_path()),
            ("backups", self.backups.as_path()),
            ("walarchive", self.walarchive.as_path()),
        ]
    }
}

/// The counters of the two in-process sources, read once at the start of a run.
#[derive(Clone, Copy, Debug)]
struct Totals {
    /// The number of panics since the start.
    panics: u64,
    /// The number of export hash mismatches since the start.
    export_hash_mismatches: u64,
}

/// The time of one run, with the largest skew that the job trusts.
///
/// The type solves one problem: a clock that goes back must not hide a condition for ever. An age
/// is `None` when the stored time is later than now by more than the skew (ADR-0093 (a)). Every
/// age compare uses nanoseconds, so a border of 26 hours does not move by a fraction of a second.
#[derive(Clone, Copy, Debug)]
struct Moment {
    /// The database time of the run in whole Unix seconds.
    now: i64,
    /// The database time of the run in Unix nanoseconds.
    now_nanos: i128,
    /// The skew in nanoseconds that the job trusts.
    skew_nanos: i128,
}

impl Moment {
    /// Build the moment from the snapshot of the run.
    fn new(now: Timestamp, interval: Duration) -> Self {
        Self {
            now: epoch_seconds(now),
            now_nanos: epoch_nanos(now),
            skew_nanos: duration_nanos(interval),
        }
    }

    /// Return the age of `then` in nanoseconds.
    ///
    /// A time later than now by at most the skew has the age zero. A later time gives `None`.
    /// The border is inclusive (ADR-0093 (a)).
    fn age(self, then: Timestamp) -> Option<i128> {
        let elapsed = self.now_nanos.checked_sub(epoch_nanos(then))?;
        if elapsed >= 0 {
            return Some(elapsed);
        }
        (elapsed.checked_neg()? <= self.skew_nanos).then_some(0)
    }

    /// Return true when `then` is older than `window`. An untrusted time counts as older.
    fn older_than(self, then: Timestamp, window: Duration) -> bool {
        self.age(then)
            .is_none_or(|age| age > duration_nanos(window))
    }

    /// Return true when `then` is younger than `window`. An untrusted time counts as younger.
    fn younger_than(self, then: Timestamp, window: Duration) -> bool {
        self.age(then)
            .is_none_or(|age| age < duration_nanos(window))
    }

    /// Return true when the first start was less than `window` ago.
    ///
    /// An unknown or untrusted first start gives no grace (ADR-0060 (x), (y)).
    fn in_grace(self, first_start: Option<Timestamp>, window: Duration) -> bool {
        first_start
            .and_then(|first| self.age(first))
            .is_some_and(|age| age < duration_nanos(window))
    }
}

/// The values that live between two runs.
///
/// The fields hold the last totals of the two counters, the times of the last panics, and the slot
/// that the next panic takes. Every access uses `Ordering::Relaxed`, because no atomic publishes
/// other memory and one run at a time uses them (ADR-0060 (h), ADR-0073 (g)).
struct RunState {
    /// The panic total that the slots hold. The run stores it right after it writes the slots.
    panics: AtomicU64,
    /// The mismatch total of the last finished run.
    export_hash_mismatches: AtomicU64,
    /// The times of the last `alerts.panic_count + 1` panics, or [`NEVER`].
    panic_times: Vec<AtomicI64>,
    /// The index of the slot that the next panic takes. It holds the oldest panic.
    next_slot: AtomicUsize,
}

/// The supervised job that tests the alert conditions and calls the webhook.
///
/// The type solves one problem: the operator must learn about a fault that needs action without
/// a person who watches the logs (`reliability.md` section 3.7). It keeps its run state as atomics
/// (`reliability.md` section 8, ADR-0060 (aa)): the last totals of the two counters and the slots
/// of the panic times (ADR-0073 (g)).
pub struct Alerts {
    /// The ports that the job calls.
    ports: AlertPorts,
    /// The in-process sources.
    sources: AlertSources,
    /// The mount points of the three volumes.
    volumes: AlertVolumes,
    /// The thresholds and windows.
    settings: AlertSettings,
    /// The values between two runs.
    state: RunState,
}

impl Alerts {
    /// Build the job. No panic has been seen yet, so the first run counts all panics so far.
    #[must_use]
    pub fn new(
        ports: AlertPorts,
        sources: AlertSources,
        volumes: AlertVolumes,
        settings: AlertSettings,
    ) -> Self {
        let state = RunState {
            panics: AtomicU64::new(0),
            export_hash_mismatches: AtomicU64::new(0),
            panic_times: (0..=settings.panic_count)
                .map(|_| AtomicI64::new(NEVER))
                .collect(),
            next_slot: AtomicUsize::new(0),
        };
        Self {
            ports,
            sources,
            volumes,
            settings,
            state,
        }
    }

    /// Test the ten conditions once and send the active ones to the webhook.
    ///
    /// The run kind has no effect, because the job has one trigger. The run writes no row with the
    /// request id. A fault of the webhook, the disk, or the backup directory is a handled fault:
    /// the run logs it once at `WARN` and continues.
    ///
    /// # Errors
    ///
    /// Returns the error of the snapshot read. The run then evaluates no condition, writes no
    /// line, and sends no call. The supervisor logs the error (ADR-0089 (d)).
    pub async fn run_once(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        let snapshot = self.ports.operations.snapshot(&self.windows()).await?;
        let moment = Moment::new(snapshot.database_now, self.settings.interval);
        let totals = Totals {
            panics: self.sources.counters.panics(),
            export_hash_mismatches: self.sources.counters.export_hash_mismatches(),
        };
        let mut active = Vec::new();
        for condition in AlertCondition::ALL {
            if self.test(condition, &snapshot, moment, totals).await {
                active.push(condition);
            }
        }
        self.send(&active).await;
        self.store_totals(totals);
        Ok(())
    }

    /// Return the two windows of the snapshot.
    fn windows(&self) -> SnapshotWindows {
        SnapshotWindows {
            failed_logins: self.settings.failed_login_window,
            security: self.settings.security_window,
        }
    }

    /// Test one condition and write its lines. Return true when it is active.
    async fn test(
        &self,
        condition: AlertCondition,
        snapshot: &OperationsSnapshot,
        moment: Moment,
        totals: Totals,
    ) -> bool {
        let active = match condition {
            AlertCondition::LowDisk => return self.low_disk().await,
            AlertCondition::BackupAge => self.backup_age(snapshot, moment).await,
            _ => self.test_snapshot(condition, snapshot, moment, totals),
        };
        if active {
            tracing::warn!(alert = condition.name(), "{ACTIVE_MESSAGE}");
        }
        active
    }

    /// Test a condition that needs no read beyond the snapshot and the in-process sources.
    fn test_snapshot(
        &self,
        condition: AlertCondition,
        snapshot: &OperationsSnapshot,
        moment: Moment,
        totals: Totals,
    ) -> bool {
        let settings = &self.settings;
        match condition {
            AlertCondition::Panics => self.panics(moment, totals.panics),
            AlertCondition::NotReady => self
                .sources
                .readiness
                .not_ready_for(snapshot.database_now)
                .is_some_and(|elapsed| elapsed > settings.not_ready_for),
            AlertCondition::Replication => {
                let state = snapshot.replication;
                !state.connected
                    || state
                        .lag_bytes
                        .is_none_or(|lag| lag >= settings.wal_max_lag_bytes)
            }
            AlertCondition::FailedLogins => snapshot.failed_logins > settings.failed_login_count,
            AlertCondition::SecurityEvents => snapshot.security_alerts > 0,
            AlertCondition::AuditChainBroken => snapshot
                .audit_chain_broken_at
                .is_some_and(|broken| moment.younger_than(broken, settings.audit_max_check_age)),
            AlertCondition::AuditCheckAge => match snapshot.audit_last_check_at {
                Some(checked) => moment.older_than(checked, settings.audit_max_check_age),
                None => !moment.in_grace(snapshot.first_setting_at, settings.audit_max_check_age),
            },
            AlertCondition::ExportHashMismatch => {
                new_since(
                    totals.export_hash_mismatches,
                    &self.state.export_hash_mismatches,
                ) > 0
            }
            AlertCondition::BackupAge | AlertCondition::LowDisk => false,
        }
    }

    /// Record the new panics and return true when more than `alerts.panic_count` are in the window.
    ///
    /// The slots hold the times of the last `alerts.panic_count + 1` panics. The slot at the
    /// cursor holds the oldest of them. When that panic is in the window, the count is above the
    /// setting. A slot later than now (a clock that went back) takes the value of now.
    ///
    /// The function stores the panic total after the slots and has no await. A dropped run
    /// therefore never writes a panic into the slots twice and never loses one.
    fn panics(&self, moment: Moment, panic_total: u64) -> bool {
        let slots = &self.state.panic_times;
        for slot in slots {
            slot.fetch_min(moment.now, Ordering::Relaxed);
        }
        let fresh = new_since(panic_total, &self.state.panics);
        self.record_panics(moment.now, fresh);
        // The total follows the slots with no await between them. A run that the caller drops
        // later has then counted each panic once, and the next run takes only the panics above this
        // total.
        self.state.panics.store(panic_total, Ordering::Relaxed);
        let cursor = self.state.next_slot.load(Ordering::Relaxed);
        let oldest = slots.get(cursor).map(|slot| slot.load(Ordering::Relaxed));
        oldest
            .filter(|time| *time != NEVER)
            .and_then(|time| moment.now.checked_sub(time))
            .is_some_and(|age| age <= seconds(self.settings.panic_window))
    }

    /// Write `now` into the slots of the oldest panics. A run with more panics than slots
    /// fills all slots.
    fn record_panics(&self, now: i64, fresh: u64) {
        let slots = &self.state.panic_times;
        let count = usize::try_from(fresh)
            .unwrap_or(usize::MAX)
            .min(slots.len());
        let mut cursor = self.state.next_slot.load(Ordering::Relaxed);
        for _ in 0..count {
            if let Some(slot) = slots.get(cursor) {
                slot.store(now, Ordering::Relaxed);
            }
            cursor = cursor
                .checked_add(1)
                .and_then(|next| next.checked_rem(slots.len()))
                .unwrap_or(0);
        }
        self.state.next_slot.store(cursor, Ordering::Relaxed);
    }

    /// Read the newest backup set and return true when `backup_age` is active.
    ///
    /// A read fault makes the condition active also during the start grace (ADR-0073 (c)).
    async fn backup_age(&self, snapshot: &OperationsSnapshot, moment: Moment) -> bool {
        let window = self.settings.backup_max_age_alert;
        match self.ports.backups.newest_complete_set().await {
            Err(fault) => {
                tracing::warn!("{fault}");
                true
            }
            Ok(Some(newest)) => moment.older_than(newest, window),
            Ok(None) => !moment.in_grace(snapshot.first_setting_at, window),
        }
    }

    /// Read the three volumes and return true when `low_disk` is active for one of them.
    ///
    /// Each low or unreadable volume writes its own line with the field `volume` (ADR-0075 (e)).
    async fn low_disk(&self) -> bool {
        let mut any = false;
        for (name, path) in self.volumes.named() {
            if self.volume_is_low(name, path).await {
                tracing::warn!(
                    alert = AlertCondition::LowDisk.name(),
                    volume = name,
                    "{ACTIVE_MESSAGE}"
                );
                any = true;
            }
        }
        any
    }

    /// Read one volume. Return true when it is low or unreadable and write the line of a fault.
    ///
    /// `DiskFault::Unsupported` keeps the volume inactive and writes no line (ADR-0073 (e), (f)).
    async fn volume_is_low(&self, name: &str, path: &Path) -> bool {
        let fault = match self.ports.disk.free_space(path).await {
            Ok(space) => match is_below(space, self.settings.disk_warn_percent) {
                Some(low) => return low,
                None => DiskFault::Unreadable(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "the file system reports no usable block count",
                )),
            },
            Err(DiskFault::Unsupported) => return false,
            Err(fault) => fault,
        };
        tracing::warn!(volume = name, "{fault}");
        true
    }

    /// Send the active conditions in one call. Write one `WARN` line for each failed send.
    async fn send(&self, active: &[AlertCondition]) {
        if active.is_empty() {
            return;
        }
        let results = self.ports.webhook.send_all(active).await;
        for (condition, result) in active.iter().zip(results) {
            if let Err(fault) = result {
                tracing::warn!(condition = condition.name(), "{fault}");
            }
        }
    }

    /// Keep the mismatch total of this run. The call comes last, so a run that stops early
    /// counts again.
    ///
    /// The panic total is not here. [`Alerts::panics`] stores it with the slots, because a panic
    /// must reach the slots once (ADR-0073 (g)).
    fn store_totals(&self, totals: Totals) {
        self.state
            .export_hash_mismatches
            .store(totals.export_hash_mismatches, Ordering::Relaxed);
    }
}

/// Return the growth of a counter since the last finished run. A counter that went back gives zero.
fn new_since(total: u64, last: &AtomicU64) -> u64 {
    let last = last.load(Ordering::Relaxed);
    if total < last {
        return 0;
    }
    total.abs_diff(last)
}

/// Return true when the free space is below `percent` of the file system, with integers only.
///
/// The free blocks times 100 are compared with the total blocks times `percent`. A file system
/// without blocks, and a reading with more free than total blocks, give `None` (invalid).
fn is_below(space: FreeSpace, percent: u8) -> Option<bool> {
    if space.total_blocks == 0 || space.available_blocks > space.total_blocks {
        return None;
    }
    let free = u128::from(space.available_blocks).checked_mul(PERCENT_SCALE)?;
    let limit = u128::from(space.total_blocks).checked_mul(u128::from(percent))?;
    Some(free < limit)
}

/// Return the whole seconds since the Unix epoch of an instant.
fn epoch_seconds(instant: Timestamp) -> i64 {
    instant.as_offset_date_time().unix_timestamp()
}

/// Return the nanoseconds since the Unix epoch of an instant.
fn epoch_nanos(instant: Timestamp) -> i128 {
    instant.as_offset_date_time().unix_timestamp_nanos()
}

/// Return the nanoseconds of a duration. A duration beyond `i128` gives the largest value.
fn duration_nanos(duration: Duration) -> i128 {
    i128::try_from(duration.as_nanos()).unwrap_or(i128::MAX)
}

/// Return the whole seconds of a duration. A duration beyond `i64` gives the largest value.
fn seconds(duration: Duration) -> i64 {
    i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests;
