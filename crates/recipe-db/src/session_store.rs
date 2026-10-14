//! `SessionStore` on PostgreSQL: the one statement that authenticates a request (design D10).

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{SessionLimits, SessionRow, SessionStore};
use recipe_domain::identity::{CsrfToken, Login, Role, SessionId, UserId};
use recipe_domain::timestamp::Timestamp;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{corrupt, map_error};
use crate::interval::micros;

/// The table name of the defects.
const TABLE: &str = "session";

/// Check the session and move its two times in one statement.
///
/// A missing row, an idle session, an old session, an inactive user, and another epoch all match
/// no row. A concurrent `DELETE` of the session therefore gives no row and no error.
const AUTHENTICATE: &str = "\
UPDATE session s \
SET last_seen_at = now(), expires_at = now() + $2 * interval '1 microsecond' \
FROM app_user u \
WHERE s.id = $1 AND u.id = s.user_id AND s.expires_at > now() \
    AND s.created_at > now() - $3 * interval '1 microsecond' \
    AND u.is_active AND s.epoch = u.session_epoch \
RETURNING s.id, s.user_id, u.login, u.role, u.must_change_password, s.epoch, s.csrf_token, \
    s.created_at, s.last_seen_at, s.expires_at";

/// The columns of the statement. The type has no `Debug`, because it holds two secrets.
#[derive(sqlx::FromRow)]
struct SessionRecord {
    id: String,
    user_id: Uuid,
    login: String,
    role: String,
    must_change_password: bool,
    epoch: i64,
    csrf_token: String,
    created_at: OffsetDateTime,
    last_seen_at: OffsetDateTime,
    expires_at: OffsetDateTime,
}

impl TryFrom<SessionRecord> for SessionRow {
    type Error = StorageError;

    fn try_from(record: SessionRecord) -> Result<Self, StorageError> {
        Ok(Self {
            id: SessionId::parse(&record.id).map_err(|error| corrupt(TABLE, error))?,
            user_id: UserId::from(record.user_id),
            login: Login::parse(&record.login).map_err(|error| corrupt(TABLE, error))?,
            role: Role::parse(&record.role).map_err(|error| corrupt(TABLE, error))?,
            must_change_password: record.must_change_password,
            epoch: record.epoch,
            csrf_token: CsrfToken::parse(&record.csrf_token)
                .map_err(|error| corrupt(TABLE, error))?,
            created_at: Timestamp::from(record.created_at),
            last_seen_at: Timestamp::from(record.last_seen_at),
            expires_at: Timestamp::from(record.expires_at),
        })
    }
}

/// The session store of `recipe-db`.
///
/// The type solves one problem: every request touches its session, and the touch must not wait
/// for a command or hold a connection longer than one statement. The single `UPDATE` runs in its
/// own implicit transaction on the pool, also in read-only mode (ADR-0035 k).
pub(crate) struct PgSessionStore {
    pool: PgPool,
}

impl PgSessionStore {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionStore for PgSessionStore {
    async fn authenticate(
        &self,
        id: &SessionId,
        limits: &SessionLimits,
    ) -> Result<Option<SessionRow>, StorageError> {
        let record = sqlx::query_as::<_, SessionRecord>(AUTHENTICATE)
            .bind(id.as_str())
            .bind(micros(TABLE, limits.idle_timeout)?)
            .bind(micros(TABLE, limits.absolute_timeout)?)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_error)?;
        record.map(SessionRow::try_from).transpose()
    }
}
