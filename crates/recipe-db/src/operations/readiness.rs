//! `ReadinessProbe` on PostgreSQL (task 4.5, ADR-0059, `reliability.md` section 7.2).
//!
//! The role `recipe_app` runs both statements with the grants of `grants.expected`: `SELECT` on
//! `_sqlx_migrations`. The probe runs on the health pool, so a busy request pool cannot hide a
//! fault and a slow check cannot take a connection of the requests.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{NotReadyCause, Readiness, ReadinessProbe};
use sqlx::PgPool;

use crate::error::map_error;
use crate::migrator::embedded;

/// The liveness statement of the check.
const PING: &str = "SELECT 1";

/// Read the migrations that the database applied.
const APPLIED: &str = "SELECT version, checksum, success FROM public._sqlx_migrations";

/// One row of `_sqlx_migrations`: the version, the checksum, and the success flag.
type AppliedRow = (i64, Vec<u8>, bool);

/// The readiness probe of `recipe-db`.
///
/// The type solves one problem: a server must not take requests when the database does not
/// answer, or when the schema does not belong to this binary (ADR-0059 (h)). The check never
/// returns an error. Every fault is a [`Readiness::NotReady`] with its cause.
pub(crate) struct PgReadinessProbe {
    pool: PgPool,
}

impl PgReadinessProbe {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Ping the database and apply the schema rule. The statements use one connection and no
    /// explicit transaction, so a dropped check cannot return an open transaction to the pool.
    async fn run(&self) -> Readiness {
        let mut connection = match self.pool.acquire().await {
            Ok(connection) => connection,
            Err(_) => return Readiness::NotReady(NotReadyCause::Unreachable),
        };
        if let Err(error) = sqlx::query(PING).execute(&mut *connection).await {
            return Readiness::NotReady(cause_of(error, NotReadyCause::Unreachable));
        }
        match sqlx::query_as::<_, AppliedRow>(APPLIED)
            .fetch_all(&mut *connection)
            .await
        {
            Ok(applied) if schema_matches(embedded_checksums(), &applied) => Readiness::Ready,
            Ok(_) => Readiness::NotReady(NotReadyCause::SchemaMismatch),
            Err(error) => Readiness::NotReady(cause_of(error, NotReadyCause::SchemaMismatch)),
        }
    }
}

#[async_trait]
impl ReadinessProbe for PgReadinessProbe {
    async fn check(&self, timeout: Duration) -> Readiness {
        tokio::time::timeout(timeout, self.run())
            .await
            .unwrap_or(Readiness::NotReady(NotReadyCause::TimedOut))
    }
}

/// Return the cause of a failed statement.
///
/// A lost connection is `Unreachable`. A statement or lock timeout of the session is `TimedOut`.
/// Every other failure has the cause `otherwise` of the statement: a ping that fails means an
/// unreachable database, and a failed read of `_sqlx_migrations` (for example a missing table)
/// means a schema that does not match.
fn cause_of(error: sqlx::Error, otherwise: NotReadyCause) -> NotReadyCause {
    match map_error(error) {
        StorageError::Unavailable | StorageError::PoolTimeout => NotReadyCause::Unreachable,
        StorageError::StatementTimeout | StorageError::Deadlock { .. } => NotReadyCause::TimedOut,
        _ => otherwise,
    }
}

/// Return the version and the checksum of every forward migration that the binary embeds.
fn embedded_checksums() -> impl Iterator<Item = (i64, &'static [u8])> {
    embedded()
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| (migration.version, migration.checksum.as_ref()))
}

/// Apply the schema rule of ADR-0059 (h) and (i).
///
/// The schema matches when no row of `_sqlx_migrations` failed, and when each embedded migration
/// has a row with the same version and the same checksum. A row of a version that the binary does
/// not know is a newer database. It does not change the result, unless it failed.
fn schema_matches<'a>(
    embedded: impl IntoIterator<Item = (i64, &'a [u8])>,
    applied: &[AppliedRow],
) -> bool {
    if applied.iter().any(|(_, _, success)| !success) {
        return false;
    }
    let checksums: HashMap<i64, &[u8]> = applied
        .iter()
        .map(|(version, checksum, _)| (*version, checksum.as_slice()))
        .collect();
    embedded
        .into_iter()
        .all(|(version, checksum)| checksums.get(&version) == Some(&checksum))
}

#[cfg(test)]
mod tests;
