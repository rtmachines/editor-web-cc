//! `SessionRepository` on the transaction (task 5.5, `data-model.md` sections 3.6 and 6).

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{NewSession, SessionRepository};
use recipe_domain::identity::{SessionId, UserId};

use crate::error::map_error;
use crate::interval::micros;
use crate::transaction::PgTransaction;

/// The table name of the defects.
const TABLE: &str = "session";

/// The three times come from the time of the transaction. The foreign key and the primary key
/// check the row, and the error map names them.
const INSERT: &str = "\
INSERT INTO session (id, user_id, epoch, csrf_token, created_at, last_seen_at, expires_at) \
VALUES ($1, $2, $3, $4, now(), now(), now() + $5 * interval '1 microsecond')";

const DELETE: &str = "DELETE FROM session WHERE id = $1";

const DELETE_ALL_OF_USER: &str = "DELETE FROM session WHERE user_id = $1";

#[async_trait]
impl SessionRepository for PgTransaction {
    async fn insert(&mut self, session: &NewSession) -> Result<(), StorageError> {
        sqlx::query(INSERT)
            .bind(session.id.as_str())
            .bind(*session.user_id.as_uuid())
            .bind(session.epoch)
            .bind(session.csrf_token.to_hex())
            .bind(micros(TABLE, session.idle_timeout)?)
            .execute(&mut *self.tx)
            .await
            .map_err(map_error)?;
        Ok(())
    }

    async fn delete(&mut self, id: &SessionId) -> Result<bool, StorageError> {
        let result = sqlx::query(DELETE)
            .bind(id.as_str())
            .execute(&mut *self.tx)
            .await
            .map_err(map_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_all_of_user(&mut self, user: UserId) -> Result<u64, StorageError> {
        let result = sqlx::query(DELETE_ALL_OF_USER)
            .bind(*user.as_uuid())
            .execute(&mut *self.tx)
            .await
            .map_err(map_error)?;
        Ok(result.rows_affected())
    }
}
