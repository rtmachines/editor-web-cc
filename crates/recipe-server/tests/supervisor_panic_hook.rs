#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Test of task 7.5 of the change `operations-compose`: the field `job` on the line of the real
//! panic hook (ADR-0088 (a)).
//!
//! The panic hook is global state of the process, so a unit test cannot install it. This file
//! holds exactly one test, and the test binary runs no other panic. The test installs the hook,
//! runs a job that panics under a real [`Supervisor`], and reads the JSON lines. Time is paused,
//! so the test never waits for real time.

use std::io;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use recipe_app::error::AppError;
use recipe_app::jobs::RunKind;
use recipe_app::operations::AlertCounters;
use recipe_app::ports::{Clock, TimeStateKey};
use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;
use recipe_server::schedule::Schedule;
use recipe_server::supervisor::{JobName, JobRun, Supervisor, SupervisorSettings};
use recipe_server::{logging, panic_hook};
use tokio_util::sync::CancellationToken;

/// The longest virtual time of the test. A test that needs more has a fault.
const BOUND: Duration = Duration::from_secs(1_000_000);

/// The interval of the job. The first run starts one interval after the start of the schedule.
const INTERVAL: Duration = Duration::from_secs(60);

/// The virtual time that the test lets pass. The run at 60 seconds panics, and no restart follows.
const OBSERVED: Duration = Duration::from_secs(65);

/// The time that the drain waits for the open runs.
const DRAIN_DEADLINE: Duration = Duration::from_secs(5);

/// The message of the panic of the job.
const PANIC_MESSAGE: &str = "the job panic";

/// The message of the line of the panic hook.
const HOOK_TEXT: &str = "a panic occurred";

/// The message of the line of the supervisor for a panic.
const SUPERVISOR_TEXT: &str = "the job run panicked";

/// A writer that sends each write to a channel, so a test reads the log output without a lock.
struct ChannelWriter(Sender<Vec<u8>>);

impl io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf.to_vec()).map_err(io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A clock with a fixed instant. The schedule `every` reads only the time since its start.
struct FixedClock(Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

/// A job whose run panics. `Result::expect` starts the panic, because `clippy::panic` also applies
/// in tests.
struct PanickingJob;

impl JobRun for PanickingJob {
    async fn run(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        let too_large = u32::from(u8::MAX).checked_add(1).unwrap();
        let _small: u8 = u8::try_from(too_large).expect(PANIC_MESSAGE);
        Ok(())
    }
}

/// The text inside the JSON object `key` of one log line, without the braces.
///
/// The text is empty when the line has no such object. The helper stops at the first closing
/// brace, so it fits only an object without a nested object, as `fields` and `span` of the server.
fn object_of<'a>(line: &'a str, key: &str) -> &'a str {
    let opening = format!("\"{key}\":{{");
    line.split_once(opening.as_str())
        .and_then(|(_, rest)| rest.split_once('}'))
        .map_or("", |(inner, _)| inner)
}

/// Run `PanickingJob` as the job `cleanup` under a supervisor until the first run has panicked.
async fn run_panicking_job() {
    let first = TimeStateKey::parse_text("2026-10-24T00:00:00.000000Z").unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(first));
    let settings = SupervisorSettings {
        restart_delay: Duration::from_secs(10),
    };
    let stop = CancellationToken::new();
    let mut supervisor = Supervisor::new(settings, clock, stop.clone());
    supervisor.spawn(
        JobName::Cleanup,
        Schedule::every(INTERVAL).unwrap(),
        PanickingJob,
    );
    tokio::time::sleep(OBSERVED).await;
    stop.cancel();
    assert!(supervisor.drain(DRAIN_DEADLINE).await.is_complete());
}

#[tokio::test(start_paused = true)]
async fn the_hook_line_and_the_supervisor_line_of_a_job_panic_both_get_the_job_from_the_span() {
    let counters = AlertCounters::default();
    panic_hook::install(counters.clone());
    let (sender, receiver) = mpsc::channel();
    let filter = logging::build_filter(Some("info")).unwrap();
    let subscriber = logging::subscriber(filter, move || ChannelWriter(sender.clone()));
    let guard = tracing::subscriber::set_default(subscriber);

    let outcome = tokio::time::timeout(BOUND, run_panicking_job()).await;
    drop(guard);
    // A failed assertion below then goes through the default hook.
    drop(std::panic::take_hook());

    assert!(outcome.is_ok(), "the test ran longer than its time bound");
    let bytes: Vec<u8> = receiver.try_iter().flatten().collect();
    let text = String::from_utf8(bytes).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let hook_lines: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| line.contains(HOOK_TEXT))
        .collect();
    let supervisor_lines: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| line.contains(SUPERVISOR_TEXT))
        .collect();
    assert_eq!(hook_lines.len(), 1, "{text}");
    assert_eq!(supervisor_lines.len(), 1, "{text}");
    let (hook, supervisor) = (hook_lines[0], supervisor_lines[0]);
    assert_ne!(hook, supervisor, "the two lines must be two events");
    assert!(
        // The message of `expect` also holds the `Debug` text of the error.
        hook.contains(&format!("\"panic_message\":\"{PANIC_MESSAGE}:")),
        "{hook}"
    );
    for line in [hook, supervisor] {
        let span = object_of(line, "span");
        assert!(span.contains("\"job\":\"cleanup\""), "{line}");
        assert!(span.contains("\"iteration\":1"), "{line}");
        assert!(line.contains("\"level\":\"ERROR\""), "{line}");
    }
    assert_eq!(counters.panics(), 1);
}
