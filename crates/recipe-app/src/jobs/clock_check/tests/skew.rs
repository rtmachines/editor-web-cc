//! The `WARN` line for a skew above `clock.skew_warn` (`reliability.md` 7.9, spec "Clock skew
//! warning"). The border is strict: a skew equal to the limit gives no line.

use std::time::Duration;

use tracing::Level;

use super::support::{
    LogCapture, SECOND, behind, rig, rig_with, run, run_kind, seconds, set_host_offset,
};
use crate::jobs::RunKind;
use crate::jobs::clock_check::ClockCheckSettings;
use crate::memory::OperationsMethod;

#[tokio::test]
async fn a_host_ahead_by_7_seconds_writes_one_warn_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(7));
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, Level::WARN);
    assert_eq!(events[0].field("skew"), Some("7s"));
    assert_eq!(events[0].field("limit"), Some("5s"));
}

#[tokio::test]
async fn a_host_behind_by_7_seconds_writes_one_warn_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, behind(seconds(7)));
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, Level::WARN);
    assert_eq!(events[0].field("skew"), Some("7s"));
}

#[tokio::test]
async fn a_skew_of_3_seconds_writes_no_line_in_both_directions() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(3));
    run(&rig).await.unwrap();
    set_host_offset(&rig, behind(seconds(3)));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 0);
}

#[tokio::test]
async fn equal_clocks_write_no_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 0);
}

#[tokio::test]
async fn a_skew_equal_to_the_limit_writes_no_line_in_both_directions() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(5));
    run(&rig).await.unwrap();
    set_host_offset(&rig, behind(seconds(5)));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 0);
}

#[tokio::test]
async fn a_skew_one_nanosecond_above_the_limit_writes_one_line_in_both_directions() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(5).checked_add(1).unwrap());
    run(&rig).await.unwrap();
    set_host_offset(&rig, behind(seconds(5).checked_add(1).unwrap()));
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 2);
}

#[tokio::test]
async fn a_skew_of_5_9_seconds_shows_the_exact_skew_next_to_the_limit_of_5_seconds() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    let nine_tenths = SECOND.checked_div(10).unwrap().checked_mul(9).unwrap();
    let skew = seconds(5).checked_add(nine_tenths).unwrap();
    set_host_offset(&rig, skew);
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].field("skew"), Some("5.9s"));
    assert_eq!(events[0].field("limit"), Some("5s"));
}

#[tokio::test]
async fn a_host_behind_by_5_9_seconds_shows_the_exact_skew_across_a_second_border() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    let nine_tenths = SECOND.checked_div(10).unwrap().checked_mul(9).unwrap();
    let skew = seconds(5).checked_add(nine_tenths).unwrap();
    set_host_offset(&rig, behind(skew));
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].field("skew"), Some("5.9s"));
}

#[tokio::test]
async fn the_line_has_no_field_that_rounds_down_to_whole_seconds() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(7));
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events[0].field("skew_seconds"), None);
    assert_eq!(events[0].field("limit_seconds"), None);
}

#[tokio::test]
async fn the_line_holds_only_the_skew_and_the_limit() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(7));
    run(&rig).await.unwrap();
    let events = logs.events();
    let names: Vec<&str> = events[0]
        .fields
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| *name != "message")
        .collect();
    assert_eq!(names, ["skew", "limit"]);
}

#[tokio::test]
async fn a_run_reads_the_time_of_the_database_once() {
    let rig = rig();
    run(&rig).await.unwrap();
    assert_eq!(rig.operations.calls(OperationsMethod::DatabaseNow), 1);
}

#[tokio::test]
async fn every_run_with_a_skew_writes_its_own_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(7));
    run(&rig).await.unwrap();
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 2);
}

#[tokio::test]
async fn the_limit_comes_from_the_settings() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig_with(ClockCheckSettings {
        skew_warn: Duration::from_secs(1),
    });
    set_host_offset(&rig, seconds(2));
    run(&rig).await.unwrap();
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].field("limit"), Some("1s"));
}

#[tokio::test]
async fn the_run_kind_has_no_effect() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    set_host_offset(&rig, seconds(7));
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    run_kind(&rig, RunKind::Scheduled).await.unwrap();
    assert_eq!(logs.count(), 2);
}

#[test]
fn the_default_limit_equals_the_table_of_the_settings() {
    // `reliability.md` 7.11: `clock.skew_warn` is 5 s.
    assert_eq!(
        ClockCheckSettings::default().skew_warn,
        Duration::from_secs(5)
    );
}
