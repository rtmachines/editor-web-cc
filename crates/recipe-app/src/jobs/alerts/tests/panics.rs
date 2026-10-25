//! Tests of the condition `panics` and its slots (ADR-0073 (g), ADR-0060 (aa)).

use super::support::{LogCapture, Rig, rig, rig_with, run, sent_by_one_run, set_time};
use crate::jobs::alerts::AlertSettings;
use crate::ports::AlertCondition;

/// The default `alerts.panic_window` in seconds: 10 minutes.
const WINDOW: i64 = 600;

/// Count `count` panics.
fn panic_times(rig: &Rig, count: u32) {
    for _ in 0..count {
        rig.counters.record_panic();
    }
}

/// Count one panic at `seconds`, run the job, and return the conditions that it sent.
async fn panic_at(rig: &Rig, seconds: i64) -> Vec<AlertCondition> {
    set_time(rig, seconds);
    panic_times(rig, 1);
    sent_by_one_run(rig).await
}

#[tokio::test]
async fn three_panics_are_not_more_than_three() {
    let rig = rig();
    panic_times(&rig, 3);
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn four_panics_in_one_run_make_panics_active() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    panic_times(&rig, 4);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
    assert_eq!(logs.alert_names(), vec!["panics"]);
}

#[tokio::test]
async fn more_new_panics_than_slots_make_panics_active() {
    let rig = rig();
    panic_times(&rig, 40);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
}

#[tokio::test]
async fn panics_over_several_runs_fill_the_slots_one_by_one() {
    let rig = rig();
    assert!(panic_at(&rig, 0).await.is_empty());
    assert!(panic_at(&rig, 100).await.is_empty());
    assert!(panic_at(&rig, 200).await.is_empty());
    assert_eq!(panic_at(&rig, 300).await, vec![AlertCondition::Panics]);
}

#[tokio::test]
async fn the_condition_stays_active_while_the_oldest_panic_is_in_the_window() {
    let rig = rig();
    panic_times(&rig, 4);
    run(&rig).await.unwrap();
    set_time(&rig, WINDOW);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
    set_time(&rig, WINDOW + 1);
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn a_panic_is_counted_once() {
    let rig = rig();
    panic_times(&rig, 4);
    run(&rig).await.unwrap();
    set_time(&rig, WINDOW + 1);
    assert!(sent_by_one_run(&rig).await.is_empty());
    set_time(&rig, 2 * WINDOW + 2);
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn a_new_panic_replaces_the_oldest_slot() {
    let rig = rig();
    for seconds in [0, 100, 200, 300] {
        panic_at(&rig, seconds).await;
    }
    set_time(&rig, 650);
    assert!(sent_by_one_run(&rig).await.is_empty());
    // The oldest panic (time 0) is now outside the window. A new panic takes its slot.
    assert_eq!(panic_at(&rig, 650).await, vec![AlertCondition::Panics]);
    // The oldest panic is now the one at 100. At 750 it leaves the window.
    set_time(&rig, 750);
    assert!(sent_by_one_run(&rig).await.is_empty());
    // A new panic takes the slot of 100, and the oldest is the one at 200.
    assert_eq!(panic_at(&rig, 750).await, vec![AlertCondition::Panics]);
}

#[tokio::test]
async fn the_number_of_slots_follows_the_panic_count_setting() {
    let settings = AlertSettings {
        panic_count: 0,
        ..AlertSettings::default()
    };
    let rig = rig_with(settings);
    assert_eq!(panic_at(&rig, 0).await, vec![AlertCondition::Panics]);
    set_time(&rig, WINDOW + 1);
    assert!(sent_by_one_run(&rig).await.is_empty());
}

/// Build a job with `panic_count` and return the number of its panic slots.
fn slots_for(panic_count: u16) -> usize {
    let settings = AlertSettings {
        panic_count,
        ..AlertSettings::default()
    };
    rig_with(settings).job.state.panic_times.len()
}

#[test]
fn the_default_panic_count_gives_four_slots() {
    assert_eq!(slots_for(3), 4);
}

#[test]
fn a_large_panic_count_gives_one_more_slot_than_the_count() {
    assert_eq!(slots_for(2000), 2001);
}

#[test]
fn the_largest_panic_count_gives_65536_slots() {
    assert_eq!(slots_for(u16::MAX), 65_536);
}

#[tokio::test]
async fn a_large_panic_count_is_the_exact_threshold() {
    let settings = AlertSettings {
        panic_count: 2000,
        ..AlertSettings::default()
    };
    let rig = rig_with(settings);
    panic_times(&rig, 2000);
    assert!(sent_by_one_run(&rig).await.is_empty());
    set_time(&rig, 1);
    panic_times(&rig, 1);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
}

#[tokio::test]
async fn slots_from_the_future_are_clamped_when_the_clock_moves_back() {
    let rig = rig();
    set_time(&rig, 50_000);
    panic_times(&rig, 4);
    run(&rig).await.unwrap();
    set_time(&rig, 100);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
    set_time(&rig, 100 + WINDOW);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
    set_time(&rig, 100 + WINDOW + 1);
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn the_panic_slots_use_the_time_of_the_snapshot() {
    let rig = rig();
    set_time(&rig, 500);
    panic_times(&rig, 4);
    run(&rig).await.unwrap();
    set_time(&rig, 500 + WINDOW);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
    set_time(&rig, 500 + WINDOW + 1);
    assert!(sent_by_one_run(&rig).await.is_empty());
}
