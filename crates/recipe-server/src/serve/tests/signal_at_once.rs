//! A stop signal that arrives while a step ends in the same poll ends the start (ADR-0061 (f)).
//!
//! The stub fires the signal inside step 9 and the step returns at once. The step never yields,
//! so the start must see the signal by itself. ADR-0061 says that a signal before step 10 ends
//! the start with exit code 0.

use recipe_app::memory::{Fault, OperationsMethod};

use super::expect_interrupted;
use crate::serve::host::ListenerKind;
use crate::serve::test_support::stub::{Event, Point, Pool, StubHost};
use crate::serve::test_support::{LogCapture, PUBLIC_PORT};

const STARTED: &str = "the server started";

/// Check the unwind of a start that had both pools open, and that no public listener bound.
fn assert_interrupted_before_step_10(host: &StubHost) {
    let events = host.log.events();
    let tail = [
        Event::Disarm,
        Event::PoolClosed(Pool::Main),
        Event::PoolClosed(Pool::Health),
        Event::WatchdogStop,
    ];
    let start = events.len().checked_sub(tail.len()).unwrap();
    assert_eq!(events.get(start..).unwrap(), tail, "{events:?}");
    assert!(
        !host
            .log
            .contains(&Event::Bind(ListenerKind::Public, PUBLIC_PORT))
    );
    assert_eq!(host.ports.of(ListenerKind::Public), None);
}

#[test]
fn a_signal_in_a_step_that_returns_at_once_ends_the_start_before_step_10() {
    let logs = LogCapture::start();
    let host = StubHost::new();
    host.fire.at_once_at(Point::StateRead);
    expect_interrupted(&host);
    assert_interrupted_before_step_10(&host);
    let lines = logs.lines();
    assert!(
        lines.iter().all(|line| !line.contains(STARTED)),
        "{lines:?}"
    );
}

#[test]
fn a_signal_in_a_step_that_fails_at_once_gives_the_exit_of_the_signal() {
    let logs = LogCapture::start();
    let host = StubHost::new();
    host.operations
        .fail_on_call(OperationsMethod::State, 1, Fault::Unavailable);
    host.fire.at_once_at(Point::StateRead);
    expect_interrupted(&host);
    assert_interrupted_before_step_10(&host);
    assert!(logs.at_level("ERROR").is_empty(), "{:?}", logs.lines());
}
