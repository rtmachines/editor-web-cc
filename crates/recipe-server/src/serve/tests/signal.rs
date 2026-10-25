//! A stop signal before step 10 ends the start with exit code 0 (ADR-0061 (f)).
//!
//! The stub fires the signal at an exact point. The start must then close what it opened, in the
//! order of the unwind, and report `Interrupted`.

use super::{expect_interrupted, expect_running};
use crate::serve::host::ListenerKind;
use crate::serve::test_support::stub::{Event, Point, Pool, StubHost};
use crate::serve::test_support::{INTERNAL_PORT, LogCapture, PUBLIC_PORT};

/// The events of the unwind with the pools in `pools` open, in order.
fn unwind(pools: &[Pool]) -> Vec<Event> {
    let mut events = vec![Event::Disarm];
    events.extend(pools.iter().map(|pool| Event::PoolClosed(*pool)));
    events.push(Event::WatchdogStop);
    events
}

/// Check that the log ends with the unwind and that the unwind is the last thing that happened.
fn assert_ends_with(host: &StubHost, tail: &[Event]) {
    let events = host.log.events();
    let start = events.len().checked_sub(tail.len()).unwrap();
    assert_eq!(events.get(start..).unwrap(), tail, "{events:?}");
}

fn interrupt_at(point: Point) -> StubHost {
    let host = StubHost::new();
    host.fire.at(point);
    expect_interrupted(&host);
    host
}

#[test]
fn a_signal_while_the_main_pool_opens_unwinds_only_the_watchdog() {
    let host = interrupt_at(Point::OpenMain);
    assert_ends_with(&host, &unwind(&[]));
    assert!(!host.log.contains(&Event::OpenHealth(2)));
}

#[test]
fn a_signal_while_the_health_pool_opens_closes_the_main_pool() {
    let host = interrupt_at(Point::OpenHealth);
    assert_ends_with(&host, &unwind(&[Pool::Main]));
    assert!(!host.log.contains(&Event::RoleCheck));
}

#[test]
fn a_signal_during_the_first_disk_check_closes_both_pools_and_skips_the_comparison() {
    let host = interrupt_at(Point::Disk);
    assert_ends_with(&host, &unwind(&[Pool::Main, Pool::Health]));
    assert!(!host.log.contains(&Event::UnitOfWork));
}

#[test]
fn a_signal_during_the_comparison_of_the_settings_ends_before_the_internal_listener() {
    let host = interrupt_at(Point::UnitOfWork);
    assert_ends_with(&host, &unwind(&[Pool::Main, Pool::Health]));
    assert!(
        !host
            .log
            .contains(&Event::Bind(ListenerKind::Internal, INTERNAL_PORT))
    );
}

#[test]
fn a_signal_while_the_internal_listener_binds_never_arms_the_watchdog() {
    let host = interrupt_at(Point::BindInternal);
    assert_ends_with(&host, &unwind(&[Pool::Main, Pool::Health]));
    assert!(!host.log.contains(&Event::Arm));
}

#[test]
fn a_signal_before_the_loops_start_ends_before_the_public_listener() {
    let host = interrupt_at(Point::StateRead);
    assert_ends_with(&host, &unwind(&[Pool::Main, Pool::Health]));
    assert!(host.log.contains(&Event::Arm));
    assert!(
        !host
            .log
            .contains(&Event::Bind(ListenerKind::Public, PUBLIC_PORT))
    );
}

#[test]
fn a_signal_while_the_public_listener_binds_closes_the_loops_and_the_pools() {
    let host = interrupt_at(Point::BindPublic);
    assert_ends_with(&host, &unwind(&[Pool::Main, Pool::Health]));
    assert!(host.log.contains(&Event::ClockRead));
}

#[test]
fn an_interrupted_start_writes_no_error_line() {
    let logs = LogCapture::start();
    let _host = interrupt_at(Point::BindInternal);
    assert!(logs.at_level("ERROR").is_empty(), "{:?}", logs.lines());
}

#[test]
fn a_signal_after_the_start_ends_the_wait_of_the_running_server() {
    let host = StubHost::new();
    let mut server = expect_running(&host);
    host.fire.now();
    server.wait_for_signal();
    let _report = server.close();
}
