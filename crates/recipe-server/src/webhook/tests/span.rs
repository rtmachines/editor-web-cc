//! Test that a webhook call keeps the span `job` of the caller (`reliability.md` section 3.2).
//!
//! `run_bounded` runs each call in its own task. A spawned task does not inherit the current
//! span, so the call must carry it. The panic hook of `panic_hook.rs` is a process-wide hook, so
//! a unit test does not install it. The hook writes with `tracing::error!` in the span of the
//! panicking thread. The test call writes the same kind of event just before it panics.

use std::io;
use std::sync::LazyLock;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Instrument};

use super::{Call, broken_step, run_bounded};
use crate::logging;

/// The value of the field `job` that the caller sets.
const JOB: &str = "alerts";

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first. A test thread
/// without a subscriber then caches `Interest::never` for all threads.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

/// A writer that sends each write to a channel.
struct ChannelWriter(Sender<Vec<u8>>);

impl io::Write for ChannelWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        // A closed channel means that the test is over. The log line is then not needed.
        let _ = self.0.send(buffer.to_vec());
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Install a JSON subscriber for the current thread. Return the guard and the line receiver.
fn capture() -> (DefaultGuard, Receiver<Vec<u8>>) {
    let (sender, lines) = channel();
    let filter = logging::build_filter(Some("info")).unwrap();
    let subscriber = logging::subscriber(filter, move || ChannelWriter(sender.clone()));
    LazyLock::force(&KEEPALIVE);
    (tracing::subscriber::set_default(subscriber), lines)
}

/// Every line that the receiver holds.
fn lines_of(receiver: &Receiver<Vec<u8>>) -> Vec<String> {
    let mut text = Vec::new();
    while let Ok(chunk) = receiver.try_recv() {
        text.extend(chunk);
    }
    String::from_utf8(text)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The text inside the JSON object `span` of one log line. It is empty without that object.
fn span_of(line: &str) -> &str {
    line.split_once("\"span\":{")
        .and_then(|(_, rest)| rest.split_once('}'))
        .map_or("", |(inner, _)| inner)
}

#[tokio::test(start_paused = true)]
async fn run_bounded_gives_the_line_of_a_panicking_call_the_job_of_the_caller() {
    let (_guard, receiver) = capture();
    let panicking: Call = Box::pin(async {
        tracing::error!(panic_message = "a call failed", "a panic occurred");
        broken_step().unwrap();
        Ok(())
    });

    run_bounded(vec![panicking], Duration::from_secs(10))
        .instrument(tracing::info_span!("job", job = JOB))
        .await;

    let lines = lines_of(&receiver);
    let panics: Vec<_> = lines
        .iter()
        .filter(|line| line.contains("a panic occurred"))
        .collect();
    assert_eq!(panics.len(), 1, "{lines:?}");
    let span = span_of(panics[0]);
    assert!(span.contains(&format!("\"job\":\"{JOB}\"")), "{lines:?}");
}
