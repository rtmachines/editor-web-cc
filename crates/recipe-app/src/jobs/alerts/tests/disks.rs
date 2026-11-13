//! Tests of the condition `low_disk` (ADR-0073 (e), (f), ADR-0075 (e), `reliability.md` 3.7).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use tracing::Level;

use super::support::{
    LogCapture, Rig, TOTAL, UNREADABLE_TEXT, backups_path, pgdata, rig, rig_with_disk, run,
    sent_by_one_run, space, volumes, walarchive,
};
use crate::memory::{DiskFailure, DiskReading};
use crate::ports::{AlertCondition, DiskFault, DiskSpace, FreeSpace};

/// Free blocks below the warning threshold: 14.9 %.
const LOW: u64 = 149;
/// Free blocks exactly at the warning threshold: 15 %.
const AT_WARNING: u64 = 150;

#[tokio::test]
async fn the_job_reads_the_three_volumes_once_in_each_run() {
    let rig = rig();
    run(&rig).await.unwrap();
    let paths: Vec<_> = volumes().into_iter().map(|(_, path)| path).collect();
    assert_eq!(rig.disk.queries(), paths);
}

/// The number of yields that hold one call of [`PeakDisk`] in flight.
const HOLD_YIELDS: usize = 3;

/// A disk double that counts the calls of `free_space` in flight for each path.
///
/// It solves one problem: a test must show that one run never reads one path twice at once.
#[derive(Default)]
struct PeakDisk {
    /// The calls that have started and not ended, for each path.
    in_flight: Mutex<HashMap<PathBuf, usize>>,
    /// The highest number of calls in flight, for each path.
    peaks: Mutex<HashMap<PathBuf, usize>>,
}

impl PeakDisk {
    /// Record the start of a call for `path`.
    fn enter(&self, path: &Path) {
        let mut in_flight = self.in_flight.lock();
        let now = in_flight.entry(path.to_path_buf()).or_default();
        *now = now.saturating_add(1);
        let peak = *now;
        let mut peaks = self.peaks.lock();
        let highest = peaks.entry(path.to_path_buf()).or_default();
        *highest = (*highest).max(peak);
    }

    /// Record the end of a call for `path`.
    fn leave(&self, path: &Path) {
        if let Some(now) = self.in_flight.lock().get_mut(path) {
            *now = now.saturating_sub(1);
        }
    }

    /// Return the peak for `path`, or 0 when the job never read it.
    fn peak(&self, path: &Path) -> usize {
        self.peaks.lock().get(path).copied().unwrap_or(0)
    }
}

#[async_trait]
impl DiskSpace for PeakDisk {
    async fn free_space(&self, path: &Path) -> Result<FreeSpace, DiskFault> {
        self.enter(path);
        for _ in 0..HOLD_YIELDS {
            tokio::task::yield_now().await;
        }
        self.leave(path);
        Ok(FreeSpace {
            available_blocks: TOTAL / 2,
            total_blocks: TOTAL,
        })
    }
}

#[tokio::test]
async fn one_run_reads_each_volume_at_most_once_at_a_time() {
    // ADR-0161: one run of `alerts` reads one path at most once at a time.
    // The test does not forbid a read of two different paths at the same time.
    let disk = Arc::new(PeakDisk::default());
    let rig = rig_with_disk(Arc::clone(&disk) as Arc<dyn DiskSpace>);
    run(&rig).await.unwrap();
    for (_, path) in volumes() {
        assert_eq!(disk.peak(&path), 1, "path {}", path.display());
    }
}

#[tokio::test]
async fn free_space_at_the_warning_threshold_is_not_low() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.disk.set(&pgdata(), space(AT_WARNING));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 0);
}

#[tokio::test]
async fn one_low_volume_writes_one_line_with_the_field_volume_and_sends_one_call() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.disk.set(&backups_path(), space(LOW));
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::LowDisk]);
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, Level::WARN);
    assert_eq!(events[0].field("alert"), Some("\"low_disk\""));
    assert_eq!(events[0].field("volume"), Some("\"backups\""));
    assert_eq!(rig.webhook.calls().len(), 1);
}

#[tokio::test]
async fn three_low_volumes_write_three_lines_and_send_one_call() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    for (_, path) in volumes() {
        rig.disk.set(&path, space(LOW));
    }
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::LowDisk]);
    let lines = logs.alert_lines();
    let volumes: Vec<_> = lines
        .iter()
        .filter_map(|line| line.field("volume"))
        .collect();
    assert_eq!(volumes, ["\"pgdata\"", "\"backups\"", "\"walarchive\""]);
    assert_eq!(logs.alert_names(), ["low_disk"; 3]);
    assert_eq!(rig.webhook.calls(), vec![vec![AlertCondition::LowDisk]]);
}

#[tokio::test]
async fn an_unsupported_volume_keeps_the_condition_inactive_and_writes_no_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    for (_, path) in volumes() {
        rig.disk
            .set(&path, DiskReading::Failure(DiskFailure::Unsupported));
    }
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 0);
    assert!(rig.webhook.calls().is_empty());
}

#[tokio::test]
async fn a_read_fault_of_a_mount_makes_the_condition_active_for_that_volume() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.disk
        .set(&walarchive(), DiskReading::Failure(DiskFailure::Unreadable));
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::LowDisk]);
    let events = logs.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].message, UNREADABLE_TEXT);
    assert_eq!(events[0].level, Level::WARN);
    assert_eq!(events[0].field("volume"), Some("\"walarchive\""));
    assert!(events[0].field("alert").is_none());
    assert_eq!(events[1].field("alert"), Some("\"low_disk\""));
    assert_eq!(events[1].field("volume"), Some("\"walarchive\""));
}

#[tokio::test]
async fn the_fault_line_repeats_in_each_run_that_gets_the_fault() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.disk
        .set(&pgdata(), DiskReading::Failure(DiskFailure::Unreadable));
    run(&rig).await.unwrap();
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 4);
    assert_eq!(rig.webhook.calls().len(), 2);
}

#[tokio::test]
async fn a_fault_and_a_low_volume_give_two_condition_lines_and_one_call() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.disk
        .set(&pgdata(), DiskReading::Failure(DiskFailure::Unreadable));
    rig.disk.set(&backups_path(), space(LOW));
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::LowDisk]);
    assert_eq!(logs.alert_names(), ["low_disk", "low_disk"]);
    assert_eq!(logs.count(), 3);
}

#[tokio::test]
async fn a_reading_with_more_free_than_total_blocks_counts_as_a_read_fault() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    let invalid = FreeSpace {
        available_blocks: TOTAL + 1,
        total_blocks: TOTAL,
    };
    rig.disk.set(&pgdata(), DiskReading::Space(invalid));
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::LowDisk]);
    assert_eq!(logs.events()[0].message, UNREADABLE_TEXT);
}

#[tokio::test]
async fn a_file_system_without_blocks_counts_as_a_read_fault() {
    let rig = rig();
    let empty = FreeSpace {
        available_blocks: 0,
        total_blocks: 0,
    };
    rig.disk.set(&walarchive(), DiskReading::Space(empty));
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::LowDisk]);
}

#[tokio::test]
async fn no_line_of_a_disk_fault_names_a_path() {
    let (logs, _guard) = LogCapture::install();
    let rig: Rig = rig();
    rig.disk
        .set(&pgdata(), DiskReading::Failure(DiskFailure::Unreadable));
    rig.disk.set(&backups_path(), space(LOW));
    run(&rig).await.unwrap();
    for event in logs.events() {
        let text = format!("{} {:?}", event.message, event.fields);
        assert!(!text.contains("/srv"), "{text}");
    }
}
