//! The open transaction (design D3).
//!
//! One struct implements the five sub-ports of `recipe-app`. Each accessor returns `self`, and
//! each port has its own module with its `impl`.

mod draft_rows;
mod draft_writes;
mod drafts;
mod machines;
mod parameter_rows;
mod plant;
mod recipe_rows;
mod recipe_writes;
mod recipes;
mod type_rows;
mod types;
mod unit_rows;
mod units;
mod version_rows;
mod version_writes;

use std::collections::BTreeMap;

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{
    AuditWritten, DraftRepository, NonEmpty, PlantRepository, RecipeRepository, Transaction,
    TypeCatalog, UnitLookup,
};
use recipe_domain::audit::AuditEntry;
use recipe_domain::recipe::RecipeVersionId;
use sqlx::Postgres;

use crate::audit_insert::insert_entries;
use crate::error::map_error;

/// A transaction on one pooled connection.
///
/// The type solves one problem: a command must see one connection from `BEGIN` to the commit,
/// and a transaction that nobody finishes must roll back. The `sqlx` transaction rolls back when
/// the value is dropped without `finish`.
pub(crate) struct PgTransaction {
    tx: sqlx::Transaction<'static, Postgres>,
    /// The `row_version` that each version lock read (ADR-0030). Both `RecipeRepository` and
    /// `DraftRepository` write it, and a status write reads it as its guard.
    locked_versions: BTreeMap<RecipeVersionId, i64>,
}

impl PgTransaction {
    pub(crate) const fn new(tx: sqlx::Transaction<'static, Postgres>) -> Self {
        Self {
            tx,
            locked_versions: BTreeMap::new(),
        }
    }

    /// Record the `row_version` that a lock or a status write left on a version.
    fn record_row_version(&mut self, version: RecipeVersionId, row_version: i64) {
        self.locked_versions.insert(version, row_version);
    }

    /// Return the `row_version` that the last lock or status write of this transaction recorded.
    fn recorded_row_version(&self, version: RecipeVersionId) -> Option<i64> {
        self.locked_versions.get(&version).copied()
    }
}

#[async_trait]
impl Transaction for PgTransaction {
    fn recipes(&mut self) -> &mut dyn RecipeRepository {
        self
    }

    fn drafts(&mut self) -> &mut dyn DraftRepository {
        self
    }

    fn plant(&mut self) -> &mut dyn PlantRepository {
        self
    }

    fn units(&mut self) -> &mut dyn UnitLookup {
        self
    }

    fn types(&mut self) -> &mut dyn TypeCatalog {
        self
    }

    async fn append_audit(&mut self, entries: &NonEmpty<AuditEntry>) -> Result<(), StorageError> {
        insert_entries(&mut self.tx, entries)
            .await
            .map_err(map_error)
    }

    /// Commit. A deferred trigger can fail here, so the error goes through the same map.
    async fn finish(self: Box<Self>, _written: AuditWritten) -> Result<(), StorageError> {
        self.tx.commit().await.map_err(map_error)
    }
}
