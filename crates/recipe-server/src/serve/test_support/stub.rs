//! The stub host and the decorators that log what the start touches (design D14 row T26).
//!
//! Every effect of the start goes through [`StubHost`]. The decorators write one [`Event`] when the
//! start uses a port, so a test reads the order of the steps from one list. [`Fire`] ends the
//! start at an exact point with a stop signal.

use std::io;
use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::ThreadId;
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::identity::{Argon2Params, BlockingHasher, PasswordHasher};
use recipe_app::memory::{
    DiskReading, MemoryAuditChain, MemoryBackupDirectory, MemoryClock, MemoryDiskSpace,
    MemoryMaintenance, MemoryOperations, MemoryReadiness, MemoryStore, MemoryWebhook,
};
use recipe_app::ports::{
    AlertWebhook, BackupDirectory, Clock, DiskFault, DiskSpace, FreeSpace, Isolation,
    MaintenanceStore, OperationsReads, OperationsSnapshot, RoleCheck, SnapshotWindows, StateValue,
    TimeStateKey, Transaction, UnitOfWork,
};
use recipe_db::CloseOutcome;
use recipe_db::settings::PoolSettings;
use recipe_domain::timestamp::Timestamp;
use tokio::net::TcpListener;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::{Notify, oneshot};

use super::watch::Watch;
use super::{START_TEXT, TokioClock};
use crate::config::webhook::WebhookUrl;
use crate::serve::host::{
    HasherBuild, HealthPool, Host, ListenerKind, MainPool, PoolClose, Signal, WatchdogControl,
};
use crate::serve::plan::ServeSettings;
use crate::watchdog::WatchdogSettings;

/// The two pools of the stub.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::serve) enum Pool {
    Main,
    Health,
}

/// One use of a port by the start, in the order of the use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::serve) enum Event {
    Webhook {
        has_url: bool,
        has_ca: bool,
    },
    WatchdogStart(u16),
    Runtime(usize),
    Signals,
    OpenMain(u32),
    OpenHealth(u32),
    RoleCheck,
    Hasher,
    DiskRead,
    UnitOfWork,
    Bind(ListenerKind, u16),
    Arm,
    Disarm,
    WatchdogStop,
    StateRead,
    ClockRead,
    Snapshot,
    SessionPurge(Duration),
    PoolClosed(Pool),
    /// The stop turned readiness off. The watch writes it, see [`super::watch::Watch`].
    ReadinessOff,
    /// The stop closed the socket of a listener. The watch writes it.
    ListenerStopped(ListenerKind),
}

/// The ordered list of events. A clone shares the list.
#[derive(Clone, Default)]
pub(in crate::serve) struct Log(Arc<Mutex<Vec<Event>>>);

impl Log {
    pub(in crate::serve) fn push(&self, event: Event) {
        self.0.lock().unwrap().push(event);
    }

    pub(in crate::serve) fn events(&self) -> Vec<Event> {
        self.0.lock().unwrap().clone()
    }

    /// The index of the first `event`, or a failed test.
    pub(in crate::serve) fn index_of(&self, event: &Event) -> usize {
        let events = self.events();
        let position = events.iter().position(|seen| seen == event);
        assert!(position.is_some(), "{event:?} is not in {events:?}");
        position.unwrap_or_default()
    }

    pub(in crate::serve) fn count(&self, event: &Event) -> usize {
        self.events().iter().filter(|seen| *seen == event).count()
    }

    pub(in crate::serve) fn contains(&self, event: &Event) -> bool {
        self.events().contains(event)
    }
}

/// The places where a test can end the start with a stop signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::serve) enum Point {
    OpenMain,
    OpenHealth,
    Disk,
    UnitOfWork,
    BindInternal,
    BindPublic,
    StateRead,
}

#[derive(Default)]
struct FireState {
    point: Option<Point>,
    /// Do not yield after the signal fired, so the step ends in the same poll (finding VX10-1).
    immediate: bool,
    sender: Option<oneshot::Sender<()>>,
}

/// The slot that holds the stop signal until the start reaches the chosen point.
///
/// A step that fires the signal then yields once. The `select!` of the start sees the signal on
/// its next poll, before the step can end.
#[derive(Clone, Default)]
pub(in crate::serve) struct Fire(Arc<Mutex<FireState>>);

impl Fire {
    /// Make the signal fire when the start reaches `point`.
    pub(in crate::serve) fn at(&self, point: Point) {
        self.0.lock().unwrap().point = Some(point);
    }

    /// Whether a test chose [`Fire::at_once_at`]. The bind then must not wait, so it hides nothing.
    fn is_at_once(&self) -> bool {
        self.0.lock().unwrap().immediate
    }

    /// Make the signal fire when the start reaches `point`, and let the step end at once.
    ///
    /// The step does not yield after the signal. The start must see the signal by itself.
    pub(in crate::serve) fn at_once_at(&self, point: Point) {
        let mut state = self.0.lock().unwrap();
        state.point = Some(point);
        state.immediate = true;
    }

    /// Fire the signal now. It ends the wait of a running server.
    pub(in crate::serve) fn now(&self) {
        let sender = self.0.lock().unwrap().sender.take();
        if let Some(sender) = sender {
            // A dropped receiver means that the start is over. The signal is then not needed.
            let _ = sender.send(());
        }
    }

    /// Make the receiver of a new signal. The sender waits in the slot.
    fn open(&self) -> oneshot::Receiver<()> {
        let (sender, receiver) = oneshot::channel();
        self.0.lock().unwrap().sender = Some(sender);
        receiver
    }

    /// Fire the signal when `point` is the chosen point. Then yield once, unless the test chose
    /// [`Fire::at_once_at`].
    pub(in crate::serve) async fn reach(&self, point: Point) {
        let (sender, immediate) = {
            let mut state = self.0.lock().unwrap();
            if state.point == Some(point) {
                (state.sender.take(), state.immediate)
            } else {
                (None, false)
            }
        };
        if let Some(sender) = sender {
            let _ = sender.send(());
            if !immediate {
                tokio::task::yield_now().await;
            }
        }
    }
}

/// The ports that the stub listeners got from the system. A clone shares the list.
#[derive(Clone, Default)]
pub(in crate::serve) struct Ports(Arc<Mutex<Vec<(ListenerKind, u16)>>>);

impl Ports {
    fn record(&self, kind: ListenerKind, port: u16) {
        self.0.lock().unwrap().push((kind, port));
    }

    /// The port of the first listener of `kind`, or `None` before its bind.
    pub(in crate::serve) fn of(&self, kind: ListenerKind) -> Option<u16> {
        let ports = self.0.lock().unwrap();
        ports
            .iter()
            .find(|(bound, _)| *bound == kind)
            .map(|(_, port)| *port)
    }
}

#[derive(Default)]
struct HoldState {
    armed: AtomicBool,
    release: Notify,
}

/// A wait in the first read of the state of step 9. A test arms it and releases it later.
///
/// The wait lets a test send a request to the running internal listener while the start is
/// between step 8 and step 10 (finding CRX-S8-2).
#[derive(Clone, Default)]
pub(in crate::serve) struct Hold(Arc<HoldState>);

impl Hold {
    /// Make the next read of the state wait for [`Hold::release`].
    pub(in crate::serve) fn arm(&self) {
        self.0.armed.store(true, Ordering::SeqCst);
    }

    /// End the wait. A release before the wait is not lost.
    pub(in crate::serve) fn release(&self) {
        self.0.release.notify_one();
    }

    async fn wait(&self) {
        if self.0.armed.swap(false, Ordering::SeqCst) {
            self.0.release.notified().await;
        }
    }
}

/// The step that a test makes fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::serve) enum Fail {
    Webhook,
    Watchdog,
    Runtime,
    Signals,
    MainPool,
    HealthPool,
    Hasher,
    BindInternal,
    BindPublic,
}

/// A transaction opener that logs each `begin`.
struct LoggedUow {
    inner: MemoryStore,
    log: Log,
    fire: Fire,
}

#[async_trait]
impl UnitOfWork for LoggedUow {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError> {
        self.log.push(Event::UnitOfWork);
        self.fire.reach(Point::UnitOfWork).await;
        self.inner.begin(isolation).await
    }
}

/// The operations reads that log each call.
struct LoggedOps {
    inner: MemoryOperations,
    log: Log,
    fire: Fire,
    hold: Hold,
}

#[async_trait]
impl OperationsReads for LoggedOps {
    async fn snapshot(
        &self,
        windows: &SnapshotWindows,
    ) -> Result<OperationsSnapshot, StorageError> {
        self.log.push(Event::Snapshot);
        self.inner.snapshot(windows).await
    }

    async fn database_now(&self) -> Result<Timestamp, StorageError> {
        self.log.push(Event::ClockRead);
        self.inner.database_now().await
    }

    async fn role_check(&self) -> Result<RoleCheck, StorageError> {
        self.log.push(Event::RoleCheck);
        self.inner.role_check().await
    }

    async fn state(&self, key: TimeStateKey) -> Result<Option<Timestamp>, StorageError> {
        self.log.push(Event::StateRead);
        self.fire.reach(Point::StateRead).await;
        self.hold.wait().await;
        self.inner.state(key).await
    }
}

/// The maintenance writes that log the timeout of each session purge.
struct LoggedMaintenance {
    inner: MemoryMaintenance,
    log: Log,
}

#[async_trait]
impl MaintenanceStore for LoggedMaintenance {
    async fn delete_expired_sessions(
        &self,
        absolute_timeout: Duration,
    ) -> Result<u64, StorageError> {
        self.log.push(Event::SessionPurge(absolute_timeout));
        self.inner.delete_expired_sessions(absolute_timeout).await
    }

    async fn purge_login_attempts(
        &self,
        retention: Duration,
        batch: u32,
    ) -> Result<u64, StorageError> {
        self.inner.purge_login_attempts(retention, batch).await
    }

    async fn purge_security_events(&self) -> Result<u64, StorageError> {
        self.inner.purge_security_events().await
    }

    async fn record_state(&self, value: &StateValue) -> Result<(), StorageError> {
        self.inner.record_state(value).await
    }
}

/// A disk reader that logs each read.
struct LoggedDisk {
    inner: MemoryDiskSpace,
    log: Log,
    fire: Fire,
    panics: bool,
}

#[async_trait]
impl DiskSpace for LoggedDisk {
    async fn free_space(&self, path: &Path) -> Result<FreeSpace, DiskFault> {
        self.log.push(Event::DiskRead);
        self.fire.reach(Point::Disk).await;
        if self.panics {
            // The test makes the first run of `disk_check` panic. The supervisor must catch it.
            u8::try_from(-1_i32).unwrap();
        }
        self.inner.free_space(path).await
    }
}

/// A pool close that logs itself.
struct LoggedCloser {
    pool: Pool,
    log: Log,
    watch: Watch,
    outcome: CloseOutcome,
}

#[async_trait]
impl PoolClose for LoggedCloser {
    async fn close(&self, _limit: Duration) -> CloseOutcome {
        self.watch.observe();
        self.log.push(Event::PoolClosed(self.pool));
        self.outcome
    }
}

/// A watchdog handle that logs each command.
struct LoggedWatchdog {
    log: Log,
    watch: Watch,
    panics: bool,
}

impl WatchdogControl for LoggedWatchdog {
    fn arm(&self) {
        self.log.push(Event::Arm);
    }

    fn disarm(&self) {
        self.watch.observe();
        self.log.push(Event::Disarm);
    }

    fn stop(self: Box<Self>) -> std::thread::Result<()> {
        self.watch.observe();
        self.log.push(Event::WatchdogStop);
        if self.panics {
            return Err(Box::new("the watchdog thread panicked"));
        }
        Ok(())
    }
}

/// The host of a test: memory adapters behind decorators that write the event log.
pub(in crate::serve) struct StubHost {
    pub log: Log,
    pub store: MemoryStore,
    pub operations: MemoryOperations,
    pub maintenance: MemoryMaintenance,
    pub audit_chain: MemoryAuditChain,
    pub disk: MemoryDiskSpace,
    pub backups: MemoryBackupDirectory,
    pub webhook: MemoryWebhook,
    pub readiness: MemoryReadiness,
    pub fire: Fire,
    /// The wait in the first read of the state of step 9.
    pub hold: Hold,
    /// The ports of the listeners that the stub bound.
    pub ports: Ports,
    /// The observer of the stop. A test enables it with the readiness handle of the server.
    pub watch: Watch,
    pub fail: Option<Fail>,
    pub pool_outcome: CloseOutcome,
    pub watchdog_panics: bool,
    /// Make every read of the free space panic.
    pub disk_panics: bool,
    /// Use a runtime with real time. The default runtime has a paused clock.
    pub realtime: bool,
    hasher_thread: Arc<Mutex<Option<ThreadId>>>,
    clock: OnceLock<Arc<TokioClock>>,
}

impl StubHost {
    /// A host with a healthy system: 50 percent free space on the three volumes.
    pub(in crate::serve) fn new() -> Self {
        let start = TimeStateKey::parse_text(START_TEXT).unwrap();
        let store = MemoryStore::with_clock(MemoryClock::new(*start.as_offset_date_time()));
        let disk = MemoryDiskSpace::default();
        let settings = ServeSettings::default();
        let half = DiskReading::Space(FreeSpace {
            available_blocks: 50,
            total_blocks: 100,
        });
        for path in [
            &settings.pgdata_path,
            &settings.backups_path,
            &settings.walarchive_path,
        ] {
            disk.set(path, half);
        }
        let log = Log::default();
        let ports = Ports::default();
        Self {
            watch: Watch::new(log.clone(), ports.clone()),
            log,
            operations: MemoryOperations::new(store.clone()),
            maintenance: MemoryMaintenance::new(store.clone()),
            audit_chain: MemoryAuditChain::new(store.clone()),
            store,
            disk,
            backups: MemoryBackupDirectory::default(),
            webhook: MemoryWebhook::default(),
            readiness: MemoryReadiness::default(),
            fire: Fire::default(),
            hold: Hold::default(),
            ports,
            fail: None,
            pool_outcome: CloseOutcome::Closed,
            watchdog_panics: false,
            disk_panics: false,
            realtime: false,
            hasher_thread: Arc::default(),
            clock: OnceLock::new(),
        }
    }

    /// Set the reading of the free space of `pgdata` to `percent` free.
    pub(in crate::serve) fn set_pgdata_free(&self, percent: u64) {
        let reading = DiskReading::Space(FreeSpace {
            available_blocks: percent,
            total_blocks: 100,
        });
        self.disk
            .set(&ServeSettings::default().pgdata_path, reading);
    }

    /// The thread that built the hasher, or `None` before the build.
    pub(in crate::serve) fn hasher_thread(&self) -> Option<ThreadId> {
        *self.hasher_thread.lock().unwrap()
    }

    fn fails(&self, step: Fail) -> bool {
        self.fail == Some(step)
    }

    fn closer(&self, pool: Pool) -> Arc<dyn PoolClose> {
        Arc::new(LoggedCloser {
            pool,
            log: self.log.clone(),
            watch: self.watch.clone(),
            outcome: self.pool_outcome,
        })
    }
}

#[async_trait]
impl Host for StubHost {
    fn webhook(
        &self,
        url: Option<&WebhookUrl>,
        ca_file: Option<&Path>,
    ) -> Result<Arc<dyn AlertWebhook>> {
        self.log.push(Event::Webhook {
            has_url: url.is_some(),
            has_ca: ca_file.is_some(),
        });
        if self.fails(Fail::Webhook) {
            return Err(anyhow!(
                "the file of alerts.webhook_ca_file has no certificate"
            ));
        }
        Ok(Arc::new(self.webhook.clone()))
    }

    fn start_watchdog(
        &self,
        _settings: WatchdogSettings,
        internal_port: u16,
    ) -> io::Result<Box<dyn WatchdogControl>> {
        self.log.push(Event::WatchdogStart(internal_port));
        if self.fails(Fail::Watchdog) {
            return Err(io::Error::other("no thread"));
        }
        Ok(Box::new(LoggedWatchdog {
            log: self.log.clone(),
            watch: self.watch.clone(),
            panics: self.watchdog_panics,
        }))
    }

    fn build_runtime(&self, workers: usize) -> io::Result<Runtime> {
        self.log.push(Event::Runtime(workers));
        if self.fails(Fail::Runtime) {
            return Err(io::Error::other("no workers"));
        }
        let mut builder = Builder::new_current_thread();
        builder.enable_all();
        if !self.realtime {
            builder.start_paused(true);
        }
        builder.build()
    }

    fn register_signals(&self) -> io::Result<Signal> {
        self.log.push(Event::Signals);
        if self.fails(Fail::Signals) {
            return Err(io::Error::other("no handler"));
        }
        let receiver = self.fire.open();
        Ok(Box::pin(async move {
            // A dropped sender means that no test fires the signal.
            if receiver.await.is_err() {
                std::future::pending::<()>().await;
            }
        }))
    }

    async fn open_main_pool(&self, settings: &PoolSettings) -> Result<MainPool> {
        self.log.push(Event::OpenMain(settings.size));
        self.fire.reach(Point::OpenMain).await;
        if self.fails(Fail::MainPool) {
            return Err(anyhow!("the main pool is not open"));
        }
        Ok(MainPool {
            unit_of_work: Arc::new(LoggedUow {
                inner: self.store.clone(),
                log: self.log.clone(),
                fire: self.fire.clone(),
            }),
            security_events: Arc::new(self.store.sink()),
            sessions: Arc::new(self.store.clone()),
            login_attempts: Arc::new(self.store.clone()),
            maintenance: Arc::new(LoggedMaintenance {
                inner: self.maintenance.clone(),
                log: self.log.clone(),
            }),
            audit_chain: Arc::new(self.audit_chain.clone()),
            operations: Arc::new(LoggedOps {
                inner: self.operations.clone(),
                log: self.log.clone(),
                fire: self.fire.clone(),
                hold: self.hold.clone(),
            }),
            closer: self.closer(Pool::Main),
        })
    }

    async fn open_health_pool(&self, settings: &PoolSettings) -> Result<HealthPool> {
        self.log.push(Event::OpenHealth(settings.size));
        self.fire.reach(Point::OpenHealth).await;
        if self.fails(Fail::HealthPool) {
            return Err(anyhow!("the health pool is not open"));
        }
        Ok(HealthPool {
            readiness: Arc::new(self.readiness.clone()),
            closer: self.closer(Pool::Health),
        })
    }

    fn password_hasher(&self, _params: Argon2Params) -> HasherBuild {
        let log = self.log.clone();
        let thread = Arc::clone(&self.hasher_thread);
        let refuses = self.fails(Fail::Hasher);
        Box::new(move || {
            log.push(Event::Hasher);
            *thread.lock().unwrap() = Some(std::thread::current().id());
            if refuses {
                return Err(anyhow!("Argon2 refuses the parameters"));
            }
            let hasher = BlockingHasher::new();
            hasher.release();
            Ok(Arc::new(hasher) as Arc<dyn PasswordHasher>)
        })
    }

    fn clock(&self) -> Arc<dyn Clock> {
        let clock = self.clock.get_or_init(|| TokioClock::at(START_TEXT));
        Arc::clone(clock) as Arc<dyn Clock>
    }

    fn disk(&self) -> Arc<dyn DiskSpace> {
        Arc::new(LoggedDisk {
            inner: self.disk.clone(),
            log: self.log.clone(),
            fire: self.fire.clone(),
            panics: self.disk_panics,
        })
    }

    fn backups(&self, _directory: &Path) -> Arc<dyn BackupDirectory> {
        Arc::new(self.backups.clone())
    }

    async fn bind(&self, kind: ListenerKind, port: u16) -> io::Result<TcpListener> {
        if kind == ListenerKind::Public && !self.fire.is_at_once() {
            // The wait lets every ready task run first. The loops of step 9 then show before it.
            // A test with a signal that fires at once skips it, because the wait would let the
            // start see the signal and hide the order of the checks.
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        self.log.push(Event::Bind(kind, port));
        let (point, step) = match kind {
            ListenerKind::Internal => (Point::BindInternal, Fail::BindInternal),
            ListenerKind::Public => (Point::BindPublic, Fail::BindPublic),
        };
        self.fire.reach(point).await;
        if self.fails(step) {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "the port is in use",
            ));
        }
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        self.ports.record(kind, listener.local_addr()?.port());
        Ok(listener)
    }
}
