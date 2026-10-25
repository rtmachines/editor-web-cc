//! The connection pool and its settings (design D2 and D9).

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use recipe_app::error::StorageError;
use recipe_app::ports::{
    AuditChainReads, LoginAttemptStore, MaintenanceStore, OperationsReads, ReadinessProbe,
    SecurityEventSink, SessionStore, UnitOfWork,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::audit_chain::PgAuditChainReads;
use crate::error::map_error;
use crate::login_attempts::PgLoginAttemptStore;
use crate::maintenance::PgMaintenanceStore;
use crate::operations::{PgOperationsReads, PgReadinessProbe};
use crate::options::connect_options;
use crate::security_events::PgSecurityEventSink;
use crate::session_store::PgSessionStore;
use crate::settings::{Connection, PoolSettings};
use crate::unit_of_work::PgUnitOfWork;

/// The statement that sets the four session settings of every new connection.
///
/// `set_config` takes the values as binds, so no value becomes part of the SQL text.
const SESSION_SETTINGS_SQL: &str = "SELECT \
     set_config('statement_timeout', $1, false), \
     set_config('lock_timeout', $2, false), \
     set_config('idle_in_transaction_session_timeout', $3, false), \
     set_config('TimeZone', 'UTC', false)";

/// The three timeouts as text for `set_config`.
struct SessionTimeouts {
    statement: String,
    lock: String,
    idle_in_transaction: String,
}

impl SessionTimeouts {
    fn from_settings(settings: &PoolSettings) -> Self {
        Self {
            statement: millis_text(settings.statement_timeout),
            lock: millis_text(settings.lock_timeout),
            idle_in_transaction: millis_text(settings.idle_in_transaction_timeout),
        }
    }
}

/// Write a duration in the form `1500ms` that PostgreSQL reads.
fn millis_text(duration: Duration) -> String {
    format!("{}ms", duration.as_millis())
}

/// The PostgreSQL connection pool of the application.
///
/// The type solves one problem: every adapter of this crate needs the same pool, and no
/// `sqlx` type may leave the crate (design D2). It owns the pool and gives the ports of
/// `recipe-app` as trait objects.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Database").finish_non_exhaustive()
    }
}

impl Database {
    /// Open the pool and one connection to the database.
    ///
    /// Each new connection sets `statement_timeout`, `lock_timeout`,
    /// `idle_in_transaction_session_timeout`, and `TimeZone = 'UTC'`. The pool tests a
    /// connection before it hands it out (`reliability.md` sections 7.4 and 7.5). The connection
    /// never logs the text of a statement.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Unavailable` when the server does not answer within
    /// `acquire_timeout`, and `StorageError::Other` for every other failure.
    pub async fn connect(
        target: &Connection,
        settings: &PoolSettings,
    ) -> Result<Self, StorageError> {
        let pool = pool_options(settings)
            .connect_with(connect_options(target))
            .await
            .map_err(connect_error)?;
        Ok(Self { pool })
    }

    /// Return the opener of transactions. Every transaction uses one connection of the pool.
    #[must_use]
    pub fn unit_of_work(&self) -> Arc<dyn UnitOfWork> {
        Arc::new(PgUnitOfWork::new(self.pool.clone()))
    }

    /// Return the sink of security events. Every `record` call runs its own short transaction.
    #[must_use]
    pub fn security_events(&self) -> Arc<dyn SecurityEventSink> {
        Arc::new(PgSecurityEventSink::new(self.pool.clone()))
    }

    /// Return the session store. `authenticate` runs one statement on the pool.
    #[must_use]
    pub fn session_store(&self) -> Arc<dyn SessionStore> {
        Arc::new(PgSessionStore::new(self.pool.clone()))
    }

    /// Return the store of T1 of `Login`. `begin_attempt` runs its own transaction.
    #[must_use]
    pub fn login_attempts(&self) -> Arc<dyn LoginAttemptStore> {
        Arc::new(PgLoginAttemptStore::new(self.pool.clone()))
    }

    /// Return the store of the maintenance writes of the supervised jobs. Every method runs one
    /// statement in its own short transaction.
    #[must_use]
    pub fn maintenance(&self) -> Arc<dyn MaintenanceStore> {
        Arc::new(PgMaintenanceStore::new(self.pool.clone()))
    }

    /// Return the reads of the job `audit_chain_check`. Every method runs one statement in its own
    /// short `READ ONLY` transaction.
    #[must_use]
    pub fn audit_chain(&self) -> Arc<dyn AuditChainReads> {
        Arc::new(PgAuditChainReads::new(self.pool.clone()))
    }

    /// Return the reads of the jobs `alerts` and `clock_check` and of the role check. The
    /// snapshot runs in one `READ ONLY` transaction. The other methods run one statement.
    #[must_use]
    pub fn operations(&self) -> Arc<dyn OperationsReads> {
        Arc::new(PgOperationsReads::new(self.pool.clone()))
    }

    /// Return the readiness probe of `/readyz`. Call it on the `Database` of the health pool
    /// (design D3), so a busy request pool cannot hide a fault.
    #[must_use]
    pub fn readiness(&self) -> Arc<dyn ReadinessProbe> {
        Arc::new(PgReadinessProbe::new(self.pool.clone()))
    }

    /// Close the pool and wait for its connections, but not longer than `limit`.
    ///
    /// The pool refuses new work at once. The call then closes the idle connections and waits
    /// until each checked-out connection returns or closes (ADR-0087). The caller sets `limit`
    /// (`shutdown.pool_close`). The function chooses no number.
    ///
    /// The result is [`CloseOutcome::Closed`] when every connection closed within `limit`. It is
    /// [`CloseOutcome::TimedOut`] otherwise. Then a task that does not yield can keep its
    /// connection until the process exits. The pool stays closed in both cases, so a second call
    /// is safe. One `Database` owns one pool, so the caller closes the request pool and the health
    /// pool with one call each. After `TimedOut`, `serve` writes one `WARN` line and exits with
    /// code 0 (ADR-0090 (b)).
    pub async fn close(&self, limit: Duration) -> CloseOutcome {
        match tokio::time::timeout(limit, self.pool.close()).await {
            Ok(()) => CloseOutcome::Closed,
            Err(_elapsed) => CloseOutcome::TimedOut,
        }
    }
}

/// How [`Database::close`] ended.
///
/// The type solves one problem: the stop sequence must know whether a connection of an aborted
/// task is still open when the limit ends (ADR-0087).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseOutcome {
    /// Every connection returned and closed within the limit.
    Closed,
    /// The limit ended while a connection was still checked out. The pool refuses new work.
    TimedOut,
}

/// Build the pool options of the settings.
///
/// The pool uses `wait_warn` as the threshold `acquire_slow_threshold` of `sqlx`.
/// The pool measures the whole acquire. That time holds the wait for a free connection, a new
/// connect, and the check before use.
/// An acquire above the threshold makes the pool write one `WARN` event (`reliability.md`
/// section 7.5).
fn pool_options(settings: &PoolSettings) -> PgPoolOptions {
    let timeouts = Arc::new(SessionTimeouts::from_settings(settings));
    PgPoolOptions::new()
        .max_connections(settings.size)
        .acquire_timeout(settings.acquire_timeout)
        .acquire_slow_threshold(settings.wait_warn)
        .idle_timeout(Some(settings.idle_timeout))
        .max_lifetime(Some(settings.max_lifetime))
        .test_before_acquire(true)
        .after_connect(move |connection, _metadata| {
            let timeouts = Arc::clone(&timeouts);
            Box::pin(async move {
                sqlx::query(SESSION_SETTINGS_SQL)
                    .bind(&timeouts.statement)
                    .bind(&timeouts.lock)
                    .bind(&timeouts.idle_in_transaction)
                    .execute(connection)
                    .await
                    .map(|_| ())
            })
        })
}

/// Map the failure of the first connection.
///
/// The pool retries a refused connection until `acquire_timeout` ends. A pool timeout at this
/// point therefore means that the server did not answer, not that the pool is busy.
fn connect_error(error: sqlx::Error) -> StorageError {
    match map_error(error) {
        StorageError::PoolTimeout => StorageError::Unavailable,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::pool_options;
    use crate::settings::PoolSettings;

    #[test]
    fn the_pool_takes_wait_warn_as_the_slow_acquire_threshold() {
        let settings = PoolSettings {
            wait_warn: Duration::from_millis(1234),
            ..PoolSettings::default()
        };

        let options = pool_options(&settings);

        assert_eq!(
            options.get_acquire_slow_threshold(),
            Duration::from_millis(1234)
        );
    }
}
