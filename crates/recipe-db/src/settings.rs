//! Connection data and settings of the database adapter.
//!
//! The defaults equal the table of `reliability.md` section 7.11.
//! A test reads that table and compares each value.

use std::fmt;
use std::time::Duration;

/// A secret text that never appears in `Debug` output and has no `Display`.
///
/// The type solves one problem: a password in a log line or in an error message.
/// Only the adapters of this crate read the text.
#[derive(Clone)]
pub struct Password(String);

impl Password {
    /// Return the secret text for the connection options of `sqlx`.
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl From<String> for Password {
    fn from(secret: String) -> Self {
        Self(secret)
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password([redacted])")
    }
}

/// The address and the login of one database connection.
#[derive(Debug, Clone)]
pub struct Connection {
    /// The host name or the IP address of the server (`db.host`).
    pub host: String,
    /// The TCP port of the server (`db.port`).
    pub port: u16,
    /// The name of the database (`db.name`).
    pub database: String,
    /// The name of the login role.
    pub user: String,
    /// The password of the login role.
    pub password: Password,
}

/// The timeouts and the size of the connection pool (`reliability.md` sections 7.4 and 7.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolSettings {
    /// The longest run time of one statement (`db.statement_timeout`).
    pub statement_timeout: Duration,
    /// The longest wait for one lock (`db.lock_timeout`).
    pub lock_timeout: Duration,
    /// The longest idle time inside an open transaction (`db.idle_in_transaction_timeout`).
    pub idle_in_transaction_timeout: Duration,
    /// The longest wait for a free connection (`db.pool_acquire_timeout`).
    pub acquire_timeout: Duration,
    /// The number of connections of the pool (`db.pool_size`).
    pub size: u32,
    /// The idle time after which the pool closes a connection (`db.pool_idle_timeout`).
    pub idle_timeout: Duration,
    /// The age after which the pool replaces a connection (`db.pool_max_lifetime`).
    pub max_lifetime: Duration,
}

impl Default for PoolSettings {
    fn default() -> Self {
        Self {
            statement_timeout: Duration::from_secs(10),
            lock_timeout: Duration::from_secs(2),
            idle_in_transaction_timeout: Duration::from_secs(15),
            acquire_timeout: Duration::from_secs(5),
            size: 10,
            idle_timeout: Duration::from_secs(10 * 60),
            max_lifetime: Duration::from_secs(30 * 60),
        }
    }
}

/// The lock timeout and the retry rule of the migrator (`reliability.md` section 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateSettings {
    /// The longest wait for one lock during a migration (`migrate.lock_timeout`).
    pub lock_timeout: Duration,
    /// The number of retries after a lock timeout (`migrate.retries`).
    pub retries: u32,
    /// The pause before a retry (`migrate.retry_pause`).
    pub retry_pause: Duration,
}

impl Default for MigrateSettings {
    fn default() -> Self {
        Self {
            lock_timeout: Duration::from_secs(2),
            retries: 10,
            retry_pause: Duration::from_secs(5),
        }
    }
}

#[cfg(test)]
mod tests;
