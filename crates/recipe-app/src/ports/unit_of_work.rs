//! Port of the transaction and the only commit path (design D5, ADR-0015).

use async_trait::async_trait;
use recipe_domain::audit::AuditEntry;

use super::{
    DraftRepository, Isolation, NonEmpty, PlantRepository, RecipeRepository, TypeCatalog,
    UnitLookup,
};
use crate::error::{AppError, StorageError};

/// Proof that the audit entries of a command are written.
///
/// The token solves one problem: a use case must not finish a transaction without
/// audit entries (I13). The type has no public constructor. Only
/// `UnitOfWork::commit`, in this module, builds it after `append_audit` succeeds.
///
/// ```compile_fail
/// let _token = recipe_app::ports::AuditWritten::new();
/// ```
#[derive(Debug)]
pub struct AuditWritten {
    _private: (),
}

impl AuditWritten {
    const fn new() -> Self {
        Self { _private: () }
    }
}

/// Opener of transactions.
///
/// The port solves one problem: a command must run in one transaction, and a test
/// must run the same command without a database. `recipe-db` implements it with
/// `sqlx`, and `memory::MemoryStore` implements it in memory.
#[async_trait]
pub trait UnitOfWork: Send + Sync {
    /// Open one transaction with the given isolation.
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError>;
}

impl<'a> dyn UnitOfWork + 'a {
    /// Write the audit entries and commit the transaction.
    ///
    /// This is the only commit path of ADR-0015. It is an inherent method of
    /// `dyn UnitOfWork`, so an adapter cannot replace it. It calls
    /// [`Transaction::append_audit`] first. A failure there gives
    /// `AppError::AuditWriteFailed` (`E-AUD-001`) with the storage error as its source
    /// and drops the transaction, which rolls it back. Then it calls
    /// [`Transaction::finish`]. A failure there stays `AppError::Storage`. The function
    /// logs nothing, because the `WebError` mapping logs each error once.
    ///
    /// # Errors
    ///
    /// Returns `AuditWriteFailed` when the entries are not written, and `Storage`
    /// when the commit fails.
    pub async fn commit(
        &self,
        mut tx: Box<dyn Transaction>,
        entries: NonEmpty<AuditEntry>,
    ) -> Result<(), AppError> {
        if let Err(cause) = tx.append_audit(&entries).await {
            return Err(AppError::AuditWriteFailed { cause });
        }
        tx.finish(AuditWritten::new()).await?;
        Ok(())
    }
}

/// One open transaction.
///
/// Dropping a transaction without `finish` rolls it back. Each sub-port is reached
/// through an accessor.
#[async_trait]
pub trait Transaction: Send {
    /// Return the repository of the Recipe root in this transaction.
    fn recipes(&mut self) -> &mut dyn RecipeRepository;
    /// Return the repository of the Draft aggregate in this transaction.
    fn drafts(&mut self) -> &mut dyn DraftRepository;
    /// Return the repository of the Machine aggregate in this transaction.
    fn plant(&mut self) -> &mut dyn PlantRepository;

    /// Return the read port of the units in this transaction.
    fn units(&mut self) -> &mut dyn UnitLookup;

    /// Return the PLC type catalog in this transaction.
    fn types(&mut self) -> &mut dyn TypeCatalog;

    /// Write the audit entries in this transaction, in the order of the list.
    async fn append_audit(&mut self, entries: &NonEmpty<AuditEntry>) -> Result<(), StorageError>;

    /// Commit the transaction. The token proves that the audit entries are written.
    async fn finish(self: Box<Self>, written: AuditWritten) -> Result<(), StorageError>;
}
