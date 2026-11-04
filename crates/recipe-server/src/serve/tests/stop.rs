//! The stop of a running server (design D4, ADR-0087, ADR-0088, ADR-0089, ADR-0090).
//!
//! The tests run the stop on a stub host. They read the order of the steps from the event log. The
//! observer of the stub adds the events that no port shows: readiness off and a closed listener.

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use recipe_app::operations::AlertCounters;
use recipe_db::CloseOutcome;

use super::expect_running;
use super::ready::{get, status};
use crate::serve::host::ListenerKind;
use crate::serve::process::{AcceptLoopEnded, exit_code, serve_with};
use crate::serve::server::CloseReport;
use crate::serve::test_support::stub::{Event, Fire, LoopWay, Pool, StubHost};
use crate::serve::test_support::{self, LogCapture};

/// The time that a signal thread waits before it fires. It lets the main thread reach its wait.
const FIRE_DELAY: Duration = Duration::from_millis(50);
/// The time after which a signal thread gives up. A hang then fails the test.
const READY_LIMIT: Duration = Duration::from_secs(30);

/// The time that a second wait may take. A longer wait means that it blocks.
const SECOND_WAIT_LIMIT: Duration = Duration::from_secs(5);

/// The text of the INFO line that ends a stop.
const STOPPED: &str = "the server stopped";

/// Fire the signal when the thread ends, also after a panic.
///
/// The guard solves one problem: a thread that panics before it fires leaves `serve_with` in its
/// wait for ever, so the test hangs and does not fail.
struct FireOnDrop(Fire);

impl Drop for FireOnDrop {
    fn drop(&mut self) {
        self.0.now();
    }
}

/// A thread that acts after the server is ready and then waits for the end of the test run.
struct ActionThread {
    handle: thread::JoinHandle<()>,
    done: mpsc::Sender<()>,
}

impl ActionThread {
    /// Tell the thread that `serve_with` returned, so it ends at once, and wait for it.
    ///
    /// The thread has a long wait as a safety net. Without this call, a passing test would wait for
    /// the full delay.
    fn finish(self) {
        // The send fails only when the thread already ended. Then the join gives the result.
        let _ended = self.done.send(());
        self.handle.join().unwrap();
    }
}

/// Fire the signal of `host` after the server is ready. The start must have ended before.
fn fire_when_ready(host: &StubHost) -> ActionThread {
    act_when_ready(host, || {}, FIRE_DELAY)
}

/// Run `action` after the server is ready. Fire the signal `delay` later or at `finish`.
///
/// The thread fires the signal also after a panic, so a failed run does not hang.
fn act_when_ready(
    host: &StubHost,
    action: impl FnOnce() + Send + 'static,
    delay: Duration,
) -> ActionThread {
    let (done, finished) = mpsc::channel();
    let log = host.log.clone();
    let ports = host.ports.clone();
    let fire = FireOnDrop(host.fire.clone());
    let handle = thread::spawn(move || {
        let _fire = fire;
        let begin = Instant::now();
        let ready = loop {
            let port = ports.of(ListenerKind::Internal);
            let answered = port.is_some_and(|port| status(&get(port, "/readyz")) == 200);
            if answered || begin.elapsed() > READY_LIMIT {
                break answered;
            }
            thread::sleep(Duration::from_millis(5));
        };
        assert!(ready, "the server never became ready: {:?}", log.events());
        action();
        // A message or a closed channel ends the wait. Only a timeout means that nobody finished.
        let _finished = finished.recv_timeout(delay);
    });
    ActionThread { handle, done }
}

#[test]
fn the_stop_follows_the_order_of_the_design() {
    let host = StubHost::new();
    let server = expect_running(&host);
    host.watch.enable(server.health().clone());
    let before = host.log.events().len();
    let report = server.close();
    let events = host.log.events();
    assert_eq!(
        events.get(before..).unwrap(),
        [
            Event::ReadinessOff,
            Event::Disarm,
            Event::ListenerStopped(ListenerKind::Public),
            Event::PoolClosed(Pool::Main),
            Event::PoolClosed(Pool::Health),
            Event::ListenerStopped(ListenerKind::Internal),
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
fn the_wait_blocks_until_the_signal_fires() {
    let mut host = StubHost::new();
    host.realtime = true;
    let mut server = expect_running(&host);
    let fired = Arc::new(AtomicBool::new(false));
    let (flag, fire) = (Arc::clone(&fired), host.fire.clone());
    let sender = thread::spawn(move || {
        thread::sleep(FIRE_DELAY);
        flag.store(true, Ordering::SeqCst);
        fire.now();
    });
    server.wait_for_signal();
    assert!(
        fired.load(Ordering::SeqCst),
        "the wait ended before the signal"
    );
    sender.join().unwrap();
    let _report = server.close();
}

#[test]
fn a_second_wait_after_the_signal_returns_at_once() {
    let host = StubHost::new();
    let mut server = expect_running(&host);
    host.fire.now();
    server.wait_for_signal();
    let (done, finished) = mpsc::channel();
    let second = thread::spawn(move || {
        server.wait_for_signal();
        // The test thread ignores the message after its limit, so a failed send is harmless.
        let _ = done.send(());
        server
    });
    finished
        .recv_timeout(SECOND_WAIT_LIMIT)
        .expect("the second wait blocks after the signal");
    let _report = second.join().unwrap().close();
}

#[test]
fn a_clean_stop_writes_one_info_line_and_no_warning() {
    let logs = LogCapture::start();
    let host = StubHost::new();
    let server = expect_running(&host);
    let _start_lines = logs.lines();
    let report = server.stop();
    assert_eq!(report.cut_public + report.cut_internal + report.cut_jobs, 0);
    let lines = logs.lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("\"level\":\"INFO\""), "{lines:?}");
    assert!(lines[0].contains(STOPPED), "{lines:?}");
}

#[test]
fn a_pool_that_times_out_gets_its_warning_before_the_info_line_of_the_stop() {
    let logs = LogCapture::start();
    let mut host = StubHost::new();
    host.pool_outcome = CloseOutcome::TimedOut;
    let server = expect_running(&host);
    let _start_lines = logs.lines();
    let report = server.stop();
    assert_eq!(report.main_pool, Some(CloseOutcome::TimedOut));
    let lines = logs.lines();
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines[0].contains("request pool"), "{lines:?}");
    assert!(lines[1].contains("health pool"), "{lines:?}");
    assert!(lines[2].contains(STOPPED), "{lines:?}");
    assert!(
        lines[..2]
            .iter()
            .all(|line| line.contains("\"level\":\"WARN\""))
    );
}

#[test]
fn serve_stops_after_the_signal_and_ends_with_exit_code_zero_also_after_a_pool_timeout() {
    let logs = LogCapture::start();
    let mut host = StubHost::new();
    host.realtime = true;
    host.pool_outcome = CloseOutcome::TimedOut;
    let sender = fire_when_ready(&host);
    let result = serve_with(&host, test_support::plan(), AlertCounters::default());
    sender.finish();
    assert!(result.is_ok(), "{result:?}");
    let events = host.log.events();
    assert_eq!(events.last(), Some(&Event::WatchdogStop), "{events:?}");
    assert_eq!(host.log.count(&Event::PoolClosed(Pool::Main)), 1);
    assert_eq!(host.log.count(&Event::PoolClosed(Pool::Health)), 1);
    let lines = logs.lines();
    let stopped: Vec<_> = lines.iter().filter(|line| line.contains(STOPPED)).collect();
    assert_eq!(stopped.len(), 1, "{lines:?}");
    let errors: Vec<_> = lines
        .iter()
        .filter(|line| line.contains("\"level\":\"ERROR\""))
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}

/// The text of the `WARN` line that the join of a loop without a result writes today.
const NO_DRAIN_RESULT: &str = "an accept loop ended without a drain result";

#[test]
fn a_public_loop_that_returns_ends_the_wait_and_the_stop_keeps_its_order() {
    let host = StubHost::new();
    host.loop_end.at(ListenerKind::Public, LoopWay::Return);
    let mut server = expect_running(&host);
    host.watch.enable(server.health().clone());
    let before = host.log.events().len();
    // The guard fires the signal late, so a wait that sees only the signal fails and does not hang.
    let guard = FireOnDrop(host.fire.clone());
    thread::spawn(move || {
        let _guard = guard;
        thread::sleep(2 * SECOND_WAIT_LIMIT);
    });
    host.loop_end.now();
    let begin = Instant::now();
    server.wait_for_signal();
    let waited = begin.elapsed();
    assert!(
        waited < SECOND_WAIT_LIMIT,
        "the wait ignored the end of the public loop and took {waited:?}"
    );
    let _report = server.close();
    let events: Vec<_> = host
        .log
        .events()
        .into_iter()
        .skip(before)
        .filter(|event| *event != Event::ListenerStopped(ListenerKind::Public))
        .collect();
    assert_eq!(
        events,
        [
            Event::ReadinessOff,
            Event::Disarm,
            Event::PoolClosed(Pool::Main),
            Event::PoolClosed(Pool::Health),
            Event::ListenerStopped(ListenerKind::Internal),
            Event::WatchdogStop,
        ]
    );
}

#[test]
fn serve_with_returns_an_error_that_names_the_public_listener_when_its_loop_returns() {
    let logs = LogCapture::start();
    let mut host = StubHost::new();
    host.realtime = true;
    host.loop_end.at(ListenerKind::Public, LoopWay::Return);
    let end = host.loop_end.clone();
    let sender = act_when_ready(&host, move || end.now(), SECOND_WAIT_LIMIT);
    let result = serve_with(&host, test_support::plan(), AlertCounters::default());
    sender.finish();
    let error = result.expect_err("the end of the public loop must give an error");
    assert_eq!(
        error.downcast_ref::<AcceptLoopEnded>(),
        Some(&AcceptLoopEnded {
            kind: ListenerKind::Public,
            panicked: false,
        })
    );
    assert_eq!(
        error.to_string(),
        "the accept loop of the public listener ended before the stop signal without a panic"
    );
    assert_eq!(exit_code(&error), Some(ExitCode::from(70)));
    let lines = logs.lines();
    assert!(
        lines.iter().all(|line| !line.contains(NO_DRAIN_RESULT)),
        "{lines:?}"
    );
}

#[test]
fn an_internal_loop_that_panics_ends_the_wait_and_the_stop_keeps_its_order() {
    let host = StubHost::new();
    host.loop_end.at(ListenerKind::Internal, LoopWay::Panic);
    let mut server = expect_running(&host);
    host.watch.enable(server.health().clone());
    let before = host.log.events().len();
    // The guard fires the signal late, so a wait that sees only the signal fails and does not hang.
    let guard = FireOnDrop(host.fire.clone());
    thread::spawn(move || {
        let _guard = guard;
        thread::sleep(2 * SECOND_WAIT_LIMIT);
    });
    host.loop_end.now();
    let begin = Instant::now();
    server.wait_for_signal();
    let waited = begin.elapsed();
    assert!(
        waited < SECOND_WAIT_LIMIT,
        "the wait ignored the panic of the internal loop and took {waited:?}"
    );
    // The close ends only if the stop takes the `JoinError` once. A second await of the finished
    // task panics in Tokio.
    let _report = server.close();
    let events: Vec<_> = host
        .log
        .events()
        .into_iter()
        .skip(before)
        .filter(|event| *event != Event::ListenerStopped(ListenerKind::Internal))
        .collect();
    assert_eq!(
        events,
        [
            Event::ReadinessOff,
            Event::Disarm,
            Event::ListenerStopped(ListenerKind::Public),
            Event::PoolClosed(Pool::Main),
            Event::PoolClosed(Pool::Health),
            Event::WatchdogStop,
        ]
    );
}

#[test]
fn serve_with_returns_an_error_that_names_the_internal_listener_and_a_panic_and_writes_no_line() {
    let logs = LogCapture::start();
    let mut host = StubHost::new();
    host.realtime = true;
    host.loop_end.at(ListenerKind::Internal, LoopWay::Panic);
    let end = host.loop_end.clone();
    let sender = act_when_ready(&host, move || end.now(), SECOND_WAIT_LIMIT);
    let result = serve_with(&host, test_support::plan(), AlertCounters::default());
    sender.finish();
    let error = result.expect_err("the panic of the internal loop must give an error");
    assert_eq!(
        error.downcast_ref::<AcceptLoopEnded>(),
        Some(&AcceptLoopEnded {
            kind: ListenerKind::Internal,
            panicked: true,
        })
    );
    assert_eq!(
        error.to_string(),
        "the accept loop of the internal listener ended before the stop signal with a panic"
    );
    assert_eq!(exit_code(&error), Some(ExitCode::from(70)));
    let lines = logs.lines();
    let faults: Vec<_> = lines
        .iter()
        .filter(|line| line.contains("\"level\":\"WARN\"") || line.contains("\"level\":\"ERROR\""))
        .collect();
    assert!(faults.is_empty(), "{faults:?}");
}
