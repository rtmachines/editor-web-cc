//! The running server and its stop.
//!
//! [`Server`] is what a successful start returns. It owns every resource that step 3 to step 10
//! opened. [`Server::wait_for_signal`] waits for the stop signal. [`Server::stop`] closes the
//! resources in the stop order and writes the log lines of the stop.

use std::sync::Arc;

use recipe_app::identity::DetachedTasks;
#[cfg(test)]
use recipe_app::identity::IdentityContext;
use recipe_db::CloseOutcome;
#[cfg(test)]
use recipe_web::WebState;
use recipe_web::{HealthState, ReadOnlyMode};
use tokio::runtime::Runtime;
use tokio::task::{JoinError, JoinHandle};
use tokio_util::sync::CancellationToken;

use super::host::{ListenerKind, PoolClose, Signal, WatchdogControl};
use super::opened::Opened;
use super::plan::ShutdownSettings;
use crate::listener::DrainOutcome;
use crate::supervisor::Supervisor;

/// The handle of one accept loop.
///
/// The type solves one problem: the stop must cancel one listener and read its drain result later.
/// The two listeners stop at different points of the order.
pub struct ListenerHandle {
    kind: ListenerKind,
    stop: CancellationToken,
    task: JoinHandle<DrainOutcome>,
    /// The join result of a loop that ended before the stop. `None` until [`Self::ended`] ends.
    ended: Option<Result<DrainOutcome, JoinError>>,
}

impl ListenerHandle {
    /// Join the parts of one listener.
    #[must_use]
    pub(super) const fn new(
        kind: ListenerKind,
        stop: CancellationToken,
        task: JoinHandle<DrainOutcome>,
    ) -> Self {
        Self {
            kind,
            stop,
            task,
            ended: None,
        }
    }

    /// The listener that this handle belongs to.
    #[must_use]
    pub const fn kind(&self) -> ListenerKind {
        self.kind
    }

    /// Tell the accept loop to stop. The loop then drains its connections.
    pub fn stop(&self) {
        self.stop.cancel();
    }

    /// Wait until the accept loop ends and keep its join result.
    ///
    /// A second poll of a finished [`JoinHandle`] panics in Tokio. The handle therefore keeps the
    /// result, and [`Self::join`] reads it later. The method stays pending while the loop runs
    /// and is safe to drop at any point.
    async fn ended(&mut self) -> bool {
        let result = (&mut self.task).await;
        let panicked = result.as_ref().is_err_and(JoinError::is_panic);
        self.ended = Some(result);
        panicked
    }

    /// Whether [`Self::ended`] has kept a join result.
    pub(super) const fn has_ended(&self) -> bool {
        self.ended.is_some()
    }

    /// Give the join result of the accept loop. It awaits the task only when no result is kept.
    pub(super) async fn join(self) -> Result<DrainOutcome, JoinError> {
        match self.ended {
            Some(result) => result,
            None => self.task.await,
        }
    }
}

/// Why the wait of the server ended (ADR-0125).
///
/// The value holds no join result. The [`ListenerHandle`] keeps it, because a [`JoinError`] is not
/// `Clone`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StopCause {
    /// The stop signal fired.
    Signal,
    /// An accept loop ended before the signal.
    LoopEnded {
        /// The listener of the loop that ended.
        kind: ListenerKind,
        /// Whether the loop ended with a panic.
        panicked: bool,
    },
}

/// Wait for the stop signal or for the end of an accept loop. The first one wins.
///
/// The function solves one problem: a loop that ends early leaves a server that cannot serve. The
/// race starts the stop at once and does not wait for a signal that may never come. The `biased`
/// order polls the signal first, so a signal and a loop end at the same time give a normal stop.
pub(super) async fn race_stop(
    signal: Signal,
    public: &mut ListenerHandle,
    internal: &mut ListenerHandle,
) -> StopCause {
    tokio::select! {
        biased;
        () = signal => StopCause::Signal,
        panicked = public.ended() => StopCause::LoopEnded { kind: public.kind, panicked },
        panicked = internal.ended() => StopCause::LoopEnded { kind: internal.kind, panicked },
    }
}

/// What the close of the server found. [`CloseReport::log`] turns it into the log lines of the stop.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CloseReport {
    /// The public connections that the drain cut.
    pub cut_public: usize,
    /// The internal connections that the drain cut.
    pub cut_internal: usize,
    /// The job runs that the drain cut.
    pub cut_jobs: usize,
    /// The result of the close of the main pool, when it was open.
    pub main_pool: Option<CloseOutcome>,
    /// The result of the close of the health pool, when it was open.
    pub health_pool: Option<CloseOutcome>,
    /// Whether the thread of the watchdog ended with a panic.
    pub watchdog_panicked: bool,
}

impl CloseReport {
    /// Write the cut line of the report (ADR-0088, ADR-0089 (c)).
    ///
    /// A cut gets one `WARN` line with the three counts. It writes only when one count is above 0.
    /// A clean report writes nothing. The close
    /// has already written the line of a pool that timed out. The panic of the watchdog thread has
    /// its line from the panic hook.
    pub fn log(&self) {
        let cuts = [self.cut_public, self.cut_internal, self.cut_jobs];
        if cuts.iter().any(|count| *count > 0) {
            tracing::warn!(
                cut_public = self.cut_public,
                cut_internal = self.cut_internal,
                cut_jobs = self.cut_jobs,
                "the drain cut connections or job runs"
            );
        }
    }
}

/// Every part of a running server.
///
/// The type solves one problem: the close of a finished server moves each part into its own step
/// of the stop order. A reference cannot give away an owned part.
pub(super) struct ServerParts {
    /// The runtime of step 3. It runs every task below.
    pub runtime: Runtime,
    /// The stop signal of step 4. [`Server::wait_for_signal`] takes it, so it is `None` after a
    /// wait starts.
    pub signal: Option<Signal>,
    /// The public listener of step 10.
    pub public: ListenerHandle,
    /// The internal listener of step 8.
    pub internal: ListenerHandle,
    /// The supervisor with the loops of step 9.
    pub supervisor: Supervisor,
    /// The detached tasks of the use case context. The stop drains them before the pool close.
    pub detached: DetachedTasks,
    /// The close of the main pool.
    pub main_pool: Arc<dyn PoolClose>,
    /// The close of the health pool.
    pub health_pool: Arc<dyn PoolClose>,
    /// The handle of the watchdog thread. It is armed.
    pub watchdog: Box<dyn WatchdogControl>,
    /// The readiness handle. `begin_shutdown` turns `/readyz` off.
    pub health: HealthState,
    /// The flag of the read-only mode. Only the test accessor [`Server::read_only`] reads it.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "only the test accessor reads the flag")
    )]
    pub read_only: ReadOnlyMode,
    /// The times of the stop.
    pub shutdown: ShutdownSettings,
    /// A clone of the web state that the router holds. Only the tests of the close read it.
    #[cfg(test)]
    pub web: Option<WebState>,
}

/// A server that passed all ten steps of the start.
///
/// The fields are private. [`Server::wait_for_signal`] blocks until the stop signal fires. Then
/// [`Server::stop`] closes the server.
pub struct Server {
    parts: ServerParts,
    cause: Option<StopCause>,
}

impl Server {
    /// Wrap the parts of a finished start.
    #[must_use]
    pub(super) const fn new(parts: ServerParts) -> Self {
        Self { parts, cause: None }
    }

    /// The runtime of the server.
    #[cfg(test)]
    #[must_use]
    pub(super) const fn runtime(&self) -> &Runtime {
        &self.parts.runtime
    }

    /// The readiness handle.
    #[cfg(test)]
    #[must_use]
    pub(super) const fn health(&self) -> &HealthState {
        &self.parts.health
    }

    /// The use case context of the web state. The use cases spawn their detached tasks through it.
    #[cfg(test)]
    #[must_use]
    pub(super) fn identity(&self) -> &IdentityContext {
        self.parts
            .web
            .as_ref()
            .expect("the start of a test stores the web state")
            .identity()
    }

    /// The flag of the read-only mode.
    #[cfg(test)]
    #[must_use]
    pub(super) const fn read_only(&self) -> &ReadOnlyMode {
        &self.parts.read_only
    }

    /// Wait on the runtime until the stop signal fires or an accept loop ends.
    ///
    /// The wait ends once. A call after it returns at once. `Server::stop_cause` tells the cause.
    pub fn wait_for_signal(&mut self) {
        if let Some(signal) = self.parts.signal.take() {
            let ServerParts {
                runtime,
                public,
                internal,
                ..
            } = &mut self.parts;
            self.cause = Some(runtime.block_on(race_stop(signal, public, internal)));
        }
    }

    /// The cause that ended the wait. `None` before a wait has ended.
    #[must_use]
    pub(super) const fn stop_cause(&self) -> Option<StopCause> {
        self.cause
    }

    /// Close everything in the stop order of ADR-0087 and ADR-0090.
    ///
    /// The method writes the `WARN` line of each pool that timed out and of each accept loop that
    /// panicked. It writes them before it stops the internal listener (ADR-0090 (b)).
    /// [`Server::stop`] writes the cut line and the `INFO` line.
    #[must_use]
    pub(super) fn close(self) -> CloseReport {
        Opened::from(self.parts).close()
    }

    /// Stop the server: close everything and write the log lines of the stop.
    ///
    /// The close has written the pool lines. Then the cut line and one `INFO` line follow. The
    /// caller ends the process with exit code 0 when the report shows a cut drain. The caller also
    /// ends the process with exit code 0 when a pool close timed out (ADR-0090 (b)).
    pub fn stop(self) -> CloseReport {
        let report = self.close();
        report.log();
        tracing::info!("the server stopped");
        report
    }
}

#[cfg(test)]
mod race_tests;
