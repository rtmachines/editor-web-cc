//! The numbers and the checked configuration of the start of `serve` (design D4 and D7).
//!
//! [`ServeSettings`] holds every number that the start reads. Its default equals the table of
//! `reliability.md` section 7.11. [`Plan`] joins these numbers with the checked configuration, so
//! the start function reads no file and no environment variable.

use std::path::PathBuf;
use std::time::Duration;

use recipe_app::blocking::AppSettings;
use recipe_app::identity::IdentitySettings;
use recipe_app::jobs::alerts::AlertSettings;
use recipe_app::jobs::audit_chain_check::AuditChainCheckSettings;
use recipe_app::jobs::cleanup::CleanupSettings;
use recipe_app::jobs::clock_check::ClockCheckSettings;
use recipe_app::jobs::disk_check::DiskCheckSettings;
use recipe_app::operations::ConfiguredSettings;
use recipe_db::settings::PoolSettings;
use recipe_domain::identity::KeyRing;
use recipe_domain::recipe::LangCode;
use recipe_web::{CidrList, HealthSettings, WebSettings};

use super::host::ListenerKind;
use crate::config::serve::ServeConfig;
use crate::config::webhook::WebhookUrl;
use crate::listener::ListenerSettings;
use crate::watchdog::WatchdogSettings;

/// The mount point of the volume `pgdata` in the container of the server.
const PGDATA_MOUNT: &str = "/var/lib/postgresql/data";
/// The mount point of the volume `backups` in the container of the server.
const BACKUPS_MOUNT: &str = "/backups";
/// The mount point of the volume `walarchive` in the container of the server.
const WALARCHIVE_MOUNT: &str = "/wal";

/// Default of `runtime.worker_threads` (ADR-0061 (h)).
const DEFAULT_RUNTIME_WORKERS: usize = 2;
/// Default of `job.restart_delay` in seconds.
const DEFAULT_RESTART_DELAY_SECONDS: u64 = 10;
/// Default of `audit.check_time`: 03:00 UTC.
const DEFAULT_AUDIT_CHECK_HOUR: u8 = 3;
/// Default of `audit.check_time`: the minute.
const DEFAULT_AUDIT_CHECK_MINUTE: u8 = 0;
/// Default of `audit.catchup_delay` in seconds: 10 minutes.
const DEFAULT_AUDIT_CATCHUP_DELAY_SECONDS: u64 = 10 * 60;
/// Default of `audit.catchup_age` in seconds: 24 hours.
const DEFAULT_AUDIT_CATCHUP_AGE_SECONDS: u64 = 24 * 60 * 60;
/// Default of `heartbeat.interval` in seconds: 5 minutes.
const DEFAULT_HEARTBEAT_INTERVAL_SECONDS: u64 = 5 * 60;
/// Default of `clock.check_interval` in seconds: 1 hour.
const DEFAULT_CLOCK_CHECK_INTERVAL_SECONDS: u64 = 60 * 60;
/// Default of `health.disk_check_interval` in seconds.
const DEFAULT_DISK_CHECK_INTERVAL_SECONDS: u64 = 30;
/// Default of `http.accept_error_pause` in seconds.
const DEFAULT_ACCEPT_ERROR_PAUSE_SECONDS: u64 = 1;
/// Default of `shutdown.drain` in seconds.
const DEFAULT_SHUTDOWN_DRAIN_SECONDS: u64 = 30;
/// Default of `shutdown.pool_close` in seconds.
const DEFAULT_SHUTDOWN_POOL_CLOSE_SECONDS: u64 = 5;
/// Default of `shutdown.runtime_stop` in seconds.
const DEFAULT_SHUTDOWN_RUNTIME_STOP_SECONDS: u64 = 3;
/// Default of `db.health_pool_size`.
const DEFAULT_HEALTH_POOL_SIZE: u32 = 2;

/// The four times of the stop (`reliability.md` section 7.3 and 7.11).
///
/// The type solves one problem: start and stop must read the same numbers.
/// Four bare durations in a row are easy to swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownSettings {
    /// The time for the public connections and the open job runs (`shutdown.drain`).
    pub drain: Duration,
    /// The time to close one pool (`shutdown.pool_close`).
    pub pool_close: Duration,
    /// The time for the connections of the internal listener (`shutdown.internal_drain`).
    pub internal_drain: Duration,
    /// The time for the runtime to end its tasks (`shutdown.runtime_stop`).
    pub runtime_stop: Duration,
}

impl Default for ShutdownSettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            drain: Duration::from_secs(DEFAULT_SHUTDOWN_DRAIN_SECONDS),
            pool_close: Duration::from_secs(DEFAULT_SHUTDOWN_POOL_CLOSE_SECONDS),
            internal_drain: Duration::ZERO,
            runtime_stop: Duration::from_secs(DEFAULT_SHUTDOWN_RUNTIME_STOP_SECONDS),
        }
    }
}

/// Every number that the start of `serve` reads.
///
/// The type solves one problem: the start needs about thirty numbers from the table of
/// `reliability.md` section 7.11.
/// A number in the code of a step is hard to check against that table.
/// `Default` equals the table, and a test compares both.
#[derive(Debug, Clone)]
pub struct ServeSettings {
    /// The number of Tokio workers (`runtime.worker_threads`).
    pub runtime_workers: usize,
    /// The wait before a job starts again after a panic (`job.restart_delay`).
    pub job_restart_delay: Duration,
    /// The hour of the nightly audit check in UTC (`audit.check_time`).
    pub audit_check_hour: u8,
    /// The minute of the nightly audit check in UTC (`audit.check_time`).
    pub audit_check_minute: u8,
    /// The wait before the catch-up check of the audit chain (`audit.catchup_delay`).
    pub audit_catchup_delay: Duration,
    /// The age of the last audit check above which the catch-up runs (`audit.catchup_age`).
    pub audit_catchup_age: Duration,
    /// The time between two runs of `heartbeat` (`heartbeat.interval`).
    pub heartbeat_interval: Duration,
    /// The time between two runs of `clock_check` (`clock.check_interval`).
    pub clock_check_interval: Duration,
    /// The time between two runs of `disk_check` (`health.disk_check_interval`).
    pub disk_check_interval: Duration,
    /// The pause after an accept fault (`http.accept_error_pause`).
    pub accept_error_pause: Duration,
    /// The times of the stop.
    pub shutdown: ShutdownSettings,
    /// The main pool. The field `size` is `db.pool_size`.
    pub main_pool: PoolSettings,
    /// The size of the health pool (`db.health_pool_size`).
    pub health_pool_size: u32,
    /// The watchdog thread.
    pub watchdog: WatchdogSettings,
    /// The health router.
    pub health: HealthSettings,
    /// The job `cleanup`.
    pub cleanup: CleanupSettings,
    /// The job `disk_check`.
    pub disk_check: DiskCheckSettings,
    /// The job `audit_chain_check`.
    pub audit_chain_check: AuditChainCheckSettings,
    /// The job `clock_check`.
    pub clock_check: ClockCheckSettings,
    /// The job `alerts`.
    pub alerts: AlertSettings,
    /// The identity use cases.
    pub identity: IdentitySettings,
    /// The other use cases.
    pub app: AppSettings,
    /// The web layer.
    pub web: WebSettings,
    /// The mount point of the volume `pgdata`.
    pub pgdata_path: PathBuf,
    /// The mount point of the volume `backups`.
    pub backups_path: PathBuf,
    /// The mount point of the volume `walarchive`.
    pub walarchive_path: PathBuf,
}

impl Default for ServeSettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            runtime_workers: DEFAULT_RUNTIME_WORKERS,
            job_restart_delay: Duration::from_secs(DEFAULT_RESTART_DELAY_SECONDS),
            audit_check_hour: DEFAULT_AUDIT_CHECK_HOUR,
            audit_check_minute: DEFAULT_AUDIT_CHECK_MINUTE,
            audit_catchup_delay: Duration::from_secs(DEFAULT_AUDIT_CATCHUP_DELAY_SECONDS),
            audit_catchup_age: Duration::from_secs(DEFAULT_AUDIT_CATCHUP_AGE_SECONDS),
            heartbeat_interval: Duration::from_secs(DEFAULT_HEARTBEAT_INTERVAL_SECONDS),
            clock_check_interval: Duration::from_secs(DEFAULT_CLOCK_CHECK_INTERVAL_SECONDS),
            disk_check_interval: Duration::from_secs(DEFAULT_DISK_CHECK_INTERVAL_SECONDS),
            accept_error_pause: Duration::from_secs(DEFAULT_ACCEPT_ERROR_PAUSE_SECONDS),
            shutdown: ShutdownSettings::default(),
            main_pool: PoolSettings::default(),
            health_pool_size: DEFAULT_HEALTH_POOL_SIZE,
            watchdog: WatchdogSettings::default(),
            health: HealthSettings::default(),
            cleanup: CleanupSettings::default(),
            disk_check: DiskCheckSettings::default(),
            audit_chain_check: AuditChainCheckSettings::default(),
            clock_check: ClockCheckSettings::default(),
            alerts: AlertSettings::default(),
            identity: IdentitySettings::default(),
            app: AppSettings::default(),
            web: WebSettings::default(),
            pgdata_path: PathBuf::from(PGDATA_MOUNT),
            backups_path: PathBuf::from(BACKUPS_MOUNT),
            walarchive_path: PathBuf::from(WALARCHIVE_MOUNT),
        }
    }
}

impl ServeSettings {
    /// Return the settings of the health pool: the main pool with the size `db.health_pool_size`.
    #[must_use]
    pub fn health_pool(&self) -> PoolSettings {
        PoolSettings {
            size: self.health_pool_size,
            ..self.main_pool.clone()
        }
    }

    /// Return the settings of the accept loop of one listener (ADR-0087 (c)).
    ///
    /// Both listeners share the header read timeout and the accept pause. Only the drain differs.
    /// The internal listener uses `shutdown.internal_drain` and the public listener `shutdown.drain`.
    #[must_use]
    pub fn listener(&self, kind: ListenerKind) -> ListenerSettings {
        let drain_deadline = match kind {
            ListenerKind::Internal => self.shutdown.internal_drain,
            ListenerKind::Public => self.shutdown.drain,
        };
        ListenerSettings {
            header_read_timeout: self.web.body_read_timeout,
            drain_deadline,
            accept_error_pause: self.accept_error_pause,
        }
    }
}

/// The checked configuration and the numbers of one start.
///
/// The type solves one problem: the start function must run in a test with no file, no environment
/// variable, and no secret on disk. The fields are public, so a test builds a plan by hand. The
/// binary builds it with [`Plan::from_config`].
#[derive(Debug, Clone)]
pub struct Plan {
    /// The 18 values that `CompareSettings` records.
    pub configured: ConfiguredSettings,
    /// The keys of the login HMAC.
    pub login_keys: KeyRing,
    /// The signing keys of the CSRF cookie.
    pub csrf_keys: KeyRing,
    /// The webhook URL, or `None` for no webhook.
    pub webhook_url: Option<WebhookUrl>,
    /// The numbers.
    pub settings: ServeSettings,
}

impl Plan {
    /// Build a plan from checked values and the default numbers.
    ///
    /// The values override five numbers of the default: the trusted networks, the translation
    /// languages, both session timeouts, and the rule of the separation of duties.
    #[must_use]
    pub fn new(
        configured: ConfiguredSettings,
        login_keys: KeyRing,
        csrf_keys: KeyRing,
        webhook_url: Option<WebhookUrl>,
        trusted_proxies: CidrList,
        translation_langs: Vec<LangCode>,
    ) -> Self {
        let mut settings = ServeSettings::default();
        settings.web.trusted_proxies = trusted_proxies;
        settings.web.translation_langs = translation_langs;
        settings.identity.session_idle_timeout = configured.idle_timeout;
        settings.identity.session_absolute_timeout = configured.absolute_timeout;
        settings.app.approval_policy.separation_of_duties = configured.separation_of_duties;
        Self {
            configured,
            login_keys,
            csrf_keys,
            webhook_url,
            settings,
        }
    }

    /// Build the plan of a checked configuration.
    #[must_use]
    pub fn from_config(config: &ServeConfig) -> Self {
        Self::new(
            config.configured().clone(),
            config.login_keys().clone(),
            config.csrf_keys().clone(),
            config.webhook_url().cloned(),
            config.trusted_proxies().clone(),
            config.translation_langs().to_vec(),
        )
    }
}

#[cfg(test)]
mod tests;
