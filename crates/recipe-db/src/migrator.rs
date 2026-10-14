//! The migrator (design D10, ADR-0031).
//!
//! The migrator solves two problems. Two migrators must not run at the same time, and a lock wait
//! of a migration must not hang the deployment. `sqlx` takes an advisory lock. A session
//! `lock_timeout` ends each wait, and the retry loop starts the migration again.

#[cfg(feature = "test-support")]
use std::path::Path;

use recipe_app::error::StorageError;
use sqlx::migrate::{MigrateError, Migrator};
use sqlx::{ConnectOptions as _, Connection as _, PgConnection};

use crate::error::{is_lock_timeout, map_error};
use crate::options::connect_options;
use crate::settings::{Connection, MigrateSettings};

/// Apply the migrations of the directory `migrations/` that this crate embeds.
///
/// Each attempt opens one new connection without a pool and sets `lock_timeout` on it.
/// An attempt that fails with SQLSTATE `55P03` closes its connection. The function then waits
/// `retry_pause` and tries again, at most `retries` times. A migration that it applied before
/// stays applied. The last failure maps to `StorageError`: `E-DB-001` for a lock timeout.
///
/// # Errors
///
/// Returns `StorageError::Deadlock` when the last attempt ends in a lock timeout,
/// `StorageError::Unavailable` when the server does not answer, and `StorageError::Other`
/// for every other failure.
pub async fn migrate(target: &Connection, settings: &MigrateSettings) -> Result<(), StorageError> {
    static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");
    run_with_retries(target, settings, &MIGRATOR).await
}

/// Apply the migration files of a directory that the program reads at run time.
///
/// The storage tests use it to run the retry loop with small migrations. It has the same rules
/// and the same errors as [`migrate`]. The feature `test-support` enables it. It solves one
/// problem: a production build must not offer a way to run migration files from any directory.
///
/// # Errors
///
/// See [`migrate`]. A directory that cannot be read gives `StorageError::Other`.
#[cfg(feature = "test-support")]
pub async fn migrate_from_directory(
    target: &Connection,
    settings: &MigrateSettings,
    directory: &Path,
) -> Result<(), StorageError> {
    let migrator = Migrator::new(directory)
        .await
        .map_err(|error| Failure::Migrate(error).into_storage_error())?;
    run_with_retries(target, settings, &migrator).await
}

/// The failure of one attempt.
enum Failure {
    /// The attempt could not open the connection or set the timeout.
    Connect(sqlx::Error),
    /// The migration run failed.
    Migrate(MigrateError),
}

impl Failure {
    /// Return the driver error inside the failure, when there is one.
    fn driver_error(&self) -> Option<&sqlx::Error> {
        match self {
            Self::Connect(error)
            | Self::Migrate(
                MigrateError::Execute(error) | MigrateError::ExecuteMigration(error, _),
            ) => Some(error),
            Self::Migrate(_) => None,
        }
    }

    fn is_lock_timeout(&self) -> bool {
        self.driver_error().is_some_and(is_lock_timeout)
    }

    fn into_storage_error(self) -> StorageError {
        match self {
            Self::Connect(error)
            | Self::Migrate(
                MigrateError::Execute(error) | MigrateError::ExecuteMigration(error, _),
            ) => map_error(error),
            Self::Migrate(other) => StorageError::Other {
                sqlstate: None,
                source: Box::new(other),
            },
        }
    }
}

async fn run_with_retries(
    target: &Connection,
    settings: &MigrateSettings,
    migrator: &Migrator,
) -> Result<(), StorageError> {
    let mut retries_left = settings.retries;
    loop {
        let failure = match attempt(target, settings, migrator).await {
            Ok(()) => return Ok(()),
            Err(failure) => failure,
        };
        match retries_left.checked_sub(1) {
            Some(left) if failure.is_lock_timeout() => {
                retries_left = left;
                tokio::time::sleep(settings.retry_pause).await;
            }
            _ => return Err(failure.into_storage_error()),
        }
    }
}

/// Run the migrations once on a new connection. The connection closes in every case.
///
/// `sqlx` 0.8.6 does not release its advisory lock after a failed migration. A new connection
/// for each attempt therefore frees the lock for the next attempt.
async fn attempt(
    target: &Connection,
    settings: &MigrateSettings,
    migrator: &Migrator,
) -> Result<(), Failure> {
    let mut connection = open(target, settings).await.map_err(Failure::Connect)?;
    let result = migrator
        .run(&mut connection)
        .await
        .map_err(Failure::Migrate);
    // The result matters. A failed close of a finished connection changes nothing.
    let _ = connection.close().await;
    result
}

/// Open the dedicated connection and set `lock_timeout` for the whole session.
async fn open(
    target: &Connection,
    settings: &MigrateSettings,
) -> Result<PgConnection, sqlx::Error> {
    let mut connection = connect_options(target).connect().await?;
    let timeout = format!("{}ms", settings.lock_timeout.as_millis());
    sqlx::query("SELECT set_config('lock_timeout', $1, false)")
        .bind(timeout)
        .execute(&mut connection)
        .await?;
    Ok(connection)
}
