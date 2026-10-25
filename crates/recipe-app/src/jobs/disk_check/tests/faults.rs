//! Tests of the read faults of the job `disk_check` (ADR-0073 (d) and (e), reliability.md 7.8).

use tracing::Level;

use super::support::{
    DAY, HOUR, LogCapture, MINUTE, TEN_MINUTES, TOTAL, UNREADABLE_TEXT, UNSUPPORTED_TEXT, after,
    rig, rig_with, run, set_available, set_failure, set_space, set_time, times,
};
use crate::jobs::disk_check::DiskCheckSettings;
use crate::memory::DiskFailure;
use crate::ports::ReadOnlySwitch;

/// Free blocks that are below the warning threshold and above the hard threshold: 14 %.
const LOW: u64 = 140;

#[tokio::test]
async fn an_unreadable_volume_turns_the_mode_on_and_the_run_succeeds() {
    let rig = rig();
    set_failure(&rig, DiskFailure::Unreadable);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
}

#[tokio::test]
async fn an_unreadable_volume_writes_one_warn_line_with_the_text_of_the_fault() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_failure(&rig, DiskFailure::Unreadable);
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, Level::WARN);
    assert_eq!(events[0].message, UNREADABLE_TEXT);
    assert_eq!(events[0].field("volume"), Some("\"pgdata\""));
    assert!(!format!("{events:?}").contains("injected"));
}

#[tokio::test]
async fn the_fault_line_repeats_every_log_interval() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_failure(&rig, DiskFailure::Unreadable);
    run(&rig).await.unwrap();
    set_time(&rig, MINUTE);
    run(&rig).await.unwrap();
    set_time(&rig, TEN_MINUTES - 1);
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 1);
    set_time(&rig, TEN_MINUTES);
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 2);
    assert!(rig.switch.is_on());
}

#[tokio::test]
async fn the_mode_stays_on_while_the_volume_stays_unreadable() {
    let rig = rig();
    set_failure(&rig, DiskFailure::Unreadable);
    for step in 0..3 {
        set_time(&rig, times(step, MINUTE));
        run(&rig).await.unwrap();
        assert!(rig.switch.is_on());
    }
}

#[tokio::test]
async fn a_fault_does_not_end_the_mode() {
    let rig = rig_with(DiskCheckSettings::default(), true);
    set_failure(&rig, DiskFailure::Unreadable);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
}

#[tokio::test]
async fn a_measurement_between_the_thresholds_does_not_end_the_mode_after_a_fault() {
    let rig = rig();
    set_failure(&rig, DiskFailure::Unreadable);
    run(&rig).await.unwrap();
    set_available(&rig, 80);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
}

#[tokio::test]
async fn a_measurement_above_the_resume_threshold_ends_the_mode_after_a_fault() {
    let rig = rig();
    set_failure(&rig, DiskFailure::Unreadable);
    run(&rig).await.unwrap();
    set_available(&rig, 500);
    run(&rig).await.unwrap();
    assert!(!rig.switch.is_on());
}

#[tokio::test]
async fn a_fault_on_one_call_only_is_followed_by_a_normal_run() {
    let rig = rig();
    rig.disk.fail_on_call(1, DiskFailure::Unreadable);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
    run(&rig).await.unwrap();
    assert!(!rig.switch.is_on());
}

#[tokio::test]
async fn the_fault_line_has_its_own_time_after_a_low_disk_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, LOW);
    run(&rig).await.unwrap();
    set_failure(&rig, DiskFailure::Unreadable);
    set_time(&rig, MINUTE);
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].message, UNREADABLE_TEXT);
}

#[tokio::test]
async fn the_low_disk_line_has_its_own_time_after_a_fault_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_failure(&rig, DiskFailure::Unreadable);
    run(&rig).await.unwrap();
    set_available(&rig, LOW);
    set_time(&rig, MINUTE);
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 2);
}

#[tokio::test]
async fn a_clock_that_moved_back_keeps_the_fault_line_waiting_and_it_returns_later() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_failure(&rig, DiskFailure::Unreadable);
    set_time(&rig, 5 * HOUR);
    run(&rig).await.unwrap();
    set_time(&rig, HOUR);
    run(&rig).await.unwrap();
    set_time(&rig, after(HOUR, TEN_MINUTES));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 1);
    set_time(&rig, after(5 * HOUR, TEN_MINUTES));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 2);
}

#[tokio::test]
async fn more_free_blocks_than_total_blocks_is_an_unreadable_reading() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_space(&rig, TOTAL.checked_add(1).unwrap(), TOTAL);
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, Level::WARN);
    assert_eq!(events[0].message, UNREADABLE_TEXT);
    assert_eq!(events[0].field("volume"), Some("\"pgdata\""));
    assert!(rig.switch.is_on());
}

#[tokio::test]
async fn more_free_blocks_than_total_blocks_does_not_end_the_mode() {
    let rig = rig_with(DiskCheckSettings::default(), true);
    set_space(&rig, TOTAL.checked_add(1).unwrap(), TOTAL);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
}

#[tokio::test]
async fn as_many_free_blocks_as_total_blocks_is_a_valid_reading() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig_with(DiskCheckSettings::default(), true);
    set_space(&rig, TOTAL, TOTAL);
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 0);
    assert!(!rig.switch.is_on());
}

#[tokio::test]
async fn an_unsupported_system_writes_one_warn_line_per_process() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_failure(&rig, DiskFailure::Unsupported);
    for step in 0..3 {
        set_time(&rig, times(step, DAY));
        run(&rig).await.unwrap();
    }
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, Level::WARN);
    assert_eq!(events[0].message, UNSUPPORTED_TEXT);
    assert_eq!(events[0].field("volume"), Some("\"pgdata\""));
}

#[tokio::test]
async fn an_unsupported_system_does_not_change_the_mode() {
    let rig = rig();
    set_failure(&rig, DiskFailure::Unsupported);
    run(&rig).await.unwrap();
    assert!(!rig.switch.is_on());
    assert!(rig.switch.history().is_empty());
}

#[tokio::test]
async fn an_unsupported_system_keeps_the_mode_that_was_on() {
    let rig = rig_with(DiskCheckSettings::default(), true);
    set_failure(&rig, DiskFailure::Unsupported);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
    assert!(rig.switch.history().is_empty());
}

#[tokio::test]
async fn the_unsupported_line_does_not_use_up_the_fault_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_failure(&rig, DiskFailure::Unsupported);
    run(&rig).await.unwrap();
    set_failure(&rig, DiskFailure::Unreadable);
    set_time(&rig, MINUTE);
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].message, UNREADABLE_TEXT);
    assert!(rig.switch.is_on());
}
