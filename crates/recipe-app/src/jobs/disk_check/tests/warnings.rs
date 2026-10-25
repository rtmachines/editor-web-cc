//! Tests of the low-disk `WARN` line of the job `disk_check` (reliability.md 7.8, ADR-0060 (aa)).

use tracing::Level;

use super::support::{
    HOUR, LogCapture, MINUTE, Rig, TEN_MINUTES, TOTAL, after, rig, rig_with, run, set_available,
    set_time, times,
};
use crate::jobs::disk_check::DiskCheckSettings;
use crate::ports::ReadOnlySwitch;

/// Free blocks that are below the warning threshold and above the hard threshold: 14 %.
const LOW: u64 = 140;
/// Free blocks exactly at the warning threshold: 15 %.
const AT_WARNING: u64 = 150;

#[tokio::test]
async fn free_space_below_the_warning_threshold_writes_one_warn_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, LOW);
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, Level::WARN);
    assert_eq!(events[0].field("volume"), Some("\"pgdata\""));
    assert_eq!(events[0].field("available_blocks"), Some("140"));
    assert_eq!(events[0].field("total_blocks"), Some("1000"));
    assert!(!rig.switch.is_on());
}

#[tokio::test]
async fn free_space_at_the_warning_threshold_writes_no_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, AT_WARNING);
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 0);
}

#[tokio::test]
async fn free_space_below_the_hard_threshold_also_writes_the_warn_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, TOTAL / 50);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
    assert_eq!(logs.count(), 1);
}

#[tokio::test]
async fn the_warn_line_repeats_every_log_interval() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, LOW);
    run(&rig).await.unwrap();
    set_time(&rig, MINUTE);
    run(&rig).await.unwrap();
    set_time(&rig, TEN_MINUTES - 1);
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 1);
    set_time(&rig, TEN_MINUTES);
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 2);
    set_time(&rig, after(TEN_MINUTES, TEN_MINUTES - 1));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 2);
    set_time(&rig, after(TEN_MINUTES, TEN_MINUTES));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 3);
}

#[tokio::test]
async fn a_recovery_does_not_reset_the_log_interval() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, LOW);
    run(&rig).await.unwrap();
    set_available(&rig, TOTAL / 2);
    set_time(&rig, MINUTE);
    run(&rig).await.unwrap();
    set_available(&rig, LOW);
    set_time(&rig, after(MINUTE, MINUTE));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 1);
}

#[tokio::test]
async fn a_clock_that_moved_back_keeps_the_line_waiting_and_it_returns_later() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, LOW);
    set_time(&rig, 5 * HOUR);
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 1);
    set_time(&rig, HOUR);
    run(&rig).await.unwrap();
    set_time(&rig, after(HOUR, TEN_MINUTES));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 1);
    set_time(&rig, after(5 * HOUR, TEN_MINUTES));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 2);
}

/// Free blocks below the hard threshold: 2 %.
const BELOW_HARD: u64 = TOTAL / 50;
/// Free blocks between the hard threshold and the resume threshold: 7 %.
const BEFORE_RESUME: u64 = 70;

/// Move the clock to `seconds`, run the job once, and check the line count and the mode.
async fn run_expecting(rig: &Rig, logs: &LogCapture, seconds: i64, lines: usize) {
    set_time(rig, seconds);
    run(rig).await.unwrap();
    assert_eq!(logs.count(), lines, "line count at {seconds} s");
    assert!(rig.switch.is_on(), "switch state at {seconds} s");
}

#[tokio::test]
async fn below_the_hard_threshold_the_rate_limit_keeps_the_mode_on_and_the_lines_apart() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, BELOW_HARD);
    run_expecting(&rig, &logs, 0, 1).await;
    run_expecting(&rig, &logs, MINUTE, 1).await;
    run_expecting(&rig, &logs, TEN_MINUTES - 1, 1).await;
    run_expecting(&rig, &logs, TEN_MINUTES, 2).await;
    run_expecting(&rig, &logs, after(TEN_MINUTES, MINUTE), 2).await;
    run_expecting(&rig, &logs, times(2, TEN_MINUTES), 3).await;
}

#[tokio::test]
async fn between_the_hard_and_the_resume_threshold_the_mode_stays_on_with_the_same_rate_limit() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, BELOW_HARD);
    run_expecting(&rig, &logs, 0, 1).await;
    set_available(&rig, BEFORE_RESUME);
    run_expecting(&rig, &logs, MINUTE, 1).await;
    run_expecting(&rig, &logs, TEN_MINUTES - 1, 1).await;
    run_expecting(&rig, &logs, TEN_MINUTES, 2).await;
    run_expecting(&rig, &logs, after(TEN_MINUTES, MINUTE), 2).await;
    run_expecting(&rig, &logs, times(2, TEN_MINUTES), 3).await;
}

#[tokio::test]
async fn a_mode_that_is_on_before_the_first_run_writes_one_line_between_both_thresholds() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig_with(DiskCheckSettings::default(), true);
    set_available(&rig, BEFORE_RESUME);
    run_expecting(&rig, &logs, 0, 1).await;
    run_expecting(&rig, &logs, TEN_MINUTES - 1, 1).await;
    run_expecting(&rig, &logs, TEN_MINUTES, 2).await;
}

#[tokio::test]
async fn the_warn_line_names_no_path() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_available(&rig, LOW);
    run(&rig).await.unwrap();
    let text = format!("{:?}", logs.events());
    assert!(!text.contains("/srv"));
}
