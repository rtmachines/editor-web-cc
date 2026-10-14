//! The harness of `Database`.

use std::sync::Arc;

use async_trait::async_trait;
use recipe_app::ports::UnitOfWork;
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::identity::UserId;

use super::harness::Harness;
use crate::support::fixtures;
use crate::support::{Role, TestDatabase};

/// The `sqlx` adapter on its own test database.
///
/// The adapter connects as `recipe_app`. One `app_user` row exists, inserted as
/// `recipe_migrator`, because the user port belongs to P2.5. This insert is the only SQL seed
/// of the contract suite (design D13).
pub struct PgHarness {
    /// The test database. A case with postgres-only checks opens its own sessions on it.
    pub database: TestDatabase,
    unit_of_work: Arc<dyn UnitOfWork>,
    user: UserId,
}

impl PgHarness {
    /// Create the test database, the user, and the pool.
    pub async fn create() -> Self {
        let database = TestDatabase::create().await;
        let mut migrator = database.connect(Role::Migrator).await;
        let user = fixtures::user(&mut migrator).await;
        let connected = Database::connect(&database.target(Role::App), &PoolSettings::default())
            .await
            .unwrap();
        Self {
            database,
            unit_of_work: connected.unit_of_work(),
            user: UserId::from(user),
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
}
