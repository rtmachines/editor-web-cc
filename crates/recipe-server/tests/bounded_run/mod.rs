#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Helpers that several test crates of `recipe-server` share.

use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const POLL_PAUSE: Duration = Duration::from_millis(50);

/// Run the prepared command with no standard input and wait for it, at most `limit`.
///
/// When the limit ends, the function kills the child and fails the test. Thus a fault cannot hang
/// the test run.
pub fn run_bounded(mut command: Command, limit: Duration) -> Output {
    let mut child: Child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now().checked_add(limit).unwrap();
    let mut killed = false;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            killed = true;
            break;
        }
        std::thread::sleep(POLL_PAUSE);
    }
    let output = child.wait_with_output().unwrap();
    assert!(!killed, "the child ran longer than {limit:?}");
    output
}
