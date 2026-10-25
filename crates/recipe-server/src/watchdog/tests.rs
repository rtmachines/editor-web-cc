//! Tests of the failure window and of the watchdog thread (design D14 row T18).
//!
//! The window tests inject the instants and never sleep. The thread tests use intervals of a few
//! milliseconds and wait only on bounded channels. No test depends on the speed of a thread: a
//! command waits in the queue before a gate opens, or the gate closes before the last stop. The exit code of the real thread has its own
//! test in `tests/watchdog_exit.rs`, because a call of `std::process::exit` ends the test binary.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::{
    FailureVerdict, FailureWindow, Message, STALLED_EXIT_CODE, TripReport, Watchdog,
    WatchdogSettings,
};
use crate::probe::{DEFAULT_INTERNAL_PORT, PROBE_BUDGET, ProbeFault};

const RELIABILITY_MD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/reliability.md"
);

/// The longest wait of a test for an event of the watchdog thread.
const EVENT_WAIT: Duration = Duration::from_secs(10);

/// Return the value cell of the key in the table of `reliability.md` section 7.11.
///
/// A row can hold several keys and several values, separated by ` / `.
fn table_value(key: &str) -> String {
    let document = std::fs::read_to_string(RELIABILITY_MD).unwrap();
    let (_, table) = document.split_once("### 7.11 Settings").unwrap();
    table
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let keys = cells.get(1)?;
            let values = cells.get(2)?;
            let position = keys
                .split(" / ")
                .position(|name| name.trim_matches('`') == key)?;
            values.split(" / ").nth(position).map(str::to_owned)
        })
        .next()
        .unwrap_or_else(|| String::from("<key not found>"))
}

/// Read a duration such as `10 s`. A cell can continue with a sentence after the unit.
fn duration_of(text: &str) -> Duration {
    let mut words = text.split_whitespace();
    let number: u64 = words.next().unwrap().parse().unwrap();
    let unit = words.next().unwrap().trim_end_matches('.');
    assert_eq!(unit, "s", "the test reads seconds only: {text}");
    Duration::from_secs(number)
}

fn count_of(text: &str) -> u64 {
    let word = text.split_whitespace().next().unwrap();
    word.trim_end_matches('.').parse().unwrap()
}

fn after(base: Instant, seconds: u64) -> Instant {
    base.checked_add(Duration::from_secs(seconds)).unwrap()
}

fn window_of_three_in_30_s() -> FailureWindow {
    FailureWindow::new(3, Duration::from_secs(30))
}

#[test]
fn defaults_equal_the_settings_table() {
    let settings = WatchdogSettings::default();

    assert_eq!(
        settings.interval,
        duration_of(&table_value("watchdog.probe_interval"))
    );
    assert_eq!(
        u64::from(settings.failures),
        count_of(&table_value("watchdog.failures"))
    );
    assert_eq!(
        settings.window,
        duration_of(&table_value("watchdog.window"))
    );
    assert_eq!(
        PROBE_BUDGET,
        duration_of(&table_value("watchdog.probe_timeout"))
    );
    assert_eq!(
        u64::from(DEFAULT_INTERNAL_PORT),
        count_of(&table_value("http.internal_port"))
    );
}

#[test]
fn the_probe_budget_is_above_the_readiness_check_timeout() {
    let readiness_check = duration_of(&table_value("health.db_check_timeout"));

    assert!(PROBE_BUDGET > readiness_check);
}

#[test]
fn the_exit_code_of_a_stalled_process_is_70() {
    assert_eq!(STALLED_EXIT_CODE, 70);
}

#[test]
fn failures_below_the_limit_are_tolerated_and_the_limit_trips() {
    let base = Instant::now();
    let mut window = window_of_three_in_30_s();

    let first = window.record(after(base, 0));
    let second = window.record(after(base, 10));
    let third = window.record(after(base, 20));

    assert_eq!(first, FailureVerdict::Tolerated);
    assert_eq!(second, FailureVerdict::Tolerated);
    assert_eq!(third, FailureVerdict::Tripped);
}

#[test]
fn failures_that_are_further_apart_than_the_window_do_not_trip() {
    let base = Instant::now();
    let mut window = window_of_three_in_30_s();

    let verdicts = [0, 20, 40, 60, 80].map(|second| window.record(after(base, second)));

    assert!(
        verdicts
            .iter()
            .all(|verdict| *verdict == FailureVerdict::Tolerated)
    );
}

#[test]
fn a_failure_exactly_one_window_old_still_counts() {
    let base = Instant::now();
    let mut window = window_of_three_in_30_s();

    window.record(after(base, 0));
    window.record(after(base, 10));
    let verdict = window.record(after(base, 30));

    assert_eq!(verdict, FailureVerdict::Tripped);
}

#[test]
fn a_failure_just_over_one_window_old_is_forgotten() {
    let base = Instant::now();
    let mut window = window_of_three_in_30_s();
    let late = after(base, 30)
        .checked_add(Duration::from_millis(1))
        .unwrap();

    window.record(after(base, 0));
    window.record(after(base, 10));
    let verdict = window.record(late);

    assert_eq!(verdict, FailureVerdict::Tolerated);
    assert_eq!(window.count(), 2);
}

#[test]
fn clear_forgets_every_failure() {
    let base = Instant::now();
    let mut window = window_of_three_in_30_s();
    window.record(after(base, 0));
    window.record(after(base, 1));

    window.clear();
    let verdict = window.record(after(base, 2));

    assert_eq!(verdict, FailureVerdict::Tolerated);
    assert_eq!(window.count(), 1);
}

#[test]
fn the_window_keeps_at_most_the_limit_of_failures() {
    let base = Instant::now();
    let mut window = window_of_three_in_30_s();

    let verdicts: Vec<FailureVerdict> = (0..100)
        .map(|second| window.record(after(base, second)))
        .collect();

    assert_eq!(window.count(), 3);
    assert_eq!(verdicts.last(), Some(&FailureVerdict::Tripped));
}

#[test]
fn a_limit_of_zero_acts_like_a_limit_of_one() {
    let mut window = FailureWindow::new(0, Duration::from_secs(30));

    assert_eq!(window.record(Instant::now()), FailureVerdict::Tripped);
}

#[test]
fn an_instant_before_an_earlier_one_does_not_panic() {
    let base = Instant::now();
    let mut window = window_of_three_in_30_s();

    window.record(after(base, 10));
    let verdict = window.record(after(base, 0));

    assert_eq!(verdict, FailureVerdict::Tolerated);
    assert_eq!(window.count(), 2);
}

fn fast_settings() -> WatchdogSettings {
    WatchdogSettings {
        interval: Duration::from_millis(5),
        failures: 3,
        window: Duration::from_secs(30),
    }
}

/// Start a watchdog with a fake probe that counts its calls and returns the given result.
fn start_with_fake(
    settings: WatchdogSettings,
    outcome: Result<(), ProbeFault>,
) -> (Watchdog, Arc<AtomicU32>, Receiver<TripReport>) {
    let calls = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&calls);
    let (sender, receiver): (Sender<TripReport>, Receiver<TripReport>) = mpsc::channel();
    let watchdog = Watchdog::start_with(
        settings,
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            outcome.clone()
        },
        move |report| {
            sender.send(report).unwrap();
        },
    )
    .unwrap();
    (watchdog, calls, receiver)
}

/// Run `stop` on a helper thread. The receiver gets `true` when `stop` returned `Ok`.
fn stop_on_helper(watchdog: Watchdog) -> (Receiver<bool>, JoinHandle<()>) {
    let (done_sender, done) = mpsc::channel();
    let helper = std::thread::spawn(move || {
        done_sender.send(watchdog.stop().is_ok()).ok();
    });
    (done, helper)
}

/// Stop the watchdog and fail the test when `stop` does not return in `EVENT_WAIT`.
fn stop_bounded(watchdog: Watchdog) {
    let (done, helper) = stop_on_helper(watchdog);
    assert_eq!(
        done.recv_timeout(EVENT_WAIT),
        Ok(true),
        "stop did not end the thread in time"
    );
    helper.join().unwrap();
}

fn wait_for(calls: &AtomicU32, at_least: u32) {
    let deadline = Instant::now().checked_add(EVENT_WAIT).unwrap();
    while calls.load(Ordering::SeqCst) < at_least {
        assert!(Instant::now() < deadline, "the probe ran too seldom");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn a_watchdog_that_is_not_armed_never_probes() {
    let (watchdog, calls, trips) = start_with_fake(fast_settings(), Err(ProbeFault::Timeout));

    std::thread::sleep(Duration::from_millis(80));
    stop_bounded(watchdog);

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(trips.try_recv().is_err());
}

#[test]
fn an_armed_watchdog_trips_after_the_failure_limit_and_stops_probing() {
    let (watchdog, calls, trips) = start_with_fake(fast_settings(), Err(ProbeFault::Timeout));

    watchdog.arm();
    let report = trips.recv_timeout(EVENT_WAIT).unwrap();
    stop_bounded(watchdog);

    assert_eq!(report.failures, 3);
    assert_eq!(report.window, Duration::from_secs(30));
    assert_eq!(report.last_fault, ProbeFault::Timeout);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[test]
fn a_healthy_probe_never_trips() {
    let (watchdog, calls, trips) = start_with_fake(fast_settings(), Ok(()));

    watchdog.arm();
    wait_for(&calls, 5);
    stop_bounded(watchdog);

    assert!(trips.try_recv().is_err());
}

#[test]
fn disarm_stops_the_probes() {
    let (watchdog, calls, trips) = start_with_fake(fast_settings(), Ok(()));
    watchdog.arm();
    wait_for(&calls, 2);

    watchdog.disarm();
    std::thread::sleep(Duration::from_millis(40));
    let settled = calls.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(60));
    let later = calls.load(Ordering::SeqCst);
    stop_bounded(watchdog);

    assert_eq!(settled, later);
    assert!(trips.try_recv().is_err());
}

#[test]
fn arm_after_disarm_probes_again() {
    let (watchdog, calls, _trips) = start_with_fake(fast_settings(), Ok(()));
    watchdog.arm();
    wait_for(&calls, 1);
    watchdog.disarm();
    std::thread::sleep(Duration::from_millis(40));
    let before = calls.load(Ordering::SeqCst);

    watchdog.arm();
    wait_for(&calls, before.checked_add(2).unwrap());
    stop_bounded(watchdog);
}

#[test]
fn stop_ends_a_thread_that_waits_for_a_long_interval() {
    let settings = WatchdogSettings {
        interval: Duration::from_secs(600),
        failures: 3,
        window: Duration::from_secs(1200),
    };
    let (watchdog, calls, trips) = start_with_fake(settings, Err(ProbeFault::Timeout));
    watchdog.arm();

    let started = Instant::now();
    stop_bounded(watchdog);
    let elapsed = started.elapsed();

    assert!(elapsed < Duration::from_secs(5), "stop took {elapsed:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(trips.try_recv().is_err());
}

/// A watchdog with a probe that blocks on a gate until the test sends the result.
struct GatedWatchdog {
    watchdog: Watchdog,
    started: Receiver<()>,
    gate: Sender<Result<(), ProbeFault>>,
    trips: Receiver<TripReport>,
}

/// Start a watchdog that trips at the first failure. Each probe reports its start on `started`.
fn start_gated() -> GatedWatchdog {
    let settings = WatchdogSettings {
        interval: Duration::from_millis(5),
        failures: 1,
        window: Duration::from_secs(30),
    };
    let (started_sender, started) = mpsc::channel();
    let (gate, gate_receiver) = mpsc::channel();
    let (trip_sender, trips) = mpsc::channel();
    let watchdog = Watchdog::start_with(
        settings,
        move || {
            started_sender.send(()).ok();
            gate_receiver.recv_timeout(EVENT_WAIT).unwrap_or(Ok(()))
        },
        move |report| {
            trip_sender.send(report).unwrap();
        },
    )
    .unwrap();
    GatedWatchdog {
        watchdog,
        started,
        gate,
        trips,
    }
}

/// The test thread queues `Stop` while the probe blocks. Thus the command waits in the queue
/// before the gate opens, and no timing decides the order.
#[test]
fn stop_during_a_probe_ends_the_thread_without_a_trip() {
    let gated = start_gated();
    gated.watchdog.arm();
    gated.started.recv_timeout(EVENT_WAIT).unwrap();
    gated.watchdog.messages.send(Message::Stop).unwrap();

    gated.gate.send(Err(ProbeFault::Timeout)).unwrap();
    stop_bounded(gated.watchdog);

    assert!(gated.trips.try_recv().is_err());
}

/// The gate closes after the check, so the probe returns at once and the thread never blocks
/// again, however late the `Stop` of the helper thread comes.
#[test]
fn a_stop_that_waits_for_a_blocked_probe_is_seen_as_late() {
    let gated = start_gated();
    gated.watchdog.arm();
    gated.started.recv_timeout(EVENT_WAIT).unwrap();

    let (done, helper) = stop_on_helper(gated.watchdog);
    let early = done.recv_timeout(Duration::from_millis(100));
    drop(gated.gate);

    assert_eq!(early, Err(mpsc::RecvTimeoutError::Timeout));
    assert_eq!(done.recv_timeout(EVENT_WAIT), Ok(true));
    helper.join().unwrap();
}

#[test]
fn disarm_during_a_probe_drops_the_failed_result() {
    let gated = start_gated();
    gated.watchdog.arm();
    gated.started.recv_timeout(EVENT_WAIT).unwrap();
    gated.watchdog.disarm();

    gated.gate.send(Err(ProbeFault::Timeout)).unwrap();
    gated.watchdog.arm();

    gated
        .started
        .recv_timeout(EVENT_WAIT)
        .expect("the watchdog probes again after the new arm");
    assert!(gated.trips.try_recv().is_err());
    drop(gated.gate);
    stop_bounded(gated.watchdog);
    assert!(gated.trips.try_recv().is_err());
}

#[test]
fn a_dropped_handle_ends_the_thread() {
    let (watchdog, calls, _trips) = start_with_fake(fast_settings(), Ok(()));
    watchdog.arm();
    wait_for(&calls, 1);

    drop(watchdog);
    std::thread::sleep(Duration::from_millis(60));
    let settled = calls.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(60));

    assert_eq!(calls.load(Ordering::SeqCst), settled);
}
