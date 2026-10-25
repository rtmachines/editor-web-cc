//! Tests of the exact panic slots, the cursor, and the threshold of `panics` (finding CR-A8-3).
//!
//! A test that reads only the result of a run misses a clamp of one slot. These tests read the
//! slots and the cursor of the job after each run, with different times in each slot.

use std::sync::atomic::Ordering;

use super::support::{Rig, at, rig_with, sent_by_one_run, set_time};
use crate::jobs::alerts::AlertSettings;
use crate::ports::AlertCondition;

/// The value of a slot that holds no panic.
const EMPTY: i64 = i64::MIN;

/// The default `alerts.panic_window` in seconds: 10 minutes.
const WINDOW: i64 = 600;

/// Build a job with `alerts.panic_count` of 2, so the job keeps three slots.
fn three_slots() -> Rig {
    rig_with(AlertSettings {
        panic_count: 2,
        ..AlertSettings::default()
    })
}

/// Return the Unix seconds of the instant `seconds` after the start of the test.
fn unix(seconds: i64) -> i64 {
    at(seconds).as_offset_date_time().unix_timestamp()
}

/// Return the value of each slot.
fn slots(rig: &Rig) -> Vec<i64> {
    rig.job
        .state
        .panic_times
        .iter()
        .map(|slot| slot.load(Ordering::Relaxed))
        .collect()
}

/// Return the index of the slot that the next panic takes.
fn cursor(rig: &Rig) -> usize {
    rig.job.state.next_slot.load(Ordering::Relaxed)
}

/// Move the time to `seconds`, count `count` new panics, run the job, and return what it sent.
async fn run_at(rig: &Rig, seconds: i64, count: u32) -> Vec<AlertCondition> {
    set_time(rig, seconds);
    for _ in 0..count {
        rig.counters.record_panic();
    }
    sent_by_one_run(rig).await
}

#[tokio::test]
async fn the_cursor_wraps_and_the_oldest_slot_sets_the_threshold() {
    let rig = three_slots();
    assert!(run_at(&rig, 0, 2).await.is_empty());
    assert_eq!(slots(&rig), vec![unix(0), unix(0), EMPTY]);
    assert_eq!(cursor(&rig), 2);
    assert_eq!(run_at(&rig, 100, 2).await, vec![AlertCondition::Panics]);
    assert_eq!(slots(&rig), vec![unix(100), unix(0), unix(100)]);
    assert_eq!(cursor(&rig), 1);
    assert_eq!(run_at(&rig, WINDOW, 0).await, vec![AlertCondition::Panics]);
    assert!(run_at(&rig, WINDOW + 1, 0).await.is_empty());
    assert_eq!(slots(&rig), vec![unix(100), unix(0), unix(100)]);
    assert_eq!(cursor(&rig), 1);
}

#[tokio::test]
async fn a_clock_that_moves_back_clamps_every_slot_and_not_only_the_cursor_slot() {
    let rig = three_slots();
    for seconds in [50_000, 50_100, 50_200] {
        run_at(&rig, seconds, 1).await;
    }
    assert_eq!(slots(&rig), vec![unix(50_000), unix(50_100), unix(50_200)]);
    assert_eq!(cursor(&rig), 0);
    assert_eq!(run_at(&rig, 100, 0).await, vec![AlertCondition::Panics]);
    assert_eq!(slots(&rig), vec![unix(100), unix(100), unix(100)]);
    assert_eq!(cursor(&rig), 0);
}

#[tokio::test]
async fn the_clamped_slots_leave_the_window_at_the_exact_threshold() {
    let rig = three_slots();
    for seconds in [50_000, 50_100, 50_200] {
        run_at(&rig, seconds, 1).await;
    }
    run_at(&rig, 100, 0).await;
    assert_eq!(
        run_at(&rig, 100 + WINDOW, 1).await,
        vec![AlertCondition::Panics]
    );
    assert_eq!(slots(&rig), vec![unix(100 + WINDOW), unix(100), unix(100)]);
    assert_eq!(cursor(&rig), 1);
    assert!(run_at(&rig, 101 + WINDOW, 1).await.is_empty());
    assert_eq!(
        slots(&rig),
        vec![unix(100 + WINDOW), unix(101 + WINDOW), unix(100)]
    );
    assert_eq!(cursor(&rig), 2);
}
