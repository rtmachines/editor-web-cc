//! Test that `run_bounded` uses ONE timeout for the whole run (ADR-0063).
//!
//! A mutant that starts a new timeout for each `join_next` lets the second call finish after
//! 12 s with a timeout of 10 s. The test uses paused time and no socket.

use std::time::Duration;

use recipe_app::ports::WebhookFault;
use tokio::time::Instant;

use super::{Call, after, run_bounded};

#[tokio::test(start_paused = true)]
async fn run_bounded_applies_one_timeout_to_the_whole_run_and_not_one_to_each_call() {
    let calls: Vec<Call> = vec![
        after(Duration::from_secs(5), Ok(())),
        after(Duration::from_secs(12), Ok(())),
    ];
    let started = Instant::now();

    let results = run_bounded(calls, Duration::from_secs(10)).await;

    assert_eq!(results, vec![Ok(()), Err(WebhookFault::Timeout)]);
    assert!(
        started.elapsed() >= Duration::from_secs(10),
        "the run ended before the timeout"
    );
    assert!(
        started.elapsed() < Duration::from_secs(11),
        "the run waited longer than one timeout"
    );
}
