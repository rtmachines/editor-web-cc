//! The fixtures of the identity tests on the database (tasks 5.7 and 5.8, design D19).
//!
//! The functions seed users and sessions as `recipe_migrator`, build an `IdentityContext` over
//! the adapters of `Database`, and read the state that a failed command must leave unchanged.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use recipe_app::identity::{
    Authenticated, HashFailure, HashGate, IdentityContext, IdentitySettings, PasswordHasher,
    authenticate,
};
use recipe_app::ports::{LoginAttemptStore, SessionStore, UnitOfWork};
use recipe_domain::identity::{
    CsrfToken, KeyRing, NewPassword, Password, PasswordHash, Role, SessionId, SessionToken, UserId,
};
use uuid::Uuid;

use super::app::App;
use super::gate::gate_statement;

/// The password of every seeded user.
pub const PASSWORD: &str = "Seed-password-2024";
/// The start of the text of a hash that [`FastHasher`] makes.
const FAKE_PREFIX: &str = "$argon2id$fake$";
/// A key file with one valid key of 64 lower-case hex characters.
const LOGIN_KEY: &str = "0101010101010101010101010101010101010101010101010101010101010101";
/// The permits of the gate of a normal test.
const PERMITS: usize = 4;
/// The wait for a permit in a test. A flood test needs more than the default of the application.
pub const PERMIT_WAIT: Duration = Duration::from_secs(20);
/// The idle time of a seeded session in minutes.
const SESSION_MINUTES: i64 = 30;

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// A hasher without cost.
///
/// The hash of a password is the prefix `$argon2id$fake$` and the password. A check passes only
/// when the stored hash is the hash of the typed password.
#[derive(Debug, Default)]
pub struct FastHasher;

impl PasswordHasher for FastHasher {
    fn hash(&self, password: &NewPassword) -> Result<PasswordHash, HashFailure> {
        PasswordHash::parse(&format!("{FAKE_PREFIX}{}", password.as_str()))
            .map_err(|_malformed| HashFailure::Compute)
    }

    fn verify(&self, password: &Password, hash: Option<&PasswordHash>) -> bool {
        hash.is_some_and(|stored| stored.as_str() == format!("{FAKE_PREFIX}{}", password.as_str()))
    }
}

/// The rows that a failed identity command must leave as they were.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityState {
    /// Every `app_user` row as text, ordered by login.
    pub users: String,
    /// The id and the epoch of every session, ordered by id.
    pub sessions: String,
    /// Every `login_attempt` row as text, ordered by key.
    pub attempts: String,
}

impl App {
    /// Build an identity context with a hasher, a number of permits, and a wait for a permit.
    pub fn identity_with(
        &self,
        hasher: Arc<dyn PasswordHasher>,
        permits: usize,
        permit_wait: Duration,
    ) -> IdentityContext {
        IdentityContext::new(
            self.context(),
            self.attempts(),
            self.sessions(),
            HashGate::new(permits, permit_wait, hasher),
            KeyRing::parse(LOGIN_KEY).unwrap(),
            IdentitySettings::default(),
        )
    }

    /// Build an identity context with a [`FastHasher`] and the normal 4 permits.
    pub fn identity(&self) -> IdentityContext {
        self.identity_with(Arc::new(FastHasher), PERMITS, PERMIT_WAIT)
    }

    /// The pool-level port of T1 of `Login`.
    pub fn attempts(&self) -> Arc<dyn LoginAttemptStore> {
        self.database.login_attempts()
    }

    /// The pool-level port that authenticates a request.
    pub fn sessions(&self) -> Arc<dyn SessionStore> {
        self.database.session_store()
    }

    /// The unit of work of the adapters.
    pub fn unit_of_work(&self) -> Arc<dyn UnitOfWork> {
        self.database.unit_of_work()
    }

    /// Insert an active user whose hash [`FastHasher`] makes of `password`.
    pub async fn seed_user(&mut self, login: &str, role: Role, password: &str) -> UserId {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO app_user (login, password_hash, role) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(login)
        .bind(format!("{FAKE_PREFIX}{password}"))
        .bind(role.as_sql())
        .fetch_one(self.migrator())
        .await
        .unwrap();
        UserId::from(id)
    }

    /// Insert a session of a user with the current epoch of that user. Return its token.
    pub async fn seed_session(&mut self, user: UserId) -> SessionToken {
        let number = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        let mut bytes = [0_u8; 16];
        bytes[8..].copy_from_slice(&number.to_be_bytes());
        let token = SessionToken::from_bytes(bytes);
        let csrf = CsrfToken::from_bytes([7_u8; 32]);
        sqlx::query(
            "INSERT INTO session (id, user_id, epoch, csrf_token, expires_at) \
             SELECT $1, id, session_epoch, $3, now() + $4 * interval '1 minute' \
             FROM app_user WHERE id = $2",
        )
        .bind(SessionId::of(&token).as_str())
        .bind(*user.as_uuid())
        .bind(csrf.to_hex())
        .bind(SESSION_MINUTES)
        .execute(self.migrator())
        .await
        .unwrap();
        token
    }

    /// Authenticate a request with a token, as `recipe-web` does before a use case.
    pub async fn sign_in(&self, ctx: &IdentityContext, token: &SessionToken) -> Authenticated {
        authenticate(ctx, token).await.unwrap().unwrap()
    }

    /// Give the administrator of the fixture a session and authenticate it.
    pub async fn admin_session(&mut self, ctx: &IdentityContext) -> Authenticated {
        let token = self.seed_session(self.user.user()).await;
        self.sign_in(ctx, &token).await
    }

    /// Mark a user as inactive, as `DeactivateUser` leaves the row.
    pub async fn make_inactive(&mut self, user: UserId) {
        sqlx::query("UPDATE app_user SET is_active = false, deactivated_at = now() WHERE id = $1")
            .bind(*user.as_uuid())
            .execute(self.migrator())
            .await
            .unwrap();
    }

    /// Insert failed attempts of a login from an address, as the superuser.
    pub async fn seed_attempts(&mut self, login: &str, address: &str, count: i32) {
        sqlx::query(
            "INSERT INTO login_attempt (login, client_ip, succeeded) \
             SELECT $1, $2::inet, false FROM generate_series(1, $3)",
        )
        .bind(login)
        .bind(address)
        .bind(count)
        .execute(self.admin())
        .await
        .unwrap();
    }

    /// Read the rows that a failed identity command must leave as they were.
    pub async fn identity_state(&mut self) -> IdentityState {
        IdentityState {
            users: self.text(USERS).await,
            sessions: self.text(SESSIONS).await,
            attempts: self.text(ATTEMPTS).await,
        }
    }

    /// Run a query that returns one text and return it.
    pub async fn text(&mut self, query: &str) -> String {
        sqlx::query_scalar(query)
            .fetch_one(self.admin())
            .await
            .unwrap()
    }

    /// Count the rows of a query without a bind.
    pub async fn rows(&mut self, query: &str) -> i64 {
        sqlx::query_scalar(query)
            .fetch_one(self.admin())
            .await
            .unwrap()
    }

    /// The stored hash of a user.
    pub async fn password_hash_of(&mut self, login: &str) -> String {
        sqlx::query_scalar("SELECT password_hash FROM app_user WHERE login = $1")
            .bind(login)
            .fetch_one(self.admin())
            .await
            .unwrap()
    }

    /// Create a sequence that counts how often a test trigger ran.
    ///
    /// A sequence ignores the rollback of a transaction, so a failed run still counts.
    pub async fn install_run_counter(&mut self) {
        sqlx::raw_sql(
            "CREATE SEQUENCE public.test_runs; \
             GRANT USAGE ON SEQUENCE public.test_runs TO recipe_app;",
        )
        .execute(self.admin())
        .await
        .unwrap();
    }

    /// Create a row trigger that counts its runs and waits at the gate when `condition` holds.
    pub async fn install_counted_gate(
        &mut self,
        table: &str,
        timing: &str,
        events: &str,
        condition: &str,
    ) {
        let body = format!(
            "PERFORM pg_catalog.nextval('public.test_runs'); {}",
            gate_statement(condition)
        );
        self.install_probe(table, timing, events, &body).await;
    }

    /// The number of runs of the counted trigger.
    pub async fn counted_runs(&mut self) -> i64 {
        sqlx::query_scalar(
            "SELECT CASE WHEN is_called THEN last_value ELSE 0 END FROM public.test_runs",
        )
        .fetch_one(self.admin())
        .await
        .unwrap()
    }
}

/// Every `app_user` row as text.
const USERS: &str =
    "SELECT COALESCE(string_agg(a::text, ';' ORDER BY a.login), '') FROM app_user a";
/// The id and the epoch of every session.
const SESSIONS: &str =
    "SELECT COALESCE(string_agg(s.id || '/' || s.epoch, ',' ORDER BY s.id), '') FROM session s";
/// Every `login_attempt` row as text.
const ATTEMPTS: &str =
    "SELECT COALESCE(string_agg(l::text, ';' ORDER BY l.id), '') FROM login_attempt l";
