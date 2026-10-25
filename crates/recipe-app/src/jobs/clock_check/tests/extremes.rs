//! The skew at extreme instants (`reliability.md` 7.9).
//!
//! The tests of `skew.rs` use times near one million seconds after the epoch. These tests use
//! instants before the epoch and the earliest and the latest instant of the type. Each test runs
//! both directions: the host ahead of the database and the host behind it.

use std::time::Duration;

use time::OffsetDateTime;

use super::support::{
    LogCapture, Logged, at_nanos, behind, earliest, exact_skew, latest, rig, run, seconds,
    set_clocks, shifted,
};

/// Run the job once with the clocks at `database` and `host`. Return the events of the run.
async fn events_at(database: OffsetDateTime, host: OffsetDateTime) -> Vec<Logged> {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_clocks(&rig, database, host);
    run(&rig).await.unwrap();
    logs.events()
}

/// Run the job with `first` as the database and `second` as the host, then the other way round.
async fn both_directions(
    first: OffsetDateTime,
    second: OffsetDateTime,
) -> (Vec<Logged>, Vec<Logged>) {
    let host_ahead = events_at(first, second).await;
    let host_behind = events_at(second, first).await;
    (host_ahead, host_behind)
}

/// Assert that the events hold one `WARN` line with the skew `skew` and the limit of 5 seconds.
fn assert_one_line(events: &[Logged], skew: Duration) {
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, tracing::Level::WARN);
    assert_eq!(events[0].field("skew"), Some(format!("{skew:?}").as_str()));
    assert_eq!(events[0].field("limit"), Some("5s"));
}

/// Return the limit of 5 seconds plus `nanos` nanoseconds, as a `Duration`.
fn limit_plus(nanos: u32) -> Duration {
    Duration::new(5, nanos)
}

#[tokio::test]
async fn a_skew_between_two_instants_before_the_epoch_writes_one_line_in_both_directions() {
    let database = at_nanos(behind(seconds(1_000_000_000)));
    let host = shifted(database, seconds(7));
    let (host_ahead, host_behind) = both_directions(database, host).await;
    assert_one_line(&host_ahead, Duration::from_secs(7));
    assert_one_line(&host_behind, Duration::from_secs(7));
}

#[tokio::test]
async fn a_skew_between_two_instants_before_the_epoch_below_the_limit_writes_no_line() {
    let database = at_nanos(behind(seconds(1_000_000_000)));
    let host = shifted(database, seconds(3));
    let (host_ahead, host_behind) = both_directions(database, host).await;
    assert!(host_ahead.is_empty());
    assert!(host_behind.is_empty());
}

#[tokio::test]
async fn a_skew_across_the_epoch_writes_one_line_in_both_directions() {
    let before = at_nanos(behind(seconds(3)));
    let after = at_nanos(seconds(3));
    let (host_ahead, host_behind) = both_directions(before, after).await;
    assert_one_line(&host_ahead, Duration::from_secs(6));
    assert_one_line(&host_behind, Duration::from_secs(6));
}

#[tokio::test]
async fn a_skew_across_the_epoch_equal_to_the_limit_writes_no_line() {
    let half = seconds(5).checked_div(2).unwrap();
    let before = at_nanos(behind(half));
    let after = at_nanos(half);
    let (host_ahead, host_behind) = both_directions(before, after).await;
    assert!(host_ahead.is_empty());
    assert!(host_behind.is_empty());
}

#[tokio::test]
async fn a_skew_across_the_epoch_one_nanosecond_above_the_limit_writes_one_line() {
    let half = seconds(5).checked_div(2).unwrap();
    let before = at_nanos(behind(half));
    let after = at_nanos(half.checked_add(1).unwrap());
    let (host_ahead, host_behind) = both_directions(before, after).await;
    assert_one_line(&host_ahead, limit_plus(1));
    assert_one_line(&host_behind, limit_plus(1));
}

#[tokio::test]
async fn a_skew_between_the_earliest_and_the_latest_instant_writes_one_line() {
    let (earliest, latest) = (earliest(), latest());
    let skew = exact_skew(earliest, latest);
    let (host_ahead, host_behind) = both_directions(earliest, latest).await;
    assert_one_line(&host_ahead, skew);
    assert_one_line(&host_behind, skew);
}

#[tokio::test]
async fn a_skew_between_the_earliest_instant_and_the_epoch_writes_one_line() {
    let (earliest, epoch) = (earliest(), OffsetDateTime::UNIX_EPOCH);
    let skew = exact_skew(earliest, epoch);
    let (host_ahead, host_behind) = both_directions(earliest, epoch).await;
    assert_one_line(&host_ahead, skew);
    assert_one_line(&host_behind, skew);
}

#[tokio::test]
async fn a_skew_between_the_epoch_and_the_latest_instant_writes_one_line() {
    let (epoch, latest) = (OffsetDateTime::UNIX_EPOCH, latest());
    let skew = exact_skew(epoch, latest);
    let (host_ahead, host_behind) = both_directions(epoch, latest).await;
    assert_one_line(&host_ahead, skew);
    assert_one_line(&host_behind, skew);
}

#[tokio::test]
async fn a_skew_equal_to_the_limit_at_the_earliest_instant_writes_no_line() {
    let earliest = earliest();
    let (host_ahead, host_behind) = both_directions(earliest, shifted(earliest, seconds(5))).await;
    assert!(host_ahead.is_empty());
    assert!(host_behind.is_empty());
}

#[tokio::test]
async fn a_skew_one_nanosecond_above_the_limit_at_the_earliest_instant_writes_one_line() {
    let earliest = earliest();
    let later = shifted(earliest, seconds(5).checked_add(1).unwrap());
    let (host_ahead, host_behind) = both_directions(earliest, later).await;
    assert_one_line(&host_ahead, limit_plus(1));
    assert_one_line(&host_behind, limit_plus(1));
}

#[tokio::test]
async fn a_skew_equal_to_the_limit_at_the_latest_instant_writes_no_line() {
    let latest = latest();
    let (host_ahead, host_behind) =
        both_directions(shifted(latest, behind(seconds(5))), latest).await;
    assert!(host_ahead.is_empty());
    assert!(host_behind.is_empty());
}

#[tokio::test]
async fn a_skew_one_nanosecond_above_the_limit_at_the_latest_instant_writes_one_line() {
    let latest = latest();
    let earlier = shifted(latest, behind(seconds(5).checked_add(1).unwrap()));
    let (host_ahead, host_behind) = both_directions(earlier, latest).await;
    assert_one_line(&host_ahead, limit_plus(1));
    assert_one_line(&host_behind, limit_plus(1));
}
