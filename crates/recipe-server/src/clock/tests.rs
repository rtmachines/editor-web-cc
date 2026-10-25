//! Tests of the system clock.

use recipe_app::ports::Clock;
use time::OffsetDateTime;

use super::SystemClock;

#[test]
fn now_lies_between_two_reads_of_the_system_clock() {
    let before = OffsetDateTime::now_utc();
    let read = SystemClock.now();
    let after = OffsetDateTime::now_utc();

    let instant = *read.as_offset_date_time();
    assert!(before <= instant, "{before} must not follow {instant}");
    assert!(instant <= after, "{instant} must not follow {after}");
}

#[test]
fn now_carries_the_utc_offset() {
    assert!(SystemClock.now().as_offset_date_time().offset().is_utc());
}

#[test]
fn clones_of_the_clock_read_the_same_system_time() {
    let clock = SystemClock;
    let first = clock.now();
    let second = clock.clone().now();
    assert!(first <= second);
}
