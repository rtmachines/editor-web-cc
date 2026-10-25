//! The production host of the start of `serve`.
//!
//! [`ProcessHost`] opens the real database pools, binds the real ports, and listens for the real
//! stop signals. The function [`run`] joins the configuration, the start, the wait for the stop
//! signal, and the stop.

use std::io;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use recipe_app::identity::{Argon2Hasher, Argon2Params, PasswordHasher};
use recipe_app::operations::AlertCounters;
use recipe_app::ports::{AlertWebhook, BackupDirectory, Clock, DiskSpace};
use recipe_db::Database;
use recipe_db::settings::{Connection, PoolSettings};
use tokio::net::TcpListener;
use tokio::runtime::{Builder, Runtime};

use super::host::{HasherBuild, HealthPool, Host, ListenerKind, MainPool, Signal, WatchdogControl};
use super::plan::Plan;
use super::start::{Outcome, start};
use super::webhook_client::build_webhook;
use crate::backups::FileBackupDirectory;
use crate::clock::SystemClock;
use crate::coded::coded;
use crate::config::serve::load;
use crate::config::webhook::WebhookUrl;
use crate::disk::SystemDiskSpace;
use crate::watchdog::{Watchdog, WatchdogSettings};

/// The prefix of the names of the worker threads of the runtime.
const WORKER_THREAD_NAME: &str = "recipe-worker";

/// The address that both listeners bind. The proxy reaches the server by the network of Compose.
const BIND_ADDRESS: &str = "0.0.0.0";

/// The host of the real process.
pub struct ProcessHost {
    connection: Connection,
}

impl ProcessHost {
    /// Build the host for the connection of the application role.
    #[must_use]
    pub const fn new(connection: Connection) -> Self {
        Self { connection }
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
        Arc::new(SystemDiskSpace)
    }

    fn backups(&self, directory: &Path) -> Arc<dyn BackupDirectory> {
        Arc::new(FileBackupDirectory::new(directory))
    }

    async fn bind(&self, _kind: ListenerKind, port: u16) -> io::Result<TcpListener> {
        TcpListener::bind((BIND_ADDRESS, port)).await
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
    let host = ProcessHost::new(connection);
    serve_with(&host, plan, counters)
}

/// Start on `host`, wait for the stop signal, and stop. [`run`] calls it with the real host.
pub(super) fn serve_with(host: &dyn Host, plan: Plan, counters: AlertCounters) -> Result<()> {
    match start(host, plan, counters)? {
        Outcome::Interrupted => Ok(()),
        Outcome::Running(mut server) => {
            server.wait_for_signal();
            let _report = server.stop();
            Ok(())
        }
    }
}
