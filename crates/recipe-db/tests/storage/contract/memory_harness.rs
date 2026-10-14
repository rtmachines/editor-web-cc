//! The harness of `MemoryStore`.

use std::sync::Arc;

use async_trait::async_trait;
use recipe_app::memory::{MemoryClock, MemoryStore};
use recipe_app::ports::UnitOfWork;
use recipe_domain::identity::UserId;
use uuid::Uuid;

use super::harness::Harness;

/// The in-memory adapter with the seed types of the plant model.
pub struct MemoryHarness {
    store: MemoryStore,
    user: UserId,
}

impl MemoryHarness {
    /// Build an empty store with a clock at the Unix epoch and the primitive types.
    pub fn new() -> Self {
        let store = MemoryStore::with_clock(MemoryClock::default());
        store.seed_primitive_types();
        Self {
            store,
            user: UserId::from(Uuid::new_v4()),
        }
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
}
