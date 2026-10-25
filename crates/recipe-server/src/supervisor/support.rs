//! Helpers that the tests of the supervisor share.
//!
//! Every test uses paused time. The probe records the start and the end of each run on a channel,
//! with the time since the start of the test. A test reads the channel after the supervisor
//! stops, so no test depends on the speed of a thread.

use std::future::Future;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use recipe_app::error::AppError;
use recipe_app::jobs::RunKind;
use recipe_app::ports::{Clock, TimeStateKey};
use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::Instant;
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Subscriber};

use super::{JobRun, SupervisorSettings};
use crate::logging;

/// The longest virtual time of a test. A test that needs more has a fault.
const BOUND: Duration = Duration::from_secs(1_000_000);

/// The restart delay of the tests. It is shorter than every interval of the tests.
const RESTART_DELAY: Duration = Duration::from_secs(10);

pub(super) fn settings() -> SupervisorSettings {
    SupervisorSettings {
        restart_delay: RESTART_DELAY,
    }
}

pub(super) fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

/// Run `body` and fail the test when it needs more than `BOUND` of virtual time.
pub(super) async fn bounded<T>(body: impl Future<Output = T>) -> T {
    tokio::time::timeout(BOUND, body)
        .await
        .expect("the test ran longer than its time bound")
}

/// A clock that follows the paused clock of Tokio, from a fixed first instant.
#[derive(Debug)]
pub(super) struct PausedClock {
    first: Timestamp,
    started: Instant,
}

impl PausedClock {
    /// A clock that shows the UTC time `text` (for example `2026-10-24T02:59:30.000000Z`) now.
    pub(super) fn at(text: &str) -> Arc<Self> {
        Arc::new(Self {
            first: TimeStateKey::parse_text(text).expect("a valid test instant"),
            started: Instant::now(),
        })
    }
}

impl Clock for PausedClock {
    fn now(&self) -> Timestamp {
        let elapsed = self.started.elapsed().try_into().unwrap();
        let now = self
            .first
            .as_offset_date_time()
            .checked_add(elapsed)
            .unwrap();
        Timestamp::from(now)
    }
}

/// What one run of a probe does after it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Behavior {
    /// Work for the probe duration, then end with `Ok`.
    Succeed,
    /// Work for the probe duration, then end with `Err`.
    Fail,
    /// Work for the probe duration, then panic.
    Panic,
    /// Never end.
    Hang,
}

/// What the probe saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Event {
    Started(Duration, RequestId, RunKind),
    Ended(Duration),
    /// The future of a run was dropped before it ended.
    Dropped(Duration),
}

/// A job for the tests. The call number picks the behavior. Later calls use the last one.
pub(super) struct Probe {
    origin: Instant,
    events: UnboundedSender<Event>,
    work: Duration,
    script: Vec<Behavior>,
    calls: AtomicUsize,
    running: AtomicUsize,
    overlaps: Arc<AtomicUsize>,
}

/// The receiving side of a probe and its overlap counter.
pub(super) struct Observer {
    events: UnboundedReceiver<Event>,
    overlaps: Arc<AtomicUsize>,
}

impl Observer {
    /// Take every event that the probe has sent so far.
    pub(super) fn events(&mut self) -> Vec<Event> {
        let mut taken = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            taken.push(event);
        }
        taken
    }

    /// The whole seconds since the test start of each run start.
    pub(super) fn starts(&mut self) -> Vec<u64> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Started(at, _, _) => Some(at.as_secs()),
                _ => None,
            })
            .collect()
    }

    /// The run kind that each run start received, in the order of the starts.
    pub(super) fn kinds(&mut self) -> Vec<RunKind> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Started(_, _, kind) => Some(kind),
                _ => None,
            })
            .collect()
    }

    /// How often a run started while another run of the same probe was open.
    pub(super) fn overlaps(&self) -> usize {
        self.overlaps.load(Ordering::SeqCst)
    }
}

/// Make a probe that works for `work` and follows `script`. `origin` is the test start.
pub(super) fn probe(origin: Instant, work: Duration, script: &[Behavior]) -> (Probe, Observer) {
    let (events, receiver) = unbounded_channel();
    let overlaps = Arc::new(AtomicUsize::new(0));
    let probe = Probe {
        origin,
        events,
        work,
        script: script.to_vec(),
        calls: AtomicUsize::new(0),
        running: AtomicUsize::new(0),
        overlaps: Arc::clone(&overlaps),
    };
    (
        probe,
        Observer {
            events: receiver,
            overlaps,
        },
    )
}

/// Sends `Dropped` when the future of a run goes away before the run ends.
struct DropSignal {
    origin: Instant,
    events: UnboundedSender<Event>,
    running: bool,
}

impl Drop for DropSignal {
    fn drop(&mut self) {
        if self.running {
            // The receiver can be gone at the end of a test. Then nobody needs the event.
            let _ = self.events.send(Event::Dropped(self.origin.elapsed()));
        }
    }
}

impl JobRun for Probe {
    async fn run(&self, request_id: RequestId, kind: RunKind) -> Result<(), AppError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let behavior = self
            .script
            .get(call)
            .or(self.script.last())
            .copied()
            .unwrap_or(Behavior::Succeed);
        if self.running.fetch_add(1, Ordering::SeqCst) != 0 {
            self.overlaps.fetch_add(1, Ordering::SeqCst);
        }
        let mut signal = DropSignal {
            origin: self.origin,
            events: self.events.clone(),
            running: true,
        };
        let _ = self
            .events
            .send(Event::Started(self.origin.elapsed(), request_id, kind));
        tokio::time::sleep(self.work).await;
        if behavior == Behavior::Hang {
            std::future::pending::<()>().await;
        }
        self.running.fetch_sub(1, Ordering::SeqCst);
        signal.running = false;
        let _ = self.events.send(Event::Ended(self.origin.elapsed()));
        match behavior {
            Behavior::Fail => Err(AppError::Unauthenticated),
            Behavior::Panic => {
                // A failed parse stands for any bug of a job that panics.
                let number: u8 = "not a number".parse().unwrap();
                assert_eq!(number, 0);
                Ok(())
            }
            Behavior::Succeed | Behavior::Hang => Ok(()),
        }
    }
}

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first. A test thread
/// without a subscriber then caches `Interest::never` for all threads.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

/// Install `subscriber` for the current thread until the guard drops.
///
/// Every test that needs a subscriber calls this function and not `tracing::subscriber::set_default`.
pub(super) fn set_test_default<S>(subscriber: S) -> DefaultGuard
where
    S: Subscriber + Send + Sync + 'static,
{
    LazyLock::force(&KEEPALIVE);
    tracing::subscriber::set_default(subscriber)
}

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

/// The log lines of the current thread, as parsed text.
pub(super) struct LogCapture {
    lines: Receiver<Vec<u8>>,
    _guard: DefaultGuard,
}

impl LogCapture {
    /// Install a JSON subscriber for the current thread until the capture drops.
    pub(super) fn start() -> Self {
        Self::start_with("info")
    }

    /// Install a JSON subscriber with the log filter `directives`, as the operator sets it.
    pub(super) fn start_with(directives: &str) -> Self {
        let (sender, lines) = std::sync::mpsc::channel();
        let filter = logging::build_filter(Some(directives)).expect("a valid filter");
        let subscriber = logging::subscriber(filter, move || ChannelWriter(sender.clone()));
        Self {
            lines,
            _guard: set_test_default(subscriber),
        }
    }

    /// Every line written so far.
    pub(super) fn lines(&self) -> Vec<String> {
        let mut text = Vec::new();
        while let Ok(chunk) = self.lines.try_recv() {
            text.extend(chunk);
        }
        String::from_utf8(text)
            .expect("log lines are UTF-8")
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

/// The text inside the JSON object `key` of one log line, without the braces.
///
/// The text is empty when the line has no such object. The helper stops at the first closing
/// brace, so it fits only an object without a nested object, as `fields` and `span` of the server.
pub(super) fn object_of<'a>(line: &'a str, key: &str) -> &'a str {
    let opening = format!("\"{key}\":{{");
    line.split_once(opening.as_str())
        .and_then(|(_, rest)| rest.split_once('}'))
        .map_or("", |(inner, _)| inner)
}
