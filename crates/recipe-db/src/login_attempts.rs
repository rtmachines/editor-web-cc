//! `LoginAttemptStore` on PostgreSQL: T1 of `Login` (design D6, `data-model.md` sections 3.7, 6).

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{
    AttemptId, AttemptKey, AttemptOutcome, BasicChange, BasicConfirmation, LoginAttemptStore,
};
use recipe_domain::identity::{AttemptCount, PasswordHash, Role, ThrottlePolicy, UserId};
use recipe_domain::timestamp::Timestamp;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::address::canonical_text;
use crate::error::{corrupt, map_error};
use crate::interval::micros;

/// The table name of the defects.
const TABLE: &str = "login_attempt";

/// Statement 1: the class-1 advisory lock of the typed login, alone in its statement.
///
/// Under `READ COMMITTED`, the next statement then sees every attempt that a request with the same
/// login committed before it (`data-model.md` section 1.4, finding F8).
const LOCK_LOGIN: &str = "SELECT pg_advisory_xact_lock(1, hashtext($1))";

/// Statement 2: one scan of `login_attempt_ip_idx` for the failures of the address in the window.
///
/// The two `FILTER` clauses narrow the count to the pair (login, address). The statement also
/// reads `now()`, so the throttle decision uses the database time only.
const COUNT_FAILURES: &str = "\
SELECT count(*) FILTER (WHERE login = $1) AS pair_failures, \
    max(attempted_at) FILTER (WHERE login = $1) AS last_pair_failure, \
    count(*) AS address_failures, \
    min(attempted_at) AS oldest_address_failure, \
    now() AS now \
FROM login_attempt \
WHERE client_ip = $2::inet AND NOT succeeded \
    AND attempted_at > now() - $3 * interval '1 microsecond'";

/// Statement 3: the attempt row. The database sets `attempted_at`.
const INSERT_ATTEMPT: &str = "\
INSERT INTO login_attempt (login, client_ip, succeeded) VALUES ($1, $2::inet, false) RETURNING id";

/// T2 statement 1: lock the user row and compare the stored hash with the hash of T1.
///
/// The lock is `FOR NO KEY UPDATE`, as in every identity command (ADR-0015). A password reset or
/// a deactivation that holds the row makes the statement wait. The comparison runs in the
/// database, so the stored hash never leaves it.
const LOCK_USER: &str = "SELECT is_active, role, password_hash = $2 AS same_hash FROM app_user WHERE id = $1 FOR NO KEY UPDATE";

/// T2 statement 2: mark the attempt. No row changes when `UnlockLogin` deleted it (ADR-0035).
const MARK_SUCCEEDED: &str = "UPDATE login_attempt SET succeeded = true WHERE id = $1";

/// The columns of the T2 statement 1.
#[derive(sqlx::FromRow)]
struct UserState {
    is_active: bool,
    role: String,
    same_hash: bool,
}

impl UserState {
    /// Return the first change that the Basic check rejects, or `None` for an unchanged account.
    fn change(&self) -> Result<Option<BasicChange>, StorageError> {
        let role = Role::parse(&self.role).map_err(|error| corrupt("app_user", error))?;
        Ok(if !self.is_active {
            Some(BasicChange::Inactive)
        } else if role != Role::ExportClient {
            Some(BasicChange::NotExportClient)
        } else if !self.same_hash {
            Some(BasicChange::PasswordChanged)
        } else {
            None
        })
    }
}

/// The columns of statement 2.
#[derive(sqlx::FromRow)]
struct CountRecord {
    pair_failures: i64,
    last_pair_failure: Option<OffsetDateTime>,
    address_failures: i64,
    oldest_address_failure: Option<OffsetDateTime>,
    now: OffsetDateTime,
}

impl TryFrom<CountRecord> for AttemptCount {
    type Error = StorageError;

    fn try_from(record: CountRecord) -> Result<Self, StorageError> {
        Ok(Self {
            pair_failures: u32::try_from(record.pair_failures)
                .map_err(|error| corrupt(TABLE, error))?,
            last_pair_failure: record.last_pair_failure.map(Timestamp::from),
            address_failures: u32::try_from(record.address_failures)
                .map_err(|error| corrupt(TABLE, error))?,
            oldest_address_failure: record.oldest_address_failure.map(Timestamp::from),
            now: Timestamp::from(record.now),
        })
    }
}

/// The login attempt store of `recipe-db`.
///
/// The type solves one problem: T1 of `Login` writes no audit entry, so it cannot use
/// `UnitOfWork::commit`. It runs its three statements in its own transaction and commits before
/// the password check.
pub(crate) struct PgLoginAttemptStore {
    pool: PgPool,
}

impl PgLoginAttemptStore {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LoginAttemptStore for PgLoginAttemptStore {
    async fn begin_attempt(
        &self,
        key: &AttemptKey,
        policy: &ThrottlePolicy,
    ) -> Result<AttemptOutcome, StorageError> {
        let window = micros(TABLE, policy.window)?;
        let address = canonical_text(key.client_ip);
        let mut tx = self
            .pool
            .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED")
            .await
            .map_err(map_error)?;
        sqlx::query(LOCK_LOGIN)
            .bind(key.login.as_str())
            .execute(&mut *tx)
            .await
            .map_err(map_error)?;
        let record = sqlx::query_as::<_, CountRecord>(COUNT_FAILURES)
            .bind(key.login.as_str())
            .bind(&address)
            .bind(window)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_error)?;
        let count = AttemptCount::try_from(record)?;
        let attempt = if policy.budget_reached(count.address_failures) {
            None
        } else {
            let id: i64 = sqlx::query_scalar(INSERT_ATTEMPT)
                .bind(key.login.as_str())
                .bind(&address)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_error)?;
            Some(AttemptId::from(id))
        };
        tx.commit().await.map_err(map_error)?;
        Ok(AttemptOutcome { count, attempt })
    }

    async fn confirm_basic(
        &self,
        attempt: AttemptId,
        user: UserId,
        expected_hash: &PasswordHash,
    ) -> Result<BasicConfirmation, StorageError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED")
            .await
            .map_err(map_error)?;
        let state = sqlx::query_as::<_, UserState>(LOCK_USER)
            .bind(*user.as_uuid())
            .bind(expected_hash.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_error)?;
        let reason = match state {
            None => Some(BasicChange::Missing),
            Some(state) => state.change()?,
        };
        if let Some(reason) = reason {
            tx.rollback().await.map_err(map_error)?;
            return Ok(BasicConfirmation::Changed { reason });
        }
        sqlx::query(MARK_SUCCEEDED)
            .bind(attempt.get())
            .execute(&mut *tx)
            .await
            .map_err(map_error)?;
        tx.commit().await.map_err(map_error)?;
        Ok(BasicConfirmation::Confirmed)
    }
}
