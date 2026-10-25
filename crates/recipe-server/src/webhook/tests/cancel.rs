//! Tests that `run_bounded` cancels its calls (finding CRX-S7-3, ADR-0063).
//!
//! Each call holds a guard. The drop of the guard proves that the runtime dropped the future of
//! the call. The tests use paused time and no socket.

use std::future::pending;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use recipe_app::ports::WebhookFault;

use super::{Call, run_bounded};

/// The number of turns that a test gives the runtime to finish an abort.
const POLL_TURNS: usize = 100;

/// A guard that counts its drops.
struct DropCount(Arc<AtomicUsize>);

impl Drop for DropCount {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// A call that never ends and holds a guard until the runtime drops its future.
fn never_ending(drops: &Arc<AtomicUsize>) -> Call {
    let guard = DropCount(Arc::clone(drops));
    Box::pin(async move {
        let _guard = guard;
        pending::<Result<(), WebhookFault>>().await
    })
}

/// Give the runtime a bounded number of turns, until the count of drops reaches `expected`.
async fn drops_after_polling(drops: &AtomicUsize, expected: usize) -> usize {
    for _turn in 0..POLL_TURNS {
        if drops.load(Ordering::SeqCst) >= expected {
            break;
        }
        tokio::task::yield_now().await;
    }
    drops.load(Ordering::SeqCst)
}

#[tokio::test(start_paused = true)]
async fn run_bounded_drops_the_open_calls_when_the_timeout_ends() {
    let drops = Arc::new(AtomicUsize::new(0));
    let calls: Vec<Call> = (0..3).map(|_| never_ending(&drops)).collect();

    let results = run_bounded(calls, Duration::from_secs(10)).await;

    assert_eq!(results, vec![Err(WebhookFault::Timeout); 3]);
    assert_eq!(
        drops_after_polling(&drops, 3).await,
        3,
        "a call runs on after the timeout"
    );
}

#[tokio::test(start_paused = true)]
async fn run_bounded_drops_only_the_open_call_and_keeps_the_finished_answer() {
    let drops = Arc::new(AtomicUsize::new(0));
    let finished: Call = Box::pin(async { Ok(()) });
    let calls: Vec<Call> = vec![finished, never_ending(&drops)];

    let results = run_bounded(calls, Duration::from_secs(10)).await;

    assert_eq!(results, vec![Ok(()), Err(WebhookFault::Timeout)]);
    assert_eq!(drops_after_polling(&drops, 1).await, 1);
}

#[tokio::test(start_paused = true)]
async fn run_bounded_drops_every_pending_call_when_the_caller_drops_it() {
    let drops = Arc::new(AtomicUsize::new(0));
    let calls: Vec<Call> = (0..3).map(|_| never_ending(&drops)).collect();

    // The caller gives up after 1 s, long before the timeout of 10 s.
    let run = run_bounded(calls, Duration::from_secs(10));
    let outcome = tokio::time::timeout(Duration::from_secs(1), run).await;

    assert!(outcome.is_err(), "the caller must give up first");
    assert_eq!(
        drops_after_polling(&drops, 3).await,
        3,
        "a call runs on after the caller dropped the run"
    );
}

#[tokio::test(start_paused = true)]
async fn run_bounded_keeps_every_call_open_until_the_timeout() {
    let drops = Arc::new(AtomicUsize::new(0));
    let calls: Vec<Call> = (0..3).map(|_| never_ending(&drops)).collect();

    let run = tokio::spawn(run_bounded(calls, Duration::from_secs(10)));

    tokio::time::sleep(Duration::from_secs(9)).await;
    assert_eq!(drops_after_polling(&drops, 1).await, 0, "dropped too early");
    tokio::time::sleep(Duration::from_secs(2)).await;
    let results = run.await.unwrap();

    assert_eq!(results, vec![Err(WebhookFault::Timeout); 3]);
    assert_eq!(drops_after_polling(&drops, 3).await, 3);
}
