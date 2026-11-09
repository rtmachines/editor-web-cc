//! Tests of the free function `read_with_limit` (ADR-0133 (c) to (e), design D10).
//!
//! Each test gives the function a reader that blocks until the test opens a gate. A real limit of
//! some milliseconds makes the call reach the limit, so the tests run on every system.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use parking_lot::{Condvar, Mutex};

use super::{Call, LimitFault, StalledPaths, is_stalled, read_with_limit};

/// A limit that no test call reaches.
const LONG_LIMIT: Duration = Duration::from_secs(30);
/// A real limit of some milliseconds for the tests of a stalled reader.
const SHORT_LIMIT: Duration = Duration::from_millis(40);
/// The time between two checks of a background condition.
const POLL_STEP: Duration = Duration::from_millis(5);
/// The number of checks of a background condition: a wait of about ten seconds.
const POLL_ATTEMPTS: u32 = 2000;
/// The value that a reader gives when it ends.
const READING: u32 = 7;

/// A reader that counts its starts and blocks until the test opens the gate.
struct Probe {
    starts: AtomicUsize,
    open: Mutex<bool>,
    changed: Condvar,
}

impl Probe {
    fn blocked() -> Arc<Self> {
        Arc::new(Self {
            starts: AtomicUsize::new(0),
            open: Mutex::new(false),
            changed: Condvar::new(),
        })
    }

    /// Let every blocked and every later reader end.
    fn release(&self) {
        *self.open.lock() = true;
        self.changed.notify_all();
    }

    fn starts(&self) -> usize {
        self.starts.load(Ordering::SeqCst)
    }

    /// Count the start and wait for the gate.
    fn wait(&self) {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let mut open = self.open.lock();
        while !*open {
            self.changed.wait(&mut open);
        }
    }

    fn read(&self) -> u32 {
        self.wait();
        READING
    }

    /// Wait for the gate, then panic. The panic ends the blocking task.
    fn wait_and_panic(&self) -> u32 {
        self.wait();
        let nothing: Vec<u32> = Vec::new();
        nothing.first().copied().unwrap()
    }
}

/// Wait until `condition` holds. The test fails after `POLL_ATTEMPTS` checks.
async fn wait_until(what: &str, condition: impl Fn() -> bool) {
    for _ in 0..POLL_ATTEMPTS {
        if condition() {
            return;
        }
        tokio::time::sleep(POLL_STEP).await;
    }
    assert!(condition(), "timed out: {what}");
}

fn data_path() -> PathBuf {
    PathBuf::from("data")
}

/// Call the function with a reader that waits for the gate of `probe`.
async fn read_through(
    probe: &Arc<Probe>,
    limit: Duration,
    stalled: &StalledPaths,
) -> Result<u32, LimitFault> {
    let probe = Arc::clone(probe);
    read_with_limit(limit, stalled, data_path(), move |_path| probe.read()).await
}

#[tokio::test]
async fn a_reader_that_ends_in_time_gives_its_value() {
    let probe = Probe::blocked();
    probe.release();
    let stalled = StalledPaths::default();

    let reading = read_through(&probe, LONG_LIMIT, &stalled).await;

    assert_eq!(reading.unwrap(), READING);
    assert!(!is_stalled(&stalled, &data_path()));
}

#[tokio::test]
async fn the_reader_gets_the_path_of_the_call() {
    let stalled = StalledPaths::default();

    let reading = read_with_limit(LONG_LIMIT, &stalled, data_path(), |path| path.to_path_buf())
        .await
        .unwrap();

    assert_eq!(reading, data_path());
}

#[tokio::test]
async fn a_reader_that_blocks_gives_timed_out_and_marks_the_path() {
    let probe = Probe::blocked();
    let stalled = StalledPaths::default();

    let fault = read_through(&probe, SHORT_LIMIT, &stalled)
        .await
        .unwrap_err();
    let marked = is_stalled(&stalled, &data_path());
    probe.release();

    assert!(matches!(fault, LimitFault::TimedOut), "got {fault:?}");
    assert!(marked, "the call that reached the limit left no mark");
    assert_eq!(probe.starts(), 1);
}

#[tokio::test]
async fn a_call_on_a_stalled_path_gives_stalled_at_once_and_starts_no_reader() {
    let probe = Probe::blocked();
    let stalled = StalledPaths::default();
    let first = read_through(&probe, SHORT_LIMIT, &stalled)
        .await
        .unwrap_err();
    assert!(matches!(first, LimitFault::TimedOut), "got {first:?}");

    let started = tokio::time::Instant::now();
    let second = read_through(&probe, SHORT_LIMIT, &stalled)
        .await
        .unwrap_err();
    let waited = started.elapsed();
    probe.release();

    assert!(matches!(second, LimitFault::Stalled), "got {second:?}");
    assert!(waited < SHORT_LIMIT, "the second call waited {waited:?}");
    assert_eq!(probe.starts(), 1, "the second call started a reader");
}

#[tokio::test]
async fn a_call_on_another_path_runs_while_one_path_is_stalled() {
    let probe = Probe::blocked();
    let stalled = StalledPaths::default();
    let first = read_through(&probe, SHORT_LIMIT, &stalled)
        .await
        .unwrap_err();
    assert!(matches!(first, LimitFault::TimedOut), "got {first:?}");

    let other = read_with_limit(LONG_LIMIT, &stalled, PathBuf::from("other"), |_path| {
        READING
    })
    .await;
    probe.release();

    assert_eq!(other.unwrap(), READING);
}

#[tokio::test]
async fn a_call_after_the_end_of_the_first_reader_runs_a_new_reader() {
    let probe = Probe::blocked();
    let stalled = StalledPaths::default();
    let first = read_through(&probe, SHORT_LIMIT, &stalled)
        .await
        .unwrap_err();
    assert!(matches!(first, LimitFault::TimedOut), "got {first:?}");

    probe.release();
    wait_until("the mark of the ended reader clears", || {
        !is_stalled(&stalled, &data_path())
    })
    .await;
    let second = read_through(&probe, SHORT_LIMIT, &stalled).await;

    assert_eq!(second.unwrap(), READING);
    assert_eq!(probe.starts(), 2);
}

#[tokio::test]
async fn two_concurrent_fast_calls_of_one_path_both_give_a_reading() {
    let probe = Probe::blocked();
    let stalled = StalledPaths::default();

    let (first, second, ()) = tokio::join!(
        read_through(&probe, LONG_LIMIT, &stalled),
        read_through(&probe, LONG_LIMIT, &stalled),
        async {
            wait_until("both readers started", || probe.starts() == 2).await;
            probe.release();
        }
    );

    assert_eq!(first.unwrap(), READING);
    assert_eq!(second.unwrap(), READING);
    assert_eq!(probe.starts(), 2);
}

#[tokio::test]
async fn a_reader_that_ends_at_the_limit_leaves_no_mark() {
    let stalled = StalledPaths::default();
    let call = Call::new(data_path(), Arc::clone(&stalled));

    // The reader ends first. The timeout branch looks at the state afterwards.
    call.finish();
    call.mark_if_running();

    assert!(
        !is_stalled(&stalled, &data_path()),
        "the ended call left a mark"
    );
}

#[tokio::test]
async fn a_running_call_that_reaches_the_limit_leaves_a_mark_until_it_ends() {
    let stalled = StalledPaths::default();
    let call = Call::new(data_path(), Arc::clone(&stalled));

    call.mark_if_running();
    assert!(is_stalled(&stalled, &data_path()));

    call.finish();
    assert!(!is_stalled(&stalled, &data_path()));
}

#[tokio::test]
async fn a_call_that_did_not_mark_the_path_leaves_the_mark_of_a_running_call() {
    let stalled = StalledPaths::default();
    let marking = Call::new(data_path(), Arc::clone(&stalled));
    let unmarked = Call::new(data_path(), Arc::clone(&stalled));
    marking.mark_if_running();

    // The second call ends without a mark of its own.
    unmarked.finish();

    assert!(
        is_stalled(&stalled, &data_path()),
        "the end of an unmarked call took away the mark of another call"
    );
    marking.finish();
    assert!(!is_stalled(&stalled, &data_path()));
}

#[tokio::test]
async fn two_marks_on_one_path_clear_one_by_one() {
    let stalled = StalledPaths::default();
    let first = Call::new(data_path(), Arc::clone(&stalled));
    let second = Call::new(data_path(), Arc::clone(&stalled));
    first.mark_if_running();
    second.mark_if_running();

    first.finish();
    assert!(
        is_stalled(&stalled, &data_path()),
        "the path must stay stalled while one marking call runs"
    );
    second.finish();
    assert!(!is_stalled(&stalled, &data_path()));
}

#[tokio::test]
async fn a_reader_that_panics_gives_join_and_leaves_no_mark() {
    let stalled = StalledPaths::default();
    let probe = Probe::blocked();
    probe.release();
    let reader_probe = Arc::clone(&probe);

    let fault = read_with_limit(LONG_LIMIT, &stalled, data_path(), move |_path| {
        reader_probe.wait_and_panic()
    })
    .await
    .unwrap_err();

    assert!(matches!(fault, LimitFault::Join(_)), "got {fault:?}");
    assert!(!is_stalled(&stalled, &data_path()));
}

#[tokio::test]
async fn a_reader_that_panics_after_the_limit_clears_its_mark() {
    let stalled = StalledPaths::default();
    let probe = Probe::blocked();
    let reader_probe = Arc::clone(&probe);

    let fault = read_with_limit(SHORT_LIMIT, &stalled, data_path(), move |_path| {
        reader_probe.wait_and_panic()
    })
    .await
    .unwrap_err();
    assert!(matches!(fault, LimitFault::TimedOut), "got {fault:?}");
    assert!(is_stalled(&stalled, &data_path()));

    probe.release();
    wait_until("the mark of the panicked reader clears", || {
        !is_stalled(&stalled, &data_path())
    })
    .await;
}
