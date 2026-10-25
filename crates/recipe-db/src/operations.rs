//! `OperationsReads` on PostgreSQL (task 4.5, ADR-0073, ADR-0069, `data-model.md` sections 3.8,
//! 3.17, and 5.5).
//!
//! The role `recipe_app` runs every statement of this file with the grants of `grants.expected`:
//! `SELECT` on `security_event` and `app_setting`, and `EXECUTE` on `replication_state()`. The
//! role check reads the system catalog, which every role may read.

mod readiness;

use async_trait::async_trait;
use recipe_app::auth::{Operation, Requirement};
use recipe_app::error::StorageError;
use recipe_app::ports::{
    OperationsReads, OperationsSnapshot, ReplicationState, RoleCheck, SnapshotWindows, TimeStateKey,
};
use recipe_domain::identity::Permission;
use recipe_domain::timestamp::Timestamp;
use sqlx::PgPool;
use time::OffsetDateTime;

pub(crate) use readiness::PgReadinessProbe;

use crate::error::{corrupt, map_error, missing_row};
use crate::interval::micros;

/// The table name in a defect of a state key or of the first setting.
const SETTING_TABLE: &str = "app_setting";

/// The table name in a defect of a count.
const EVENT_TABLE: &str = "security_event";

/// The start of the transaction of the snapshot.
const BEGIN: &str = "BEGIN ISOLATION LEVEL READ COMMITTED READ ONLY";

/// Read every value of the snapshot with one statement.
///
/// `READ COMMITTED` takes one snapshot of the data for each statement, so one statement shows one
/// moment in all its fields. The windows are whole microseconds (`$1`, `$2`). The sums use `count`,
/// because the rows of `login_failed` and `csrf_rejected` aggregate per minute. `$3` holds the
/// names of the operations that need `UserManage`. `$4` is the key of the state key. The index
/// `security_event_occurred_at_idx` serves the two window sums.
const SNAPSHOT: &str = "\
SELECT now(), r.connected, r.lag_bytes::bigint, \
    (SELECT COALESCE(sum(e.count), 0)::bigint FROM public.security_event e \
        WHERE e.kind = 'login_failed' \
          AND e.occurred_at > now() - $1 * interval '1 microsecond'), \
    (SELECT COALESCE(sum(e.count), 0)::bigint FROM public.security_event e \
        WHERE e.occurred_at > now() - $2 * interval '1 microsecond' \
          AND (e.kind = 'csrf_rejected' \
               OR (e.kind = 'forbidden' AND e.detail->>'operation' = ANY($3::text[])))), \
    (SELECT max(e.occurred_at) FROM public.security_event e WHERE e.kind = 'audit_chain_broken'), \
    (SELECT s.value FROM public.app_setting s WHERE s.key = $4), \
    (SELECT min(s.created_at) FROM public.app_setting s) \
FROM public.replication_state() AS r";

/// Read the time of the database.
const DATABASE_NOW: &str = "SELECT now()";

/// Read the value of one state key.
const STATE: &str = "SELECT value FROM public.app_setting WHERE key = $1";

/// Read the facts of the role check. A table is a relation of kind `r` or `p`.
const ROLE_CHECK: &str = "\
SELECT current_user::text, r.rolsuper, \
    EXISTS (SELECT 1 FROM pg_catalog.pg_class c \
            WHERE c.relowner = r.oid AND c.relkind IN ('r', 'p')) \
FROM pg_catalog.pg_roles r WHERE r.rolname = current_user";

/// The columns of [`SNAPSHOT`] in their order.
type SnapshotRow = (
    OffsetDateTime,
    bool,
    Option<i64>,
    i64,
    i64,
    Option<OffsetDateTime>,
    Option<String>,
    Option<OffsetDateTime>,
);

/// The operations reads of `recipe-db`.
///
/// The type solves one problem: the job `alerts` must not mix two moments. The snapshot reads in
/// one `READ ONLY` transaction with one statement. The other methods are single statements, so
/// they need no transaction.
pub(crate) struct PgOperationsReads {
    pool: PgPool,
}

impl PgOperationsReads {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run [`SNAPSHOT`] in its own `READ ONLY` transaction.
    async fn read_snapshot_row(
        &self,
        windows: &SnapshotWindows,
    ) -> Result<SnapshotRow, StorageError> {
        let failed_logins = micros(EVENT_TABLE, windows.failed_logins)?;
        let security = micros(EVENT_TABLE, windows.security)?;
        let mut tx = self.pool.begin_with(BEGIN).await.map_err(map_error)?;
        let row = sqlx::query_as::<_, SnapshotRow>(SNAPSHOT)
            .bind(failed_logins)
            .bind(security)
            .bind(user_management_operations())
            .bind(TimeStateKey::AuditLastCheckAt.key())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_error)?;
        tx.commit().await.map_err(map_error)?;
        Ok(row)
    }
}

/// Return the names of the operations that need the permission `UserManage`.
///
/// The names come from the catalog of `recipe-app`, as in the memory adapter. A denial of such an
/// operation writes a `forbidden` event with the name in `detail` (ADR-0073 (d)).
fn user_management_operations() -> Vec<&'static str> {
    Operation::ALL
        .into_iter()
        .filter(|operation| {
            operation.requirement() == Requirement::Permission(Permission::UserManage)
        })
        .map(Operation::name)
        .collect()
}

/// Convert a count or a lag of the database into the unsigned number of the port.
fn unsigned(value: i64) -> Result<u64, StorageError> {
    u64::try_from(value).map_err(|error| corrupt(EVENT_TABLE, error))
}

/// Parse the stored text of a time state key (ADR-0073 (i)).
///
/// The error names the key and never the stored text.
fn parse_state(key: TimeStateKey, text: &str) -> Result<Timestamp, StorageError> {
    TimeStateKey::parse_text(text).ok_or_else(|| {
        corrupt(
            SETTING_TABLE,
            format_args!(
                "the value of the state key {} is not a time in the form of ADR-0073 (i)",
                key.key()
            ),
        )
    })
}

/// Build the snapshot of the port from the row.
fn into_snapshot(row: SnapshotRow) -> Result<OperationsSnapshot, StorageError> {
    let (now, connected, lag, failed_logins, security_alerts, broken_at, check_text, first) = row;
    let lag_bytes = lag.map(unsigned).transpose()?;
    let audit_last_check_at = check_text
        .as_deref()
        .map(|text| parse_state(TimeStateKey::AuditLastCheckAt, text))
        .transpose()?;
    Ok(OperationsSnapshot {
        database_now: Timestamp::from(now),
        replication: ReplicationState {
            connected,
            lag_bytes,
        },
        failed_logins: unsigned(failed_logins)?,
        security_alerts: unsigned(security_alerts)?,
        audit_chain_broken_at: broken_at.map(Timestamp::from),
        audit_last_check_at,
        first_setting_at: first.map(Timestamp::from),
    })
}

#[async_trait]
impl OperationsReads for PgOperationsReads {
    async fn snapshot(
        &self,
        windows: &SnapshotWindows,
    ) -> Result<OperationsSnapshot, StorageError> {
        into_snapshot(self.read_snapshot_row(windows).await?)
    }

    async fn database_now(&self) -> Result<Timestamp, StorageError> {
        let now: OffsetDateTime = sqlx::query_scalar(DATABASE_NOW)
            .fetch_one(&self.pool)
            .await
            .map_err(map_error)?;
        Ok(Timestamp::from(now))
    }

    async fn role_check(&self) -> Result<RoleCheck, StorageError> {
        let row: Option<(String, bool, bool)> = sqlx::query_as(ROLE_CHECK)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_error)?;
        let (current_user, is_superuser, owns_relations) =
            row.ok_or_else(|| missing_row("pg_roles"))?;
        Ok(RoleCheck {
            current_user,
            is_superuser,
            owns_relations,
        })
    }

    async fn state(&self, key: TimeStateKey) -> Result<Option<Timestamp>, StorageError> {
        let text: Option<String> = sqlx::query_scalar(STATE)
            .bind(key.key())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_error)?;
        text.as_deref()
            .map(|text| parse_state(key, text))
            .transpose()
    }
}

#[cfg(test)]
mod tests;
