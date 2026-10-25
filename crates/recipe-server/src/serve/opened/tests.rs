//! The time budget of the close of both pools (`reliability.md` section 7.3 step 5, ADR-0090 (b)).

use std::net::Ipv4Addr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use recipe_app::error::AppError;
use recipe_app::jobs::RunKind;
use recipe_db::CloseOutcome;
use recipe_domain::audit::RequestId;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tokio::time::Instant;

use tokio_util::sync::CancellationToken;

use super::{Opened, Resources, close_resources, join_listener};
use crate::listener::{AcceptLoop, DrainOutcome, ListenerSettings};
use crate::schedule::Schedule;
use crate::serve::host::{ListenerKind, PoolClose, WatchdogControl};
use crate::serve::plan::ShutdownSettings;
use crate::serve::server::{CloseReport, ListenerHandle};
use crate::serve::test_support::{LogCapture, START_TEXT, TokioClock};
use crate::supervisor::{JobName, JobRun, Supervisor, SupervisorSettings};

/// A pool whose close waits for the whole limit and then reports the timeout.
///
/// The pool records the instant at which its close starts. The tests read it on the paused clock.
#[derive(Default)]
struct SlowPool {
    close_started: OnceLock<Instant>,
}

impl SlowPool {
    /// The time from `origin` to the start of the close. `None` means the close never started.
    fn close_started_after(&self, origin: Instant) -> Option<Duration> {
        self.close_started
            .get()
            .map(|started| started.duration_since(origin))
    }
}

#[async_trait]
impl PoolClose for SlowPool {
    async fn close(&self, limit: Duration) -> CloseOutcome {
        let _first_call = self.close_started.set(Instant::now());
        tokio::time::sleep(limit).await;
        CloseOutcome::TimedOut
    }
}

/// A watchdog handle that does nothing.
pub(super) struct IdleWatchdog;

impl WatchdogControl for IdleWatchdog {
    fn arm(&self) {}

    fn disarm(&self) {}

    fn stop(self: Box<Self>) -> std::thread::Result<()> {
        Ok(())
    }
}

#[tokio::test(start_paused = true)]
async fn both_pools_close_within_one_pool_close_budget() {
    let shutdown = ShutdownSettings::default();
    let resources = Resources {
        main_pool: Some(Arc::new(SlowPool::default())),
        health_pool: Some(Arc::new(SlowPool::default())),
        ..Resources::default()
    };
    let started = Instant::now();
    let report = close_resources(resources, &IdleWatchdog, shutdown).await;
    let elapsed = started.elapsed();
    let margin = Duration::from_millis(100);
    let budget = shutdown.pool_close.checked_add(margin).unwrap();
    assert!(
        elapsed <= budget,
        "the close took {elapsed:?} for a budget of {:?}",
        shutdown.pool_close
    );
    assert_eq!(report.main_pool, Some(CloseOutcome::TimedOut));
    assert_eq!(report.health_pool, Some(CloseOutcome::TimedOut));
}

/// A job run that never ends.
struct HangingJob;

impl JobRun for HangingJob {
    async fn run(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        std::future::pending().await
    }
}

/// The sum rule of `reliability.md` section 7.3 gives 30 + 5 + 0 + 3 seconds.
#[test]
fn the_four_limits_of_the_stop_sum_to_38_seconds() {
    let shutdown = ShutdownSettings::default();
    let sum = [
        shutdown.pool_close,
        shutdown.internal_drain,
        shutdown.runtime_stop,
    ]
    .into_iter()
    .try_fold(shutdown.drain, Duration::checked_add);
    assert_eq!(sum, Some(Duration::from_secs(38)));
}

/// A supervisor whose only job run never ends. The run has started when the function returns.
async fn supervisor_with_hanging_job() -> Supervisor {
    let mut supervisor = Supervisor::new(
        SupervisorSettings {
            restart_delay: Duration::from_secs(10),
        },
        TokioClock::at(START_TEXT),
        CancellationToken::new(),
    );
    let schedule = Schedule::now_then_every(Duration::from_secs(60)).unwrap();
    supervisor.spawn(JobName::Alerts, schedule, HangingJob);
    // The pause lets the loop start its run.
    tokio::time::sleep(Duration::from_millis(1)).await;
    supervisor
}

/// The async part of the stop takes `drain` and then `pool_close`. The internal listener adds none.
/// The runtime stop is a real time limit, so a paused clock cannot show it.
///
/// The pool close starts after the job drain (design D4 step 2). A sum of 35 s alone also fits a
/// job drain after the pool close, so the test reads the start of each pool close.
#[tokio::test(start_paused = true)]
async fn a_job_run_that_never_ends_and_two_slow_pools_take_the_drain_plus_one_pool_close() {
    let shutdown = ShutdownSettings::default();
    let supervisor = supervisor_with_hanging_job().await;
    let main_pool = Arc::new(SlowPool::default());
    let health_pool = Arc::new(SlowPool::default());
    let resources = Resources {
        main_pool: Some(main_pool.clone()),
        health_pool: Some(health_pool.clone()),
        supervisor: Some(supervisor),
        ..Resources::default()
    };
    let started = Instant::now();
    let report = close_resources(resources, &IdleWatchdog, shutdown).await;
    assert_eq!(
        started.elapsed(),
        shutdown.drain.checked_add(shutdown.pool_close).unwrap()
    );
    assert_eq!(report.cut_jobs, 1);
    assert_eq!(main_pool.close_started_after(started), Some(shutdown.drain));
    assert_eq!(
        health_pool.close_started_after(started),
        Some(shutdown.drain)
    );
}

/// A public listener whose accept loop needs the whole `drain` after the stop.
///
/// A real loop with a connection that never ends would need real socket timing. This stub sleeps
/// for the drain on the paused clock instead.
async fn public_listener_that_drains_for(drain: Duration) -> ListenerHandle {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let stop = CancellationToken::new();
    let stopped = stop.clone();
    let settings = ListenerSettings {
        header_read_timeout: Duration::from_secs(30),
        drain_deadline: Duration::ZERO,
        accept_error_pause: Duration::from_millis(10),
    };
    let task = tokio::spawn(async move {
        stopped.cancelled().await;
        tokio::time::sleep(drain).await;
        // The loop gets a stop that has come already, so it ends with an empty drain.
        AcceptLoop::new(Router::new(), settings, stopped)
            .run(listener)
            .await
    });
    ListenerHandle::new(ListenerKind::Public, stop, task)
}

/// The public drain and the job drain run together (design D4 step 2). Run one after the other,
/// they would take twice `drain` before the pool close starts.
#[tokio::test(start_paused = true)]
async fn the_public_drain_and_the_job_drain_run_together_before_the_pool_close() {
    let shutdown = ShutdownSettings::default();
    let supervisor = supervisor_with_hanging_job().await;
    let main_pool = Arc::new(SlowPool::default());
    let health_pool = Arc::new(SlowPool::default());
    let resources = Resources {
        main_pool: Some(main_pool.clone()),
        health_pool: Some(health_pool.clone()),
        supervisor: Some(supervisor),
        public: Some(public_listener_that_drains_for(shutdown.drain).await),
        ..Resources::default()
    };
    let started = Instant::now();
    let report = close_resources(resources, &IdleWatchdog, shutdown).await;
    assert_eq!(
        started.elapsed(),
        shutdown.drain.checked_add(shutdown.pool_close).unwrap()
    );
    assert_eq!(report.cut_jobs, 1);
    assert_eq!(main_pool.close_started_after(started), Some(shutdown.drain));
    assert_eq!(
        health_pool.close_started_after(started),
        Some(shutdown.drain)
    );
}

/// The runtime stop ends at its limit and does not wait for a blocking task (ADR-0087).
#[test]
fn the_runtime_stop_waits_for_its_limit_and_not_for_a_blocking_task() {
    const BLOCKED: Duration = Duration::from_millis(1_500);
    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
    let _task = runtime.spawn_blocking(|| std::thread::sleep(BLOCKED));
    let shutdown = ShutdownSettings {
        runtime_stop: Duration::from_millis(100),
        ..ShutdownSettings::default()
    };
    let opened = Opened {
        watchdog: Box::new(IdleWatchdog),
        runtime,
        resources: Resources::default(),
        shutdown,
    };
    let started = std::time::Instant::now();
    let report = opened.close();
    let elapsed = started.elapsed();
    assert_eq!(report, CloseReport::default());
    assert!(
        elapsed >= shutdown.runtime_stop && elapsed < BLOCKED,
        "the runtime stop took {elapsed:?} for a limit of {:?}",
        shutdown.runtime_stop
    );
}

/// A pool whose close ends at once with the timeout.
struct TimedOutPool;

#[async_trait]
impl PoolClose for TimedOutPool {
    async fn close(&self, _limit: Duration) -> CloseOutcome {
        CloseOutcome::TimedOut
    }
}

/// An internal listener on a free port. Its task writes one `INFO` line when the accept loop ends.
async fn internal_listener_that_logs_its_end() -> ListenerHandle {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let stop = CancellationToken::new();
    let settings = ListenerSettings {
        header_read_timeout: Duration::from_secs(30),
        drain_deadline: Duration::ZERO,
        accept_error_pause: Duration::from_millis(10),
    };
    let accept_loop = AcceptLoop::new(Router::new(), settings, stop.clone());
    let task = tokio::spawn(async move {
        let outcome = accept_loop.run(listener).await;
        tracing::info!("the internal listener ended");
        outcome
    });
    ListenerHandle::new(ListenerKind::Internal, stop, task)
}

/// ADR-0090 (b) and `reliability.md` section 7.3 step 5: the pool line comes before the stop of the
/// internal listener.
#[tokio::test]
async fn the_pool_warnings_come_before_the_stop_of_the_internal_listener() {
    let logs = LogCapture::start();
    let resources = Resources {
        main_pool: Some(Arc::new(TimedOutPool)),
        health_pool: Some(Arc::new(TimedOutPool)),
        internal: Some(internal_listener_that_logs_its_end().await),
        ..Resources::default()
    };
    let report = close_resources(resources, &IdleWatchdog, ShutdownSettings::default()).await;
    assert_eq!(report.main_pool, Some(CloseOutcome::TimedOut));
    let lines = logs.lines();
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines[0].contains("request pool"), "{lines:?}");
    assert!(lines[1].contains("health pool"), "{lines:?}");
    assert!(
        lines[2].contains("the internal listener ended"),
        "{lines:?}"
    );
    assert!(
        lines[..2]
            .iter()
            .all(|line| line.contains("\"level\":\"WARN\""))
    );
}

/// The panic hook owns the `ERROR` line of a panic. The join writes one `WARN` line.
#[tokio::test]
async fn join_of_a_panicked_listener_writes_a_warn_line_and_no_error_line() {
    let logs = LogCapture::start();
    let task = tokio::spawn(async { std::hint::black_box(None::<DrainOutcome>).unwrap() });
    let handle = ListenerHandle::new(ListenerKind::Public, CancellationToken::new(), task);

    let cut = join_listener(Some(handle)).await;

    assert_eq!(cut, 0);
    let lines = logs.lines();
    assert!(
        lines
            .iter()
            .all(|line| !line.contains("\"level\":\"ERROR\"")),
        "the join wrote an ERROR line: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| {
            line.contains("\"level\":\"WARN\"") && line.contains("\"listener\":\"Public\"")
        }),
        "no WARN line with the field listener: {lines:?}"
    );
}
