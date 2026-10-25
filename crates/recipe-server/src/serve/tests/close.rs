//! The close of a running server and the warning lines of its report.

use recipe_db::CloseOutcome;

use super::expect_running;
use crate::serve::server::CloseReport;
use crate::serve::test_support::LogCapture;
use crate::serve::test_support::stub::{Event, Pool, StubHost};

#[test]
fn the_close_disarms_then_closes_both_pools_then_stops_the_watchdog_thread() {
    let host = StubHost::new();
    let server = expect_running(&host);
    let before = host.log.events().len();
    let report = server.close();
    let events = host.log.events();
    assert_eq!(
        events.get(before..).unwrap(),
        [
            Event::Disarm,
            Event::PoolClosed(Pool::Main),
            Event::PoolClosed(Pool::Health),
            Event::WatchdogStop,
        ]
    );
    assert_eq!(
        report,
        CloseReport {
            main_pool: Some(CloseOutcome::Closed),
            health_pool: Some(CloseOutcome::Closed),
            ..CloseReport::default()
        }
    );
}

#[test]
fn a_pool_that_times_out_shows_in_the_report() {
    let mut host = StubHost::new();
    host.pool_outcome = CloseOutcome::TimedOut;
    let server = expect_running(&host);
    let report = server.close();
    assert_eq!(report.main_pool, Some(CloseOutcome::TimedOut));
    assert_eq!(report.health_pool, Some(CloseOutcome::TimedOut));
}

#[test]
fn a_panic_of_the_watchdog_thread_shows_in_the_report() {
    let mut host = StubHost::new();
    host.watchdog_panics = true;
    let server = expect_running(&host);
    let report = server.close();
    assert!(report.watchdog_panicked);
}

#[test]
fn a_clean_report_writes_no_line() {
    let logs = LogCapture::start();
    CloseReport {
        main_pool: Some(CloseOutcome::Closed),
        health_pool: Some(CloseOutcome::Closed),
        ..CloseReport::default()
    }
    .log();
    assert!(logs.lines().is_empty(), "{:?}", logs.lines());
}

#[test]
fn the_close_gives_each_pool_that_timed_out_one_warning_that_names_the_pool() {
    let logs = LogCapture::start();
    let mut host = StubHost::new();
    host.pool_outcome = CloseOutcome::TimedOut;
    let server = expect_running(&host);
    let _start_lines = logs.lines();
    let _report = server.close();
    let warnings = logs.at_level("WARN");
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings[0].contains("request pool"), "{warnings:?}");
    assert!(warnings[1].contains("health pool"), "{warnings:?}");
}

#[test]
fn the_report_writes_no_line_for_a_pool_that_timed_out() {
    let logs = LogCapture::start();
    CloseReport {
        main_pool: Some(CloseOutcome::TimedOut),
        health_pool: Some(CloseOutcome::TimedOut),
        ..CloseReport::default()
    }
    .log();
    assert!(logs.lines().is_empty(), "{:?}", logs.lines());
}

#[test]
fn a_cut_gets_one_warning_with_the_three_counts() {
    let logs = LogCapture::start();
    CloseReport {
        cut_public: 4,
        cut_internal: 1,
        cut_jobs: 2,
        ..CloseReport::default()
    }
    .log();
    let warnings = logs.at_level("WARN");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    for text in ["\"cut_public\":4", "\"cut_internal\":1", "\"cut_jobs\":2"] {
        assert!(warnings[0].contains(text), "{text} in {warnings:?}");
    }
}

#[test]
fn the_report_never_writes_an_error_line() {
    let logs = LogCapture::start();
    CloseReport {
        cut_public: 1,
        main_pool: Some(CloseOutcome::TimedOut),
        health_pool: Some(CloseOutcome::TimedOut),
        watchdog_panicked: true,
        ..CloseReport::default()
    }
    .log();
    assert!(logs.at_level("ERROR").is_empty());
}
