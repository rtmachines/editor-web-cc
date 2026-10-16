//! The harness of `MemoryStore`.

use std::sync::Arc;

use async_trait::async_trait;
use recipe_app::memory::{MemoryClock, MemoryStore};
use std::net::IpAddr;

use recipe_app::ports::{
    AttemptId, Clock as _, LoginAttemptStore, SecurityEventSink, SessionStore, UnitOfWork,
};
use recipe_domain::identity::{TypedLogin, UserId};
use recipe_domain::timestamp::Timestamp;

use super::harness::{Harness, seed_actor};

/// The in-memory adapter with the seed types of the plant model.
pub struct MemoryHarness {
    store: MemoryStore,
    user: UserId,
}

impl MemoryHarness {
    /// Build a store with a clock at the Unix epoch, the primitive types, and the harness user.
    ///
    /// The user comes through `UserRepository::insert`, as in the postgres harness.
    pub async fn create() -> Self {
        let store = MemoryStore::with_clock(MemoryClock::default());
        store.seed_primitive_types();
        let user = seed_actor(&store).await;
        Self { store, user }
    }
}

#[async_trait]
impl Harness for MemoryHarness {
    fn unit_of_work(&self) -> Arc<dyn UnitOfWork> {
        Arc::new(self.store.clone())
    }

    fn user(&self) -> UserId {
        self.user
    }

    fn session_store(&self) -> Arc<dyn SessionStore> {
        Arc::new(self.store.clone())
    }

    fn login_attempts(&self) -> Arc<dyn LoginAttemptStore> {
        Arc::new(self.store.clone())
    }

    fn security_events(&self) -> Arc<dyn SecurityEventSink> {
        Arc::new(self.store.sink())
    }

    async fn seed_attempts(
        &self,
        login: &TypedLogin,
        client_ip: IpAddr,
        ages: &[u64],
        succeeded: bool,
    ) -> Vec<AttemptId> {
        let now = self.store.clock().now();
        ages.iter()
            .map(|age| {
                let age = time::Duration::seconds(i64::try_from(*age).unwrap());
                let at = now.as_offset_date_time().checked_sub(age).unwrap();
                self.store
                    .seed_attempt(login, client_ip, Timestamp::from(at), succeeded)
            })
            .collect()
    }
}
