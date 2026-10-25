//! `MaintenanceStore` on PostgreSQL (task 4.3, ADR-0060, `data-model.md` sections 3.6, 3.7, 3.17,
//! 4.12, and 6).
//!
//! The role `recipe_app` runs every statement of this file with the grants of `grants.expected`:
//! `DELETE` and `SELECT` on `session` and `login_attempt`, `SELECT`, `INSERT`, and
//! `UPDATE (value, updated_by, updated_at)` on `app_setting`, and `EXECUTE` on `security_event_purge()`.

use std::time::Duration;

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{MaintenanceStore, StateValue};
use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::{PgPool, Postgres};

use crate::error::{corrupt, map_error};
use crate::interval::micros;

/// The table name in a defect of a state key.
const SETTING_TABLE: &str = "app_setting";

/// The table name in a defect of the purge of security events.
const EVENT_TABLE: &str = "security_event";

/// The start of every transaction of this file. Each method has one statement and no retry.
const BEGIN: &str = "BEGIN ISOLATION LEVEL READ COMMITTED";

/// Delete the sessions that are idle-expired or older than the absolute timeout.
///
/// The number `$1` is whole microseconds. The product with one microsecond is exact, as in the
/// other statements of this crate.
const DELETE_SESSIONS: &str = "\
DELETE FROM session \
WHERE expires_at < now() OR created_at < now() - $1 * interval '1 microsecond'";

/// Delete one batch of old attempts, the oldest first. The index `login_attempt_attempted_at_idx`
/// serves the inner query. The job calls the method again while the batch is full.
const PURGE_ATTEMPTS: &str = "\
DELETE FROM login_attempt WHERE id IN ( \
    SELECT id FROM login_attempt \
    WHERE attempted_at < now() - $1 * interval '1 microsecond' \
    ORDER BY attempted_at \
    LIMIT $2)";

/// Delete one batch of old security events. The retention and the batch size are constants of
/// the function (`data-model.md` section 4.12).
const PURGE_EVENTS: &str = "SELECT security_event_purge()";

/// Insert the state key, or set the value of the row of the same key.
///
/// The row keeps `updated_by = NULL` on every write (`data-model.md` section 3.17). The insert path
/// leaves the column at its default. The update path sets `value` and `updated_by`. The trigger
/// `app_setting_set_updated_at` sets `updated_at` on the update path.
const RECORD_STATE: &str = "\
INSERT INTO app_setting (key, value) VALUES ($1, $2) \
ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_by = NULL";

/// The maintenance store of `recipe-db`.
///
/// The type solves one problem: a long delete runs into `db.statement_timeout`, and a failed batch
/// must keep the batches before it. Each method therefore runs one statement in its own short
/// transaction, and the job repeats the call.
pub(crate) struct PgMaintenanceStore {
    pool: PgPool,
}

impl PgMaintenanceStore {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run one statement in its own `READ COMMITTED` transaction and return the changed rows.
    async fn run(&self, statement: Query<'_, Postgres, PgArguments>) -> Result<u64, StorageError> {
        let mut tx = self.pool.begin_with(BEGIN).await.map_err(map_error)?;
        let done = statement.execute(&mut *tx).await.map_err(map_error)?;
        tx.commit().await.map_err(map_error)?;
        Ok(done.rows_affected())
    }
}

/// Return the text that the row of a state key holds (ADR-0073 (i)).
///
/// The text comes from `StateValue::to_text`. A value without a text gives the same
/// `StorageError` as before.
fn state_text(value: &StateValue) -> Result<String, StorageError> {
    value.to_text().ok_or_else(|| {
        corrupt(
            SETTING_TABLE,
            match value {
                StateValue::AuditLastCheckSeq(_) => "the chain_seq is negative",
                StateValue::AuditLastCheckAt(_) | StateValue::HeartbeatLastAt(_) => {
                    "the instant has no four digit year in UTC"
                }
            },
        )
    })
}

#[async_trait]
impl MaintenanceStore for PgMaintenanceStore {
    async fn delete_expired_sessions(
        &self,
        absolute_timeout: Duration,
    ) -> Result<u64, StorageError> {
        let timeout = micros("session", absolute_timeout)?;
        self.run(sqlx::query(DELETE_SESSIONS).bind(timeout)).await
    }

    async fn purge_login_attempts(
        &self,
        retention: Duration,
        batch: u32,
    ) -> Result<u64, StorageError> {
        let retention = micros("login_attempt", retention)?;
        self.run(
            sqlx::query(PURGE_ATTEMPTS)
                .bind(retention)
                .bind(i64::from(batch)),
        )
        .await
    }

    async fn purge_security_events(&self) -> Result<u64, StorageError> {
        let mut tx = self.pool.begin_with(BEGIN).await.map_err(map_error)?;
        let deleted: i64 = sqlx::query_scalar(PURGE_EVENTS)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_error)?;
        tx.commit().await.map_err(map_error)?;
        u64::try_from(deleted).map_err(|error| corrupt(EVENT_TABLE, error))
    }

    async fn record_state(&self, value: &StateValue) -> Result<(), StorageError> {
        let text = state_text(value)?;
        self.run(sqlx::query(RECORD_STATE).bind(value.key()).bind(text))
            .await
            .map(|_rows| ())
    }
}
