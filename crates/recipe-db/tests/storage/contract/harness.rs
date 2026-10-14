//! The `Harness` of the contract suite (design D13).
//!
//! A harness gives one case a unit of work and a user that exists for the adapter. The case
//! seeds and reads back only through the ports, so it needs no access to the internals of
//! either adapter. The one exception is the `app_user` row of the postgres harness, because
//! the user port belongs to P2.5.

use std::sync::Arc;

use async_trait::async_trait;
use recipe_app::ports::{Isolation, NonEmpty, Transaction, UnitOfWork};
use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::export::Sha256;
use recipe_domain::identity::UserId;
use recipe_domain::plant::MachineId;
use serde_json::json;
use uuid::Uuid;

/// One adapter under test: `MemoryStore` or `Database`.
#[async_trait]
pub trait Harness: Send + Sync {
    /// The unit of work of the adapter. Both adapters commit through `UnitOfWork::commit`.
    fn unit_of_work(&self) -> Arc<dyn UnitOfWork>;

    /// A user that exists for the adapter. The audit entries of the seeds name it as actor.
    fn user(&self) -> UserId;
}

/// The hash that the approval seed stores: 32 bytes of `0xab`.
pub fn fixture_hash() -> Sha256 {
    Sha256::from([0xab_u8; 32])
}

/// The change note that the recipe seed and the version seed store.
pub const FIXTURE_NOTE: &str = "Fixture note";

/// Open a read-committed transaction.
pub async fn begin(harness: &dyn Harness) -> Box<dyn Transaction> {
    harness
        .unit_of_work()
        .begin(Isolation::ReadCommitted)
        .await
        .unwrap()
}

/// A valid audit list for a seed. The contract suite does not compare audit content.
fn seed_entries(harness: &dyn Harness, machine: MachineId) -> NonEmpty<AuditEntry> {
    NonEmpty::new(AuditEntry::row(
        RowAction::MachineCreated,
        AuditTarget::machine(machine),
        RowChange::Create { after: json!({}) },
        AuditContext {
            actor: Some(harness.user()),
            request_id: RequestId::from(Uuid::new_v4()),
            client_ip: None,
            session_ref: None,
        },
    ))
}

/// Commit a transaction with a valid audit list that names `machine`.
pub async fn commit(harness: &dyn Harness, tx: Box<dyn Transaction>, machine: MachineId) {
    harness
        .unit_of_work()
        .commit(tx, seed_entries(harness, machine))
        .await
        .unwrap();
}
