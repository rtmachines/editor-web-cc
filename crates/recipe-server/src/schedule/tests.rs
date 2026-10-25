//! Tests of the schedules (design D14 row T20).
//!
//! A schedule is a pure function of two instants, so no test here needs a runtime or a clock.
//! The tests use the moments of the day as seconds since midnight: 02:59:30 is 10 770.

use std::time::Duration;

use super::{Moment, RunKind, Schedule, ScheduleFault, TimeOfDay, Trigger};

/// The seconds of a UTC day.
const DAY: u64 = 86_400;

/// 03:00 UTC, the time of the nightly check (`audit.check_time`).
const THREE: u64 = 10_800;

fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

/// The answer for a wait that an interval or a time of day fires.
fn scheduled(seconds: u64) -> Option<(Duration, RunKind)> {
    Some((secs(seconds), RunKind::Scheduled))
}

/// The answer for a wait that a one-shot trigger fires.
fn catch_up(seconds: u64) -> Option<(Duration, RunKind)> {
    Some((secs(seconds), RunKind::CatchUp))
}

/// A moment `since_start` after the start of the supervisor and `time_of_day` seconds after midnight.
fn moment(since_start: u64, time_of_day: u64) -> Moment {
    let unix = i64::try_from(time_of_day).unwrap();
    Moment::new(secs(since_start), unix, 0)
}

fn nightly() -> Schedule {
    Schedule::daily_at(TimeOfDay::parse(3, 0).unwrap())
}

#[test]
fn every_waits_one_interval_after_each_run_whatever_the_time() {
    let mut schedule = Schedule::every(secs(60)).unwrap();
    for since_start in [0, 60, 150, 100_000] {
        assert_eq!(schedule.next_delay(moment(since_start, 5)), scheduled(60));
    }
}

#[test]
fn every_refuses_a_zero_interval() {
    assert_eq!(
        Schedule::every(Duration::ZERO).unwrap_err(),
        ScheduleFault::ZeroInterval
    );
    assert_eq!(
        Schedule::now_then_every(Duration::ZERO).unwrap_err(),
        ScheduleFault::ZeroInterval
    );
}

#[test]
fn now_then_every_runs_at_once_and_then_every_interval() {
    let mut schedule = Schedule::now_then_every(secs(3_600)).unwrap();
    assert_eq!(schedule.next_delay(moment(0, 0)), catch_up(0));
    assert_eq!(schedule.next_delay(moment(1, 0)), scheduled(3_600));
    assert_eq!(schedule.next_delay(moment(3_601, 0)), scheduled(3_600));
}

#[test]
fn the_first_run_of_now_then_every_is_a_one_shot_run_and_gets_the_catch_up_kind() {
    let mut schedule = Schedule::now_then_every(secs(3_600)).unwrap();
    let first = schedule.next_delay(moment(0, 0));
    assert_eq!(first, Some((Duration::ZERO, RunKind::CatchUp)));
    let later = schedule.next_delay(moment(0, 0));
    assert_eq!(later, scheduled(3_600));
}

#[test]
fn daily_waits_until_the_next_time_of_day() {
    let mut schedule = nightly();
    assert_eq!(schedule.next_delay(moment(0, 10_770)), scheduled(30));
    assert_eq!(schedule.next_delay(moment(0, 0)), scheduled(THREE));
}

#[test]
fn daily_at_the_exact_time_waits_a_full_day() {
    assert_eq!(nightly().next_delay(moment(0, THREE)), scheduled(DAY));
}

#[test]
fn daily_after_the_time_wraps_over_midnight() {
    let mut schedule = nightly();
    assert_eq!(schedule.next_delay(moment(0, 10_801)), scheduled(86_399));
    assert_eq!(schedule.next_delay(moment(0, 86_399)), scheduled(10_801));
}

#[test]
fn daily_counts_the_fraction_of_a_second() {
    let half_second_early = Moment::new(Duration::ZERO, 10_799, 500_000_000);
    assert_eq!(
        nightly().next_delay(half_second_early),
        Some((Duration::from_millis(500), RunKind::Scheduled))
    );
}

#[test]
fn daily_reduces_any_unix_time_to_the_utc_day() {
    // Day 20 000 after the epoch, at 02:59:30.
    let far_future = 1_728_010_770;
    let before_epoch = -1;
    assert_eq!(
        nightly().next_delay(Moment::new(Duration::ZERO, far_future, 0)),
        scheduled(30)
    );
    assert_eq!(
        nightly().next_delay(Moment::new(Duration::ZERO, before_epoch, 0)),
        scheduled(10_801)
    );
}

#[test]
fn once_after_fires_a_single_time() {
    let mut schedule = Schedule::once_after(secs(600));
    assert_eq!(schedule.next_delay(moment(0, 0)), catch_up(600));
    assert_eq!(schedule.next_delay(moment(600, 0)), None);
}

#[test]
fn once_after_counts_from_the_start_and_not_from_the_last_call() {
    let mut schedule = Schedule::once_after(secs(600));
    assert_eq!(schedule.next_delay(moment(100, 0)), catch_up(500));
    let mut late = Schedule::once_after(secs(600));
    assert_eq!(late.next_delay(moment(900, 0)), catch_up(0));
}

#[test]
fn a_catch_up_runs_before_the_nightly_run_when_it_is_nearer() {
    let mut schedule = nightly().with_once_after(secs(600));
    let noon = 43_200;
    assert_eq!(schedule.next_delay(moment(0, noon)), catch_up(600));
    let after_catch_up = schedule.next_delay(moment(600, 43_800));
    // From 12:10 to 03:00 of the next day.
    assert_eq!(after_catch_up, scheduled(53_400));
}

#[test]
fn a_nearer_nightly_run_goes_first_and_the_catch_up_stays_pending() {
    let mut schedule = nightly().with_once_after(secs(600));
    assert_eq!(schedule.next_delay(moment(0, 10_500)), scheduled(300));
    assert_eq!(schedule.next_delay(moment(300, THREE)), catch_up(300));
}

#[test]
fn a_catch_up_that_is_overdue_after_a_long_run_starts_at_once() {
    let mut schedule = nightly().with_once_after(secs(600));
    assert_eq!(schedule.next_delay(moment(0, 10_790)), scheduled(10));
    assert_eq!(schedule.next_delay(moment(1_010, 11_800)), catch_up(0));
}

#[test]
fn each_trigger_gives_its_own_run_kind() {
    let every = Schedule::every(secs(60)).unwrap().next_delay(moment(0, 0));
    let daily = nightly().next_delay(moment(0, 0));
    let once = Schedule::once_after(secs(60)).next_delay(moment(0, 0));
    assert_eq!(every, scheduled(60));
    assert_eq!(daily, scheduled(THREE));
    assert_eq!(once, catch_up(60));
}

#[test]
fn two_triggers_that_are_due_together_give_the_scheduled_kind_whatever_their_order() {
    // The public builders always put the one-shot trigger last. The test puts it first, so the
    // tie rule and not the order of the list picks the kind.
    let mut schedule = Schedule {
        triggers: vec![
            Trigger::Once {
                at: secs(300),
                fired: false,
            },
            Trigger::Every(secs(300)),
        ],
    };
    assert_eq!(schedule.next_delay(moment(0, 0)), scheduled(300));
}

#[test]
fn a_one_shot_trigger_that_lost_a_tie_stays_pending_and_fires_after_the_scheduled_run() {
    let mut schedule = nightly().with_once_after(secs(300));
    assert_eq!(schedule.next_delay(moment(0, 10_500)), scheduled(300));
    // The one-shot trigger is due at 300 s, so it fires at once after the scheduled run.
    assert_eq!(schedule.next_delay(moment(300, THREE)), catch_up(0));
}

/// 02:50 UTC, ten minutes before the nightly time.
const TWO_FIFTY: u64 = 10_200;

#[test]
fn a_nightly_occurrence_that_fell_due_during_a_catch_up_run_fires_at_once_after_it() {
    let mut schedule = nightly().with_once_after(Duration::ZERO);
    assert_eq!(schedule.next_delay(moment(0, TWO_FIFTY)), catch_up(0));
    // The catch-up run lasts 20 minutes and ends at 03:10, ten minutes after the nightly time.
    assert_eq!(schedule.next_delay(moment(1_200, 11_400)), scheduled(0));
    // The missed occurrence is spent. The next one is the nightly time of the next day.
    assert_eq!(
        schedule.next_delay(moment(1_300, 11_500)),
        scheduled(85_700)
    );
}

#[test]
fn a_run_that_spans_two_nightly_times_gives_one_nightly_run() {
    let mut schedule = nightly().with_once_after(Duration::ZERO);
    assert_eq!(schedule.next_delay(moment(0, TWO_FIFTY)), catch_up(0));
    // The run lasts 25 hours and ends at 03:50 on the second day, after two nightly times.
    let after_run = DAY.checked_add(3_600).unwrap();
    let time_of_day = TWO_FIFTY.checked_add(after_run).unwrap().rem_euclid(DAY);
    assert_eq!(
        schedule.next_delay(moment(after_run, time_of_day)),
        scheduled(0)
    );
    let later = after_run.checked_add(100).unwrap();
    let later_time = time_of_day.checked_add(100).unwrap();
    assert_eq!(
        schedule.next_delay(moment(later, later_time)),
        scheduled(83_300)
    );
}

#[test]
fn calls_without_a_nightly_fire_in_between_keep_the_due_occurrence() {
    let mut schedule = nightly()
        .with_once_after(secs(100))
        .with_once_after(secs(200));
    assert_eq!(schedule.next_delay(moment(0, TWO_FIFTY)), catch_up(100));
    assert_eq!(schedule.next_delay(moment(10, 10_210)), catch_up(190));
    // Both one-shot triggers are spent. The nightly occurrence of 03:00 is overdue at 03:06:40.
    assert_eq!(schedule.next_delay(moment(1_000, 11_200)), scheduled(0));
}

#[test]
fn time_of_day_accepts_the_edges_of_the_day() {
    assert!(TimeOfDay::parse(0, 0).is_ok());
    assert!(TimeOfDay::parse(23, 59).is_ok());
}

#[test]
fn time_of_day_refuses_an_hour_or_a_minute_out_of_range() {
    assert_eq!(TimeOfDay::parse(24, 0).unwrap_err(), ScheduleFault::Hour);
    assert_eq!(TimeOfDay::parse(3, 60).unwrap_err(), ScheduleFault::Minute);
}

#[test]
fn a_fault_names_the_bad_value_in_one_short_sentence() {
    for fault in [
        ScheduleFault::ZeroInterval,
        ScheduleFault::Hour,
        ScheduleFault::Minute,
    ] {
        let text = fault.to_string();
        assert!(text.split_whitespace().count() <= 25, "{text}");
        assert!(text.ends_with('.'), "{text}");
    }
}
