//! What the start has opened so far, and the order that closes it.
//!
//! The start opens its resources one step at a time. A step can fail, and a signal can end the start
//! early. [`Opened`] holds the resources that exist at that point, so one function closes them in
//! one fixed order. A finished server converts into the same type.

use std::sync::Arc;
use std::time::Duration;

use recipe_app::identity::DetachedTasks;
use recipe_db::CloseOutcome;
use recipe_web::{HealthState, ReadOnlyMode};
use tokio::runtime::Runtime;
use tokio::time::Instant;

use super::host::{PoolClose, Signal, WatchdogControl};
use super::plan::ShutdownSettings;
use super::server::{CloseReport, ListenerHandle, ServerParts};
use crate::supervisor::Supervisor;

/// The resources that the async steps have opened. Every field starts empty.
#[derive(Default)]
pub(super) struct Resources {
    pub main_pool: Option<Arc<dyn PoolClose>>,
    pub health_pool: Option<Arc<dyn PoolClose>>,
    pub health: Option<HealthState>,
    pub supervisor: Option<Supervisor>,
    pub detached: Option<DetachedTasks>,
    pub internal: Option<ListenerHandle>,
    pub public: Option<ListenerHandle>,
}

/// The resources of a start that is open or half open.
///
/// The type solves one problem: the close of a failed start and the close of a finished server must
/// share one order.
/// Otherwise a later change fixes only one of them.
pub(super) struct Opened {
    pub watchdog: Box<dyn WatchdogControl>,
    pub runtime: Runtime,
    pub resources: Resources,
    pub shutdown: ShutdownSettings,
}

impl From<ServerParts> for Opened {
    fn from(parts: ServerParts) -> Self {
        Self {
            watchdog: parts.watchdog,
            runtime: parts.runtime,
            resources: Resources {
                main_pool: Some(parts.main_pool),
                health_pool: Some(parts.health_pool),
                health: Some(parts.health),
                supervisor: Some(parts.supervisor),
                detached: Some(parts.detached),
                internal: Some(parts.internal),
                public: Some(parts.public),
            },
            shutdown: parts.shutdown,
        }
    }
}

impl Opened {
    /// Turn a complete start into the parts of a server.
    ///
    /// # Errors
    ///
    /// Gives the start back when a resource is missing. The caller then closes it.
    pub(super) fn into_parts(
        self,
        signal: Signal,
        read_only: ReadOnlyMode,
    ) -> Result<ServerParts, Box<Self>> {
        let Resources {
            main_pool,
            health_pool,
            health,
            supervisor,
            detached,
            internal,
            public,
        } = self.resources;
        match (
            main_pool,
            health_pool,
            health,
            supervisor,
            detached,
            internal,
            public,
        ) {
            (
                Some(main_pool),
                Some(health_pool),
                Some(health),
                Some(supervisor),
                Some(detached),
                Some(internal),
                Some(public),
            ) => Ok(ServerParts {
                runtime: self.runtime,
                signal: Some(signal),
                public,
                internal,
                supervisor,
                detached,
                main_pool,
                health_pool,
                watchdog: self.watchdog,
                health,
                read_only,
                shutdown: self.shutdown,
            }),
            (main_pool, health_pool, health, supervisor, detached, internal, public) => {
                Err(Box::new(Self {
                    watchdog: self.watchdog,
                    runtime: self.runtime,
                    resources: Resources {
                        main_pool,
                        health_pool,
                        health,
                        supervisor,
                        detached,
                        internal,
                        public,
                    },
                    shutdown: self.shutdown,
                }))
            }
        }
    }

    /// Close everything in the stop order and report what the drain cut.
    ///
    /// The drain waits for the public listener, the jobs, and the detached tasks to end. It counts
    /// what it cuts. It writes one `WARN` line when it aborts a detached task.
    ///
    /// The order is: turn readiness off, disarm the watchdog, stop the public listener and the
    /// jobs.
    /// The detached tasks drain after the public listener, because a request in flight can still
    /// spawn one. They share the one deadline of the jobs, and the abort line comes before the pool
    /// lines.
    /// Then it closes both pools and writes one `WARN` line for each pool that timed out.
    /// Then it stops the internal listener.
    /// Last, it ends the watchdog thread and stops the runtime (ADR-0087, ADR-0090).
    pub(super) fn close(self) -> CloseReport {
        let Self {
            watchdog,
            runtime,
            resources,
            shutdown,
        } = self;
        let mut report = runtime.block_on(close_resources(resources, watchdog.as_ref(), shutdown));
        report.watchdog_panicked = watchdog.stop().is_err();
        runtime.shutdown_timeout(shutdown.runtime_stop);
        report
    }
}

/// Close the resources in the stop order. The watchdog thread ends later, in the caller.
async fn close_resources(
    resources: Resources,
    watchdog: &dyn WatchdogControl,
    shutdown: ShutdownSettings,
) -> CloseReport {
    if let Some(health) = &resources.health {
        health.begin_shutdown();
    }
    watchdog.disarm();
    if let Some(public) = &resources.public {
        public.stop();
    }
    // The listener, the jobs, and the detached tasks share the one deadline of `shutdown.drain`
    // (7.3 steps 3 and 4, ADR-0172 (ag)). A request in flight can spawn a detached task until the
    // listener has joined. So the detached drain starts after that join and takes the rest of the
    // deadline. The pools close after both arms.
    let drain_started = Instant::now();
    let ((cut_public, aborted_detached), cut_jobs) = tokio::join!(
        async {
            let cut = join_listener(resources.public).await;
            let rest = shutdown.drain.saturating_sub(drain_started.elapsed());
            (cut, drain_detached(resources.detached, rest).await)
        },
        drain_jobs(resources.supervisor, shutdown)
    );
    warn_if_aborted(aborted_detached);
    // Both closes run at once, so both share one budget of `shutdown.pool_close` (7.3 step 5).
    let (main_pool, health_pool) = tokio::join!(
        close_pool(resources.main_pool, shutdown),
        close_pool(resources.health_pool, shutdown)
    );
    // The pool lines come before the stop of the internal listener (ADR-0090 (b), 7.3 step 5).
    warn_if_timed_out("request pool", main_pool);
    warn_if_timed_out("health pool", health_pool);
    if let Some(internal) = &resources.internal {
        internal.stop();
    }
    let cut_internal = join_listener(resources.internal).await;
    CloseReport {
        cut_public,
        cut_internal,
        cut_jobs,
        main_pool,
        health_pool,
        watchdog_panicked: false,
    }
}

/// Wait for the accept loop to drain and return the number of cut connections.
///
/// A loop that ended before the stop has its join result in the handle. A kept `JoinError` gives
/// no line, because the `ERROR` line of `main` covers that fault.
async fn join_listener(handle: Option<ListenerHandle>) -> usize {
    let Some(handle) = handle else {
        return 0;
    };
    let kind = handle.kind();
    let kept = handle.has_ended();
    match handle.join().await {
        Ok(outcome) => outcome.cut(),
        Err(_) if kept => 0,
        Err(error) => {
            tracing::warn!(
                listener = ?kind,
                panicked = error.is_panic(),
                "an accept loop ended without a drain result"
            );
            0
        }
    }
}

/// Write the `WARN` line of a pool whose close timed out. A closed or missing pool gets no line.
fn warn_if_timed_out(pool: &str, outcome: Option<CloseOutcome>) {
    if outcome == Some(CloseOutcome::TimedOut) {
        tracing::warn!(pool, "the pool close timed out and a connection stays open");
    }
}

/// Stop the jobs and return the number of runs that the drain cut.
async fn drain_jobs(supervisor: Option<Supervisor>, shutdown: ShutdownSettings) -> usize {
    match supervisor {
        Some(supervisor) => supervisor.drain(shutdown.drain).await.cut(),
        None => 0,
    }
}

/// Wait for the detached tasks for at most `rest` and return the number of aborted tasks.
async fn drain_detached(detached: Option<DetachedTasks>, rest: Duration) -> usize {
    match detached {
        Some(detached) => detached.drain(rest).await,
        None => 0,
    }
}

/// Write the one `WARN` line of the aborted detached tasks. A count of 0 gets no line (ADR-0172 (ah)).
fn warn_if_aborted(aborted: usize) {
    if aborted > 0 {
        tracing::warn!(
            aborted,
            "the drain aborted detached tasks and their security events may be lost"
        );
    }
}

/// Close one pool, when it is open.
async fn close_pool(
    pool: Option<Arc<dyn PoolClose>>,
    shutdown: ShutdownSettings,
) -> Option<CloseOutcome> {
    match pool {
        Some(pool) => Some(pool.close(shutdown.pool_close).await),
        None => None,
    }
}

#[cfg(test)]
mod cut_tests;
#[cfg(test)]
mod tests;
