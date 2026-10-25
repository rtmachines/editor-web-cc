//! The watchdog thread of `recipe-server` (`reliability.md` section 2.4 rule 5).
//!
//! A hung Tokio runtime answers no request, and only a thread outside the runtime can see that.
//! The watchdog is an OS thread. It probes `/healthz` on the internal listener every
//! `watchdog.probe_interval`. After `watchdog.failures` failed probes inside `watchdog.window`,
//! it writes one `ERROR` line and ends the process with the exit code 70. Docker then restarts
//! the container.
//!
//! The watchdog has two states (ADR-0061 (g)). It starts not armed and counts no probe. `serve`
//! arms it after the internal listener opens, and `disarm` or `stop` ends the counting again.
//! The handle commands go through a channel, so `stop` ends a thread that waits for the next
//! probe at once.

use std::collections::VecDeque;
use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::probe::{ProbeFault, ProbePath, probe_local};

/// The exit code of a process that the watchdog ends (`reliability.md` section 2.4 rule 5).
pub const STALLED_EXIT_CODE: i32 = 70;

/// The time between two probes (`watchdog.probe_interval`).
const DEFAULT_INTERVAL: Duration = Duration::from_secs(10);

/// The number of failed probes that ends the process (`watchdog.failures`).
const DEFAULT_FAILURES: u32 = 3;

/// The time in which the failed probes count (`watchdog.window`).
const DEFAULT_WINDOW: Duration = Duration::from_secs(30);

/// The numbers of the watchdog. The defaults equal `reliability.md` section 7.11.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogSettings {
    /// The time between two probes (`watchdog.probe_interval`).
    pub interval: Duration,
    /// The number of failed probes in the window that ends the process (`watchdog.failures`).
    /// A value of 0 acts like 1.
    pub failures: u32,
    /// The time in which the failed probes count (`watchdog.window`).
    pub window: Duration,
}

impl Default for WatchdogSettings {
    fn default() -> Self {
        Self {
            interval: DEFAULT_INTERVAL,
            failures: DEFAULT_FAILURES,
            window: DEFAULT_WINDOW,
        }
    }
}

/// The result of one failed probe for the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureVerdict {
    /// The window holds fewer failures than the limit.
    Tolerated,
    /// The window holds the limit of failures. The process must end.
    Tripped,
}

/// The failed probes of the last window.
///
/// The type solves one problem: the rule "N failures in T seconds" needs a clock, and a test must
/// not sleep for T seconds. The caller passes each instant, so a test injects the time. The
/// window keeps at most the limit of instants, so it never grows without bound.
#[derive(Debug, Clone)]
pub struct FailureWindow {
    limit: usize,
    span: Duration,
    failures: VecDeque<Instant>,
}

impl FailureWindow {
    /// Make an empty window. A limit of 0 acts like 1.
    #[must_use]
    pub fn new(limit: u32, span: Duration) -> Self {
        let limit = usize::try_from(limit.max(1)).unwrap_or(usize::MAX);
        Self {
            limit,
            span,
            failures: VecDeque::new(),
        }
    }

    /// Record a failed probe at `now`. A failure that is exactly one window old still counts.
    pub fn record(&mut self, now: Instant) -> FailureVerdict {
        let span = self.span;
        self.failures.retain(|failed| {
            now.checked_duration_since(*failed)
                .is_none_or(|age| age <= span)
        });
        self.failures.push_back(now);
        while self.failures.len() > self.limit {
            self.failures.pop_front();
        }
        if self.failures.len() >= self.limit {
            FailureVerdict::Tripped
        } else {
            FailureVerdict::Tolerated
        }
    }

    /// Forget every failure. The watchdog calls this when it is armed or disarmed.
    pub fn clear(&mut self) {
        self.failures.clear();
    }

    /// The number of failures that the window holds now.
    #[must_use]
    pub fn count(&self) -> usize {
        self.failures.len()
    }
}

/// What the watchdog knows when it gives up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripReport {
    /// The number of failed probes in the window.
    pub failures: usize,
    /// The length of the window.
    pub window: Duration,
    /// The fault of the last probe.
    pub last_fault: ProbeFault,
}

/// A command of the handle to the thread.
enum Message {
    Arm,
    Disarm,
    Stop,
}

/// The handle of the watchdog thread.
///
/// The methods `arm`, `disarm`, and `stop` are the three commands of ADR-0061 (g). A dropped
/// handle ends the thread, because the channel closes.
#[derive(Debug)]
pub struct Watchdog {
    messages: Sender<Message>,
    thread: JoinHandle<()>,
}

impl Watchdog {
    /// Start the thread, not armed. It probes `/healthz` on `127.0.0.1` and the given port.
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system when it cannot start the thread.
    pub fn start(settings: WatchdogSettings, port: u16) -> io::Result<Self> {
        let path = ProbePath::liveness();
        Self::start_with(settings, move || probe_local(port, &path), exit_stalled)
    }

    /// Start the thread with a given probe and a given action for the trip.
    fn start_with<P, T>(settings: WatchdogSettings, probe: P, on_trip: T) -> io::Result<Self>
    where
        P: FnMut() -> Result<(), ProbeFault> + Send + 'static,
        T: FnOnce(TripReport) + Send + 'static,
    {
        let (messages, inbox) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("watchdog".to_owned())
            .spawn(move || watch(&settings, &inbox, probe, on_trip))?;
        Ok(Self { messages, thread })
    }

    /// Start to count the probes. The window starts empty.
    ///
    /// A send fails only when the thread has ended, and then nothing is left to arm.
    pub fn arm(&self) {
        let _thread_ended = self.messages.send(Message::Arm);
    }

    /// Stop to count the probes. The thread stays alive.
    ///
    /// A send fails only when the thread has ended, and then nothing is left to disarm.
    pub fn disarm(&self) {
        let _thread_ended = self.messages.send(Message::Disarm);
    }

    /// End the thread and wait for it.
    ///
    /// # Errors
    ///
    /// Returns the panic payload when the thread panicked. The panic hook has logged it.
    pub fn stop(self) -> std::thread::Result<()> {
        let _thread_ended = self.messages.send(Message::Stop);
        self.thread.join()
    }
}

/// The loop of the thread. It returns after `Stop`, after a closed channel, and after a trip.
fn watch<P, T>(settings: &WatchdogSettings, inbox: &Receiver<Message>, mut probe: P, on_trip: T)
where
    P: FnMut() -> Result<(), ProbeFault>,
    T: FnOnce(TripReport),
{
    let mut window = FailureWindow::new(settings.failures, settings.window);
    let mut armed = false;
    let mut next_probe = after(Instant::now(), settings.interval);
    loop {
        let wait = next_probe.saturating_duration_since(Instant::now());
        match inbox.recv_timeout(wait) {
            Ok(Message::Arm) => {
                window.clear();
                armed = true;
                next_probe = after(Instant::now(), settings.interval);
            }
            Ok(Message::Disarm) => {
                window.clear();
                armed = false;
            }
            Ok(Message::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {
                let outcome = if armed { probe() } else { Ok(()) };
                match settle_commands(inbox, &mut window, &mut armed) {
                    Settled::Stopped => return,
                    Settled::Changed => {}
                    Settled::Quiet => {
                        if let Err(fault) = outcome
                            && let Some(report) = count_failure(&mut window, settings, fault)
                        {
                            on_trip(report);
                            return;
                        }
                    }
                }
                next_probe = after(Instant::now(), settings.interval);
            }
        }
    }
}

/// What the commands that arrived during a probe did to the watchdog.
enum Settled {
    /// `Stop` or a closed channel arrived. The thread ends and does not count the probe.
    Stopped,
    /// `Arm` or `Disarm` arrived. The result of the probe is old, so the thread drops it.
    Changed,
    /// No command arrived. The result of the probe counts.
    Quiet,
}

/// Apply every command that waited in the queue while a probe ran.
///
/// This function solves one problem: a probe can take up to `watchdog.probe_timeout`. A command
/// that arrives in that time must act before the result of the probe. Otherwise a stopped
/// watchdog could still end the process (ADR-0058 (l)).
fn settle_commands(
    inbox: &Receiver<Message>,
    window: &mut FailureWindow,
    armed: &mut bool,
) -> Settled {
    let mut settled = Settled::Quiet;
    loop {
        match inbox.try_recv() {
            Ok(Message::Arm) => {
                window.clear();
                *armed = true;
                settled = Settled::Changed;
            }
            Ok(Message::Disarm) => {
                window.clear();
                *armed = false;
                settled = Settled::Changed;
            }
            Ok(Message::Stop) | Err(TryRecvError::Disconnected) => return Settled::Stopped,
            Err(TryRecvError::Empty) => return settled,
        }
    }
}

/// Record a failed probe. Return the report when the window holds the limit of failures.
fn count_failure(
    window: &mut FailureWindow,
    settings: &WatchdogSettings,
    fault: ProbeFault,
) -> Option<TripReport> {
    match window.record(Instant::now()) {
        FailureVerdict::Tolerated => None,
        FailureVerdict::Tripped => Some(TripReport {
            failures: window.count(),
            window: settings.window,
            last_fault: fault,
        }),
    }
}

/// The instant that is `interval` after `start`. An overflow gives `start`, so a probe runs at once.
fn after(start: Instant, interval: Duration) -> Instant {
    start.checked_add(interval).unwrap_or(start)
}

/// Write one `ERROR` line and end the process with the exit code 70.
///
/// This is the watchdog function of `reliability.md` section 2.4 rule 5. It is the only function
/// of the workspace with the attribute `allow(clippy::exit)`.
// reliability.md section 2.3 permits this one exception: the process hangs, so `main` cannot
// return, and only this thread can end it.
#[allow(clippy::exit)]
fn exit_stalled(report: TripReport) {
    tracing::error!(
        failures = report.failures,
        window_seconds = report.window.as_secs(),
        fault = %report.last_fault,
        "the watchdog ends the process because the health probe failed"
    );
    std::process::exit(STALLED_EXIT_CODE)
}

#[cfg(test)]
mod tests;
