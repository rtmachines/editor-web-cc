//! The connection pool and its settings (design D2 and D9).

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use recipe_app::error::StorageError;
use recipe_app::ports::{LoginAttemptStore, SecurityEventSink, SessionStore, UnitOfWork};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::error::map_error;
use crate::login_attempts::PgLoginAttemptStore;
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
        let timeouts = Arc::new(SessionTimeouts::from_settings(settings));
        let pool = PgPoolOptions::new()
            .max_connections(settings.size)
            .acquire_timeout(settings.acquire_timeout)
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
