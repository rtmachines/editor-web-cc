//! The harness of `Database`.

use std::sync::Arc;

use async_trait::async_trait;
use std::net::IpAddr;

use recipe_app::ports::{
    AttemptId, LoginAttemptStore, SecurityEventSink, SessionStore, UnitOfWork,
};
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::identity::{TypedLogin, UserId};

use super::harness::{Harness, seed_actor};
use crate::support::{Role, TestDatabase};

/// The `sqlx` adapter on its own test database.
///
/// The adapter connects as `recipe_app`. One `app_user` row exists, inserted through
/// `UserRepository::insert`, as in the memory harness (design D19). The contract suite seeds
/// only through the ports.
pub struct PgHarness {
    /// The test database. A case with postgres-only checks opens its own sessions on it.
    pub database: TestDatabase,
    unit_of_work: Arc<dyn UnitOfWork>,
    session_store: Arc<dyn SessionStore>,
    login_attempts: Arc<dyn LoginAttemptStore>,
    security_events: Arc<dyn SecurityEventSink>,
    user: UserId,
}

/// Insert attempts with a fixed age as `recipe_migrator`. `WITH ORDINALITY` keeps the key order.
const SEED_ATTEMPTS: &str = "\
INSERT INTO login_attempt (login, client_ip, attempted_at, succeeded) \
SELECT $1, $2::inet, now() - a.age * interval '1 second', $4 \
FROM unnest($3::bigint[]) WITH ORDINALITY AS a(age, ordinality) \
ORDER BY a.ordinality RETURNING id";

impl PgHarness {
    /// Create the test database, the pool, and the user.
    pub async fn create() -> Self {
        let database = TestDatabase::create().await;
        let connected = Database::connect(&database.target(Role::App), &PoolSettings::default())
            .await
            .unwrap();
        let unit_of_work = connected.unit_of_work();
        let user = seed_actor(unit_of_work.as_ref()).await;
        Self {
            database,
            unit_of_work,
            session_store: connected.session_store(),
            login_attempts: connected.login_attempts(),
            security_events: connected.security_events(),
            user,
        }
    }
}

#[async_trait]
impl Harness for PgHarness {
    fn unit_of_work(&self) -> Arc<dyn UnitOfWork> {
        Arc::clone(&self.unit_of_work)
    }

    fn user(&self) -> UserId {
        self.user
    }

    fn session_store(&self) -> Arc<dyn SessionStore> {
        Arc::clone(&self.session_store)
    }

    fn login_attempts(&self) -> Arc<dyn LoginAttemptStore> {
        Arc::clone(&self.login_attempts)
    }

    fn security_events(&self) -> Arc<dyn SecurityEventSink> {
        Arc::clone(&self.security_events)
    }

    async fn seed_attempts(
        &self,
        login: &TypedLogin,
        client_ip: IpAddr,
        ages: &[u64],
        succeeded: bool,
    ) -> Vec<AttemptId> {
        let ages: Vec<i64> = ages
            .iter()
            .map(|age| i64::try_from(*age).unwrap())
            .collect();
        let mut migrator = self.database.connect(Role::Migrator).await;
        let keys: Vec<i64> = sqlx::query_scalar(SEED_ATTEMPTS)
            .bind(login.as_str())
            .bind(client_ip.to_string())
            .bind(ages)
            .bind(succeeded)
            .fetch_all(&mut migrator)
            .await
            .unwrap();
        keys.into_iter().map(AttemptId::from).collect()
    }
}
