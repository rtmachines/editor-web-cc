//! The `Harness` of the contract suite (design D13).
//!
//! A harness gives one case a unit of work and a user that exists for the adapter. The case
//! seeds and reads back only through the ports, so it needs no access to the internals of
//! either adapter. Both harnesses insert their user through `UserRepository::insert`.

use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;
use recipe_app::ports::{
    AttemptId, Isolation, LoginAttemptStore, NewUser, NonEmpty, RowImage, SessionStore,
    Transaction, UnitOfWork,
};
use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::export::Sha256;
use recipe_domain::identity::{Login, PasswordHash, Role, TypedLogin, UserId};
use recipe_domain::plant::MachineId;
use serde_json::json;
use uuid::Uuid;

/// The login of the user that every harness inserts at its creation.
const HARNESS_LOGIN: &str = "harness-admin";
/// A hash that satisfies the format check of the table and of `PasswordHash::parse`.
pub const FIXTURE_PASSWORD_HASH: &str = "$argon2id$fixture";

/// Build the values of a user for a seed. The login must match the grammar of `Login`.
pub fn new_user(login: &str, role: Role) -> NewUser {
    NewUser {
        id: UserId::from(Uuid::new_v4()),
        login: Login::parse(login).unwrap(),
        password_hash: PasswordHash::parse(FIXTURE_PASSWORD_HASH).unwrap(),
        role,
        must_change_password: false,
    }
}

/// Insert a user through `UserRepository::insert` and commit it with `UserCreated`.
///
/// The entry has no actor, as the entry of `bootstrap_administrator` has none. The function
/// returns the row image of the insert.
pub async fn seed_user(uow: &dyn UnitOfWork, user: &NewUser) -> RowImage {
    let mut tx = uow.begin(Isolation::ReadCommitted).await.unwrap();
    let image = tx.users().insert(user).await.unwrap();
    let entry = AuditEntry::row(
        RowAction::UserCreated,
        AuditTarget::app_user(user.id),
        RowChange::Create {
            after: image.as_value().clone(),
        },
        AuditContext {
            actor: None,
            request_id: RequestId::from(Uuid::new_v4()),
            client_ip: None,
            session_ref: None,
        },
    );
    uow.commit(tx, NonEmpty::new(entry)).await.unwrap();
    image
}

/// Insert the user that the harness names as actor of its seeds and return its identifier.
pub async fn seed_actor(uow: &dyn UnitOfWork) -> UserId {
    let user = new_user(HARNESS_LOGIN, Role::Administrator);
    seed_user(uow, &user).await;
    user.id
}

/// One adapter under test: `MemoryStore` or `Database`.
#[async_trait]
pub trait Harness: Send + Sync {
    /// The unit of work of the adapter. Both adapters commit through `UnitOfWork::commit`.
    fn unit_of_work(&self) -> Arc<dyn UnitOfWork>;

    /// A user that exists for the adapter. The audit entries of the seeds name it as actor.
    fn user(&self) -> UserId;

    /// The pool-level `SessionStore` of the adapter (task 5.5).
    fn session_store(&self) -> Arc<dyn SessionStore>;

    /// The pool-level `LoginAttemptStore` of the adapter (task 5.6).
    fn login_attempts(&self) -> Arc<dyn LoginAttemptStore>;

    /// Insert attempts with a fixed time: `ages[i]` seconds before the time of the adapter.
    ///
    /// The ports cannot set `attempted_at`, so the postgres harness inserts as `recipe_migrator`
    /// (design D19). The keys follow the order of `ages`.
    async fn seed_attempts(
        &self,
        login: &TypedLogin,
        client_ip: IpAddr,
        ages: &[u64],
        succeeded: bool,
    ) -> Vec<AttemptId>;
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
