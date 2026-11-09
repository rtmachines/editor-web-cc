//! The production host of the start of `serve`.
//!
//! [`ProcessHost`] opens the real database pools, binds the real ports, and listens for the real
//! stop signals. The function [`run`] joins the configuration, the start, the wait for the stop
//! signal, and the stop.

use std::fmt;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use recipe_app::identity::{Argon2Hasher, Argon2Params, PasswordHasher};
use recipe_app::operations::AlertCounters;
use recipe_app::ports::{AlertWebhook, BackupDirectory, CheckResults, Clock, DiskSpace};
use recipe_db::Database;
use recipe_db::settings::{Connection, PoolSettings};
use tokio::net::TcpListener;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

use super::host::{
    AcceptTask, HasherBuild, HealthPool, Host, ListenerKind, MainPool, Signal, WatchdogControl,
};
use super::plan::Plan;
use super::server::StopCause;
use super::start::{Outcome, start};
use super::webhook_client::build_webhook;
use crate::backups::FileBackupDirectory;
use crate::check_results::FileCheckResults;
use crate::clock::SystemClock;
use crate::coded::coded;
use crate::config::serve::load;
use crate::config::webhook::WebhookUrl;
use crate::disk::SystemDiskSpace;
use crate::listener::DrainOutcome;
use crate::watchdog::{RESTART_EXIT_CODE, Watchdog, WatchdogSettings};

/// The prefix of the names of the worker threads of the runtime.
const WORKER_THREAD_NAME: &str = "recipe-worker";

/// The address that both listeners bind. The proxy reaches the server by the network of Compose.
const BIND_ADDRESS: &str = "0.0.0.0";

/// The host of the real process.
pub struct ProcessHost {
    connection: Connection,
    /// The one disk adapter of the jobs `disk_check` and `alerts` (ADR-0133).
    disk: SystemDiskSpace,
    /// The time limit of one blocking read of a volume (ADR-0133, ADR-0151 (o)).
    read_limit: Duration,
}

impl ProcessHost {
    /// Build the host for the connection of the application role.
    ///
    /// `disk_limit` is `health.disk_check_interval`. The host builds one disk adapter with it
    /// and gives the same limit to the backup directory adapter.
    #[must_use]
    pub fn new(connection: Connection, disk_limit: Duration) -> Self {
        Self {
            connection,
            disk: SystemDiskSpace::new(disk_limit),
            read_limit: disk_limit,
        }
    }

    async fn connect(&self, settings: &PoolSettings) -> Result<Database> {
        Database::connect(&self.connection, settings)
            .await
            .map_err(coded)
    }
}

#[async_trait]
impl Host for ProcessHost {
    fn webhook(
        &self,
        url: Option<&WebhookUrl>,
        ca_file: Option<&Path>,
    ) -> Result<Arc<dyn AlertWebhook>> {
        build_webhook(url, ca_file)
    }

    fn start_watchdog(
        &self,
        settings: WatchdogSettings,
        internal_port: u16,
    ) -> io::Result<Box<dyn WatchdogControl>> {
        let watchdog = Watchdog::start(settings, internal_port)?;
        Ok(Box::new(watchdog))
    }

    fn build_runtime(&self, workers: usize) -> io::Result<Runtime> {
        Builder::new_multi_thread()
            .worker_threads(workers)
            .thread_name(WORKER_THREAD_NAME)
            .enable_all()
            .build()
    }

    fn register_signals(&self) -> io::Result<Signal> {
        signal_stream()
    }

    async fn open_main_pool(&self, settings: &PoolSettings) -> Result<MainPool> {
        let database = self.connect(settings).await?;
        Ok(MainPool::from_database(&database))
    }

    async fn open_health_pool(&self, settings: &PoolSettings) -> Result<HealthPool> {
        let database = self.connect(settings).await?;
        Ok(HealthPool::from_database(&database))
    }

    fn password_hasher(&self, params: Argon2Params) -> HasherBuild {
        Box::new(move || {
            let hasher = Argon2Hasher::new(params).map_err(coded)?;
            Ok(Arc::new(hasher) as Arc<dyn PasswordHasher>)
        })
    }

    fn clock(&self) -> Arc<dyn Clock> {
        Arc::new(SystemClock)
    }

    fn disk(&self) -> Arc<dyn DiskSpace> {
        Arc::new(self.disk.clone())
    }

    fn backups(&self, directory: &Path) -> Arc<dyn BackupDirectory> {
        Arc::new(FileBackupDirectory::new(directory, self.read_limit))
    }

    fn check_results(&self, backups: &Path, walarchive: &Path) -> Arc<dyn CheckResults> {
        Arc::new(FileCheckResults::new(backups, walarchive, self.read_limit))
    }

    async fn bind(&self, _kind: ListenerKind, port: u16) -> io::Result<TcpListener> {
        TcpListener::bind((BIND_ADDRESS, port)).await
    }

    fn spawn_accept_loop(
        &self,
        _kind: ListenerKind,
        accept_loop: AcceptTask,
    ) -> JoinHandle<DrainOutcome> {
        tokio::spawn(accept_loop)
    }
}

/// Register `SIGTERM` and `SIGINT`. The future ends at the first of them.
#[cfg(unix)]
fn signal_stream() -> io::Result<Signal> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    Ok(Box::pin(async move {
        tokio::select! {
            _ = terminate.recv() => {},
            _ = interrupt.recv() => {},
        }
    }))
}

/// Register Ctrl-C. The future ends at the first press.
#[cfg(not(unix))]
fn signal_stream() -> io::Result<Signal> {
    let mut ctrl_c = tokio::signal::windows::ctrl_c()?;
    Ok(Box::pin(async move {
        let _pressed = ctrl_c.recv().await;
    }))
}

/// Run the subcommand `serve`: load the configuration, start, wait for a stop signal, and stop.
///
/// A signal before the start ends returns `Ok`. A signal after the start runs the stop of the
/// server. The function returns `Ok` also when the drain cut work or when a pool close timed out.
///
/// # Errors
///
/// Returns the error of the configuration or of the first failed step of the start. The caller
/// writes the one `ERROR` line and exits with code 1.
pub fn run(counters: AlertCounters) -> Result<()> {
    let config = load()?;
    let connection = config.app_connection()?;
    let plan = Plan::from_config(&config);
    let host = ProcessHost::new(connection, plan.settings.disk_check_interval);
    serve_with(&host, plan, counters)
}

/// An accept loop ended before the stop signal (ADR-0125).
///
/// The type solves one problem: a server without a loop cannot serve, so the process must stop and
/// tell the supervisor why. `main` writes the one `ERROR` line from the chain of this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptLoopEnded {
    /// The listener of the loop that ended.
    pub kind: ListenerKind,
    /// Whether the loop ended with a panic.
    pub panicked: bool,
}

impl fmt::Display for AcceptLoopEnded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let listener = match self.kind {
            ListenerKind::Public => "public",
            ListenerKind::Internal => "internal",
        };
        let how = if self.panicked {
            "with a panic"
        } else {
            "without a panic"
        };
        write!(
            formatter,
            "the accept loop of the {listener} listener ended before the stop signal {how}"
        )
    }
}

impl std::error::Error for AcceptLoopEnded {}

/// The exit code that a failed `serve` earns (ADR-0125 (d)).
///
/// The function solves one problem: `main` must tell a failed start (exit code 1) from an accept
/// loop that ended early (exit code 70, so Docker restarts the container). It looks for
/// [`AcceptLoopEnded`] in the whole chain of causes, also under a context.
#[must_use]
pub fn exit_code(error: &anyhow::Error) -> Option<std::process::ExitCode> {
    error
        .chain()
        .any(|cause| cause.is::<AcceptLoopEnded>())
        .then(|| std::process::ExitCode::from(RESTART_EXIT_CODE))
}

/// Start on `host`, wait for the stop signal, and stop. [`run`] calls it with the real host.
///
/// # Errors
///
/// Returns the error of the first failed step of the start. Returns [`AcceptLoopEnded`] after the
/// stop, when an accept loop ended before the signal.
pub(super) fn serve_with(host: &dyn Host, plan: Plan, counters: AlertCounters) -> Result<()> {
    match start(host, plan, counters)? {
        Outcome::Interrupted => Ok(()),
        Outcome::Running(mut server) => {
            server.wait_for_signal();
            let cause = server.stop_cause();
            let _report = server.stop();
            match cause {
                Some(StopCause::LoopEnded { kind, panicked }) => {
                    Err(AcceptLoopEnded { kind, panicked }.into())
                }
                Some(StopCause::Signal) | None => Ok(()),
            }
        }
    }
}
