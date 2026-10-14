//! `AttemptRepository` on the transaction (task 5.6, `data-model.md` sections 3.7 and 6).

use std::net::IpAddr;
use std::time::Duration;

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{
    AttemptId, AttemptRepository, AttemptRow, DeletedAttempts, LoginThrottleState,
};
use recipe_domain::identity::TypedLogin;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use crate::error::{corrupt, map_error};
use crate::interval::micros;
use crate::transaction::PgTransaction;

/// The table name of the defects.
const TABLE: &str = "login_attempt";

const MARK_SUCCEEDED: &str = "UPDATE login_attempt SET succeeded = true WHERE id = $1";

/// The delete of `UnlockLogin`. The data-modifying CTE runs once, so the count and the newest rows
/// come from the rows that the delete removed. The join keeps one row when `keep` is 0.
const DELETE_FOR_LOGIN: &str = "\
WITH gone AS ( \
    DELETE FROM login_attempt WHERE login = $1 \
    RETURNING id, client_ip, attempted_at, succeeded \
), newest AS ( \
    SELECT id, client_ip, attempted_at, succeeded FROM gone \
    ORDER BY attempted_at DESC, id DESC LIMIT $2 \
) \
SELECT t.total, n.id, host(n.client_ip) AS client_ip, n.attempted_at, n.succeeded \
FROM (SELECT count(*) AS total FROM gone) t LEFT JOIN newest n ON true \
ORDER BY n.attempted_at DESC NULLS LAST, n.id DESC";

const THROTTLE_STATE: &str = "\
SELECT count(*) AS failures, max(attempted_at) AS last_failure FROM login_attempt \
WHERE login = $1 AND NOT succeeded AND attempted_at > now() - $2 * interval '1 microsecond'";

/// One row of `DELETE_FOR_LOGIN`. The columns of the attempt are `NULL` when no row was kept.
#[derive(sqlx::FromRow)]
struct DeletedRecord {
    total: i64,
    id: Option<i64>,
    client_ip: Option<String>,
    attempted_at: Option<OffsetDateTime>,
    succeeded: Option<bool>,
}

#[derive(sqlx::FromRow)]
struct StateRecord {
    failures: i64,
    last_failure: Option<OffsetDateTime>,
}

/// Build the kept attempt of a record, or `None` for the row without an attempt.
fn kept_row(login: &TypedLogin, record: DeletedRecord) -> Result<Option<AttemptRow>, StorageError> {
    let DeletedRecord {
        id,
        client_ip,
        attempted_at,
        succeeded,
        ..
    } = record;
    let Some(id) = id else {
        return Ok(None);
    };
    let (Some(client_ip), Some(attempted_at), Some(succeeded)) =
        (client_ip, attempted_at, succeeded)
    else {
        return Err(corrupt(TABLE, "a deleted attempt has a missing column"));
    };
    let client_ip: IpAddr = client_ip.parse().map_err(|error| corrupt(TABLE, error))?;
    Ok(Some(AttemptRow {
        id: AttemptId::from(id),
        login: login.clone(),
        client_ip,
        attempted_at: Timestamp::from(attempted_at),
        succeeded,
    }))
}

#[async_trait]
impl AttemptRepository for PgTransaction {
    async fn mark_succeeded(&mut self, id: AttemptId) -> Result<(), StorageError> {
        // No row changed means that `UnlockLogin` deleted the attempt. That is a success.
        sqlx::query(MARK_SUCCEEDED)
            .bind(id.get())
            .execute(&mut *self.tx)
            .await
            .map_err(map_error)?;
        Ok(())
    }

    async fn delete_for_login(
        &mut self,
        login: &TypedLogin,
        keep: u32,
    ) -> Result<DeletedAttempts, StorageError> {
        let records = sqlx::query_as::<_, DeletedRecord>(DELETE_FOR_LOGIN)
            .bind(login.as_str())
            .bind(i64::from(keep))
            .fetch_all(&mut *self.tx)
            .await
            .map_err(map_error)?;
        let total = records.first().map_or(0, |record| record.total);
        let count = u64::try_from(total).map_err(|error| corrupt(TABLE, error))?;
        let mut newest = Vec::new();
        for record in records {
            newest.extend(kept_row(login, record)?);
        }
        Ok(DeletedAttempts { newest, count })
    }

    async fn throttle_state(
        &mut self,
        login: &TypedLogin,
        window: Duration,
    ) -> Result<LoginThrottleState, StorageError> {
        let record = sqlx::query_as::<_, StateRecord>(THROTTLE_STATE)
            .bind(login.as_str())
            .bind(micros(TABLE, window)?)
            .fetch_one(&mut *self.tx)
            .await
            .map_err(map_error)?;
        Ok(LoginThrottleState {
            failures: u32::try_from(record.failures).map_err(|error| corrupt(TABLE, error))?,
            last_failure: record.last_failure.map(Timestamp::from),
        })
    }
}
