//! Tests of the settings and of the thresholds of the job `disk_check` (reliability.md 7.8).

use std::time::Duration;

use super::support::{
    LogCapture, TOTAL, pgdata, rig, rig_with, run, run_kind, set_available, set_space,
};
use crate::jobs::RunKind;
use crate::jobs::disk_check::DiskCheckSettings;
use crate::ports::ReadOnlySwitch;

#[test]
fn the_defaults_equal_section_7_11() {
    let settings = DiskCheckSettings::default();
    assert_eq!(settings.warn_percent, 15);
    assert_eq!(settings.readonly_percent, 5);
    assert_eq!(settings.resume_percent, 10);
    assert_eq!(settings.warn_log_interval, Duration::from_secs(600));
}

#[tokio::test]
async fn free_space_below_the_hard_threshold_turns_the_mode_on() {
    let rig = rig();
    set_available(&rig, 49);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
}

#[tokio::test]
async fn free_space_at_the_hard_threshold_leaves_the_mode_off() {
    let rig = rig();
    set_available(&rig, 50);
    run(&rig).await.unwrap();
    assert!(!rig.switch.is_on());
    assert!(rig.switch.history().is_empty());
}

#[tokio::test]
async fn free_space_above_the_resume_threshold_turns_the_mode_off() {
    let rig = rig_with(Default::default(), true);
    set_available(&rig, 101);
    run(&rig).await.unwrap();
    assert!(!rig.switch.is_on());
}

#[tokio::test]
async fn free_space_at_the_resume_threshold_keeps_the_mode_on() {
    let rig = rig_with(Default::default(), true);
    set_available(&rig, 100);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
    assert!(rig.switch.history().is_empty());
}

#[tokio::test]
async fn free_space_between_both_thresholds_leaves_the_mode_on() {
    let rig = rig_with(Default::default(), true);
    set_available(&rig, 80);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
    assert!(rig.switch.history().is_empty());
}

#[tokio::test]
async fn free_space_between_both_thresholds_leaves_the_mode_off() {
    let rig = rig();
    set_available(&rig, 80);
    run(&rig).await.unwrap();
    assert!(!rig.switch.is_on());
    assert!(rig.switch.history().is_empty());
}

#[tokio::test]
async fn the_mode_follows_the_free_space_over_several_runs() {
    let rig = rig();
    for (available, expected) in [
        (40, true),
        (80, true),
        (120, false),
        (80, false),
        (30, true),
    ] {
        set_available(&rig, available);
        run(&rig).await.unwrap();
        assert_eq!(rig.switch.is_on(), expected, "available: {available}");
    }
}

#[tokio::test]
async fn the_thresholds_come_from_the_settings() {
    let settings = DiskCheckSettings {
        warn_percent: 30,
        readonly_percent: 20,
        resume_percent: 25,
        ..DiskCheckSettings::default()
    };
    let rig = rig_with(settings, false);
    set_available(&rig, 199);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
    set_available(&rig, 251);
    run(&rig).await.unwrap();
    assert!(!rig.switch.is_on());
}

#[tokio::test]
async fn the_compare_does_not_overflow_on_the_largest_file_system() {
    let rig = rig();
    set_space(&rig, u64::MAX / 100, u64::MAX);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
    set_space(&rig, u64::MAX, u64::MAX);
    run(&rig).await.unwrap();
    assert!(!rig.switch.is_on());
}

#[tokio::test]
async fn a_file_system_without_blocks_turns_the_mode_on() {
    let rig = rig();
    set_space(&rig, 0, 0);
    run(&rig).await.unwrap();
    assert!(rig.switch.is_on());
}

#[tokio::test]
async fn each_run_reads_pgdata_once_and_nothing_else() {
    let rig = rig();
    run(&rig).await.unwrap();
    run(&rig).await.unwrap();
    assert_eq!(rig.disk.queries(), vec![pgdata(), pgdata()]);
}

#[tokio::test]
async fn the_run_kind_has_no_effect() {
    for kind in [RunKind::Scheduled, RunKind::CatchUp] {
        let rig = rig();
        set_available(&rig, TOTAL / 50);
        run_kind(&rig, kind).await.unwrap();
        assert!(rig.switch.is_on(), "kind: {kind:?}");
    }
}

#[tokio::test]
async fn a_healthy_disk_writes_no_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 0);
}
