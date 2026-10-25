//! The running server and its stop.
//!
//! [`Server`] is what a successful start returns. It owns every resource that step 3 to step 10
//! opened. [`Server::wait_for_signal`] waits for the stop signal. [`Server::stop`] closes the
//! resources in the stop order and writes the log lines of the stop.

use std::sync::Arc;

use recipe_db::CloseOutcome;
use recipe_web::{HealthState, ReadOnlyMode};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
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
}

impl ListenerHandle {
    /// Join the parts of one listener.
    #[must_use]
    pub(super) const fn new(
        kind: ListenerKind,
        stop: CancellationToken,
        task: JoinHandle<DrainOutcome>,
    ) -> Self {
        Self { kind, stop, task }
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

    /// Take the task of the accept loop. Await it for the drain result.
    #[must_use]
    pub fn into_task(self) -> JoinHandle<DrainOutcome> {
        self.task
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
    /// Write the cut line of the report (ADR-0088).
    ///
    /// A cut gets one `WARN` line with the three counts. A clean report writes nothing. The close
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
    /// The stop signal of step 4. It is `None` after the signal ended a wait.
    pub signal: Option<Signal>,
    /// The public listener of step 10.
    pub public: ListenerHandle,
    /// The internal listener of step 8.
    pub internal: ListenerHandle,
    /// The supervisor with the loops of step 9.
    pub supervisor: Supervisor,
    /// The close of the main pool.
    pub main_pool: Arc<dyn PoolClose>,
    /// The close of the health pool.
    pub health_pool: Arc<dyn PoolClose>,
    /// The handle of the watchdog thread. It is armed.
    pub watchdog: Box<dyn WatchdogControl>,
    /// The readiness handle. `begin_shutdown` turns `/readyz` off.
    pub health: HealthState,
    /// The flag of the read-only mode.
    pub read_only: ReadOnlyMode,
    /// The times of the stop.
    pub shutdown: ShutdownSettings,
}

/// A server that passed all ten steps of the start.
///
/// The fields are private. [`Server::wait_for_signal`] blocks until the stop signal fires. Then
/// [`Server::stop`] closes the server.
pub struct Server {
    parts: ServerParts,
}

impl Server {
    /// Wrap the parts of a finished start.
    #[must_use]
    pub(super) const fn new(parts: ServerParts) -> Self {
        Self { parts }
    }

    /// The runtime of the server.
    #[must_use]
    pub const fn runtime(&self) -> &Runtime {
        &self.parts.runtime
    }

    /// The readiness handle.
    #[must_use]
    pub const fn health(&self) -> &HealthState {
        &self.parts.health
    }

    /// The flag of the read-only mode.
    #[must_use]
    pub const fn read_only(&self) -> &ReadOnlyMode {
        &self.parts.read_only
    }

    /// Wait on the runtime until the stop signal fires.
    ///
    /// The signal fires once. A call after the signal returns at once.
    pub fn wait_for_signal(&mut self) {
        if let Some(signal) = self.parts.signal.take() {
            self.parts.runtime.block_on(signal);
        }
    }

    /// Close everything in the stop order of ADR-0087 and ADR-0090.
    ///
    /// The method writes the `WARN` line of each pool that timed out, before it stops the internal
    /// listener (ADR-0090 (b)). [`Server::stop`] writes the cut line and the `INFO` line.
    #[must_use]
    pub fn close(self) -> CloseReport {
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
