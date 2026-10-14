//! The security event sink on PostgreSQL (design D8, `data-model.md` section 3.8).

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::SecurityEventSink;
use recipe_domain::audit::{SecurityEvent, SecurityEventKind};
use sqlx::PgPool;

use crate::address::canonical_text;
use crate::error::map_error;

/// The columns of the insert. Both statements share them.
///
/// The list names neither `occurred_at` nor `count`. `recipe_app` holds no `INSERT` privilege on
/// them, and the database sets the time and the first count of each row (ADR-0035).
const INSERT_EVENT: &str = "\
INSERT INTO security_event \
    (kind, login_hmac, user_id, client_ip, user_agent, request_id, detail) \
VALUES ($1, $2, $3, $4::inet, $5, $6, $7)";

/// The conflict clause for the kinds that aggregate per address, kind, and minute.
///
/// The target equals the index `security_event_aggregate_unique`. The row keeps the columns of the
/// first event of the minute, and a later event adds 1 to `count`.
const AGGREGATE_ON_CONFLICT: &str = " \
ON CONFLICT (client_ip, kind, (date_trunc('minute', occurred_at AT TIME ZONE 'UTC'))) \
    WHERE kind IN ('login_failed', 'login_throttled', 'csrf_rejected') \
DO UPDATE SET count = security_event.count + 1";

/// The sink of security events of `recipe-db`.
///
/// The type solves one problem: a security event must survive the rollback of the request that
/// caused it, and a flood from one address must not fill the table. `record` therefore runs its
/// own short transaction on the pool and never joins the transaction of a command.
pub(crate) struct PgSecurityEventSink {
    pool: PgPool,
}

impl PgSecurityEventSink {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Build the statement of an event kind. Only the three aggregated kinds get the conflict clause.
fn statement_for(kind: SecurityEventKind) -> String {
    let is_aggregated = matches!(
        kind,
        SecurityEventKind::LoginFailed
            | SecurityEventKind::LoginThrottled
            | SecurityEventKind::CsrfRejected
    );
    if is_aggregated {
        format!("{INSERT_EVENT}{AGGREGATE_ON_CONFLICT}")
    } else {
        INSERT_EVENT.to_owned()
    }
}

#[async_trait]
impl SecurityEventSink for PgSecurityEventSink {
    async fn record(&self, event: SecurityEvent) -> Result<(), StorageError> {
        let kind = event.kind();
        let mut tx = self.pool.begin().await.map_err(map_error)?;
        sqlx::query(&statement_for(kind))
            .bind(kind.as_sql())
            .bind(event.login_hmac().map(|hmac| hmac.as_bytes().as_slice()))
            .bind(event.user().map(|user| *user.as_uuid()))
            .bind(event.client_ip().map(canonical_text))
            .bind(event.user_agent().map(|agent| agent.as_str()))
            .bind(*event.request_id().as_uuid())
            .bind(event.detail())
            .execute(&mut *tx)
            .await
            .map_err(map_error)?;
        tx.commit().await.map_err(map_error)
    }
}

#[cfg(test)]
mod tests {
    use recipe_domain::audit::SecurityEventKind;

    use super::statement_for;

    #[test]
    fn exactly_the_three_aggregated_kinds_get_the_conflict_clause() {
        for kind in SecurityEventKind::ALL {
            let aggregated = matches!(
                kind,
                SecurityEventKind::LoginFailed
                    | SecurityEventKind::LoginThrottled
                    | SecurityEventKind::CsrfRejected
            );
            let statement = statement_for(kind);
            assert_eq!(
                statement.contains("ON CONFLICT"),
                aggregated,
                "{}",
                kind.as_sql()
            );
        }
    }
}
