//! Probes of row locks from a second session (design D13).
//!
//! A probe tries one row lock with `NOWAIT` in its own transaction. The SQLSTATE `55P03` shows
//! that the adapter holds a conflicting lock. PostgreSQL grants a lock that does not conflict.

use std::time::Duration;

use sqlx::Connection as _;
use sqlx::PgConnection;
use uuid::Uuid;

/// SQLSTATE `lock_not_available`. `NOWAIT` raises it when a conflicting row lock exists.
pub const LOCK_NOT_AVAILABLE: &str = "55P03";

/// The pause between two probes of `wait_until_released`.
const RELEASE_PAUSE: Duration = Duration::from_millis(20);
/// The number of probes before `wait_until_released` gives up. The bound is about 5 s.
const RELEASE_LIMIT: u32 = 250;

/// Wait until the row `id` of `table` accepts `FOR UPDATE NOWAIT` from the second session.
///
/// A dropped sqlx transaction rolls back in the background, so the lock may outlive `drop`.
/// The port has no rollback method, so the case polls with a bound and then fails.
pub async fn wait_until_released(session: &mut PgConnection, table: &'static str, id: Uuid) {
    let mut last = None;
    for _ in 0..RELEASE_LIMIT {
        last = try_row_lock(session, table, "UPDATE", id).await;
        if last.is_none() {
            return;
        }
        tokio::time::sleep(RELEASE_PAUSE).await;
    }
    assert_eq!(last, None, "{table} {id} stays locked");
}

/// Try the row lock `mode` on the row `id` of `table` in a transaction of a second session.
///
/// `table` is a constant of the test. Return the SQLSTATE of the failure, or `None` when
/// PostgreSQL granted the lock.
pub async fn try_row_lock(
    session: &mut PgConnection,
    table: &'static str,
    mode: &str,
    id: Uuid,
) -> Option<String> {
    let mut tx = session.begin().await.unwrap();
    let statement = format!("SELECT 1 FROM {table} WHERE id = $1 FOR {mode} NOWAIT");
    let result = sqlx::query(&statement)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await;
    tx.rollback().await.unwrap();
    result.err().and_then(|error| match error {
        sqlx::Error::Database(database_error) => database_error.code().map(|c| c.into_owned()),
        other => Some(other.to_string()),
    })
}
