//! The ports of the start of `serve` (design D4, test row T26).
//!
//! The start order of ADR-0061 touches the operating system, the database, and the network. The
//! trait [`Host`] holds every one of these effects, so a test runs the ten steps with stubs and
//! observes their order. The production host is `ProcessHost`.

use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use recipe_app::identity::{Argon2Params, PasswordHasher};
use recipe_app::ports::{
    AlertWebhook, AuditChainReads, BackupDirectory, Clock, DiskSpace, LoginAttemptStore,
    MaintenanceStore, OperationsReads, ReadinessProbe, SecurityEventSink, SessionStore, UnitOfWork,
};
use recipe_db::settings::PoolSettings;
use recipe_db::{CloseOutcome, Database};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

use crate::config::webhook::WebhookUrl;
use crate::watchdog::{Watchdog, WatchdogSettings};

/// The future that ends at the first stop signal (`SIGTERM` or Ctrl-C).
///
/// The type solves one problem: a test must end the start at an exact point.
/// The operating system cannot deliver a signal at that point.
/// Step 4 builds the future inside the runtime.
pub type Signal = Pin<Box<dyn Future<Output = ()> + Send>>;

/// The work that builds the Argon2id hasher (step 6).
///
/// The type solves one problem: the build hashes once, so it must run in `spawn_blocking`. A
/// borrowed host cannot cross that call, so the host hands over an owned closure.
pub type HasherBuild = Box<dyn FnOnce() -> Result<Arc<dyn PasswordHasher>> + Send>;

/// The two listeners of the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerKind {
    /// The listener of `/healthz` and `/readyz` (step 8).
    Internal,
    /// The listener of the application (step 10).
    Public,
}

/// The commands that the start sends to the watchdog thread (ADR-0061 (g)).
///
/// The trait solves one problem: a test must see the moment of the arm without a thread that probes
/// a socket. [`Watchdog`] is the production implementation.
pub trait WatchdogControl: Send {
    /// Start to count the failed probes.
    fn arm(&self);

    /// Stop to count the failed probes. The thread stays alive.
    fn disarm(&self);

    /// End the thread and wait for it.
    ///
    /// # Errors
    ///
    /// Returns the panic payload when the thread panicked. The panic hook has logged it.
    fn stop(self: Box<Self>) -> std::thread::Result<()>;
}

impl WatchdogControl for Watchdog {
    fn arm(&self) {
        Self::arm(self);
    }

    fn disarm(&self) {
        Self::disarm(self);
    }

    fn stop(self: Box<Self>) -> std::thread::Result<()> {
        Self::stop(*self)
    }
}

/// The close of one connection pool (`shutdown.pool_close`, ADR-0087).
///
/// The trait solves one problem: the stop closes two pools, and a test has no `sqlx` pool.
#[async_trait]
pub trait PoolClose: Send + Sync {
    /// Close the pool and wait for its connections, but not longer than `limit`.
    async fn close(&self, limit: Duration) -> CloseOutcome;
}

#[async_trait]
impl PoolClose for Database {
    async fn close(&self, limit: Duration) -> CloseOutcome {
        Self::close(self, limit).await
    }
}

/// The ports of the main pool (`db.pool_size`) and the handle that closes it.
///
/// The type solves one problem: the start wires ten adapters from one pool, and a long list of
/// arguments hides which adapter is which. A test fills the fields with memory adapters.
pub struct MainPool {
    /// The opener of transactions.
    pub unit_of_work: Arc<dyn UnitOfWork>,
    /// The sink of security events.
    pub security_events: Arc<dyn SecurityEventSink>,
    /// The session store.
    pub sessions: Arc<dyn SessionStore>,
    /// The store of T1 of `Login`.
    pub login_attempts: Arc<dyn LoginAttemptStore>,
    /// The maintenance writes of the jobs.
    pub maintenance: Arc<dyn MaintenanceStore>,
    /// The reads of the audit chain.
    pub audit_chain: Arc<dyn AuditChainReads>,
    /// The reads of the jobs `alerts` and `clock_check`, and the role check.
    pub operations: Arc<dyn OperationsReads>,
    /// The close of the pool.
    pub closer: Arc<dyn PoolClose>,
}

impl MainPool {
    /// Take every port from one database pool.
    #[must_use]
    pub fn from_database(database: &Database) -> Self {
        Self {
            unit_of_work: database.unit_of_work(),
            security_events: database.security_events(),
            sessions: database.session_store(),
            login_attempts: database.login_attempts(),
            maintenance: database.maintenance(),
            audit_chain: database.audit_chain(),
            operations: database.operations(),
            closer: Arc::new(database.clone()),
        }
    }
}

/// The readiness probe on the health pool (`db.health_pool_size`) and the handle that closes it.
pub struct HealthPool {
    /// The probe of `/readyz`.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// The close of the pool.
    pub closer: Arc<dyn PoolClose>,
}

impl HealthPool {
    /// Take the probe from one database pool.
    #[must_use]
    pub fn from_database(database: &Database) -> Self {
        Self {
            readiness: database.readiness(),
            closer: Arc::new(database.clone()),
        }
    }
}

/// Every effect of the start order.
///
/// The trait solves one problem: the order of the ten steps of ADR-0061 is the behavior under test.
/// A test cannot open a real database, bind a real port, or wait for a real signal.
/// Each method belongs to one step. The methods without `async` run in synchronous code.
#[async_trait]
pub trait Host: Send + Sync {
    /// Step 2: build the webhook client from the checked configuration.
    ///
    /// `ca_file` is the path in `alerts.webhook_ca_file`. `url` is the URL of the webhook file.
    ///
    /// # Errors
    ///
    /// Returns an error that names the key `alerts.webhook_ca_file` and never a path.
    fn webhook(
        &self,
        url: Option<&WebhookUrl>,
        ca_file: Option<&Path>,
    ) -> Result<Arc<dyn AlertWebhook>>;

    /// Step 3: start the watchdog thread, not armed. It probes `/healthz` on `internal_port`.
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system when it cannot start the thread.
    fn start_watchdog(
        &self,
        settings: WatchdogSettings,
        internal_port: u16,
    ) -> io::Result<Box<dyn WatchdogControl>>;

    /// Step 3: build the runtime with `workers` Tokio workers.
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system when it cannot start the workers.
    fn build_runtime(&self, workers: usize) -> io::Result<Runtime>;

    /// Step 4: register the signal stream. The runtime of step 3 is the current runtime.
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system when it cannot register a handler.
    fn register_signals(&self) -> io::Result<Signal>;

    /// Step 5: open the main pool.
    ///
    /// # Errors
    ///
    /// Returns an error that never holds the password.
    async fn open_main_pool(&self, settings: &PoolSettings) -> Result<MainPool>;

    /// Step 5: open the health pool.
    ///
    /// # Errors
    ///
    /// Returns an error that never holds the password.
    async fn open_health_pool(&self, settings: &PoolSettings) -> Result<HealthPool>;

    /// Step 6: return the work that builds the Argon2id hasher. The caller runs it off the runtime.
    ///
    /// The build computes one dummy hash, which takes time. The work returns an error when
    /// Argon2 refuses the cost parameters.
    fn password_hasher(&self, params: Argon2Params) -> HasherBuild;

    /// The clock of the host.
    fn clock(&self) -> Arc<dyn Clock>;

    /// The reader of the free disk space.
    fn disk(&self) -> Arc<dyn DiskSpace>;

    /// The reader of the backup directory at `directory`.
    fn backups(&self, directory: &Path) -> Arc<dyn BackupDirectory>;

    /// Steps 8 and 10: bind a listener on `0.0.0.0` and `port`.
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system, for example when the port is in use.
    async fn bind(&self, kind: ListenerKind, port: u16) -> io::Result<TcpListener>;
}
