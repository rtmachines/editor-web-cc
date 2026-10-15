//! `TypeCatalog` on the transaction (task 9.4). The SQL is in `type_rows`.

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{DerivedTypeLock, Ensured, RowImage, TypeCatalog, TypeUsageRow};
use recipe_domain::plant::{MachineId, PlcType, PlcTypeId, PlcTypeRecord};

use super::PgTransaction;
use super::{type_rows, type_usage};

#[async_trait]
impl TypeCatalog for PgTransaction {
    async fn type_rows(&mut self, machine: MachineId) -> Result<Vec<PlcType>, StorageError> {
        type_rows::rows(&mut self.tx, machine).await
    }

    async fn type_usage(
        &mut self,
        machine: MachineId,
        type_id: PlcTypeId,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<TypeUsageRow>, StorageError> {
        type_usage::usage(&mut self.tx, machine, type_id, offset, limit).await
    }

    async fn lock_derived_types(
        &mut self,
        machine: MachineId,
        keys: &[DerivedTypeLock],
    ) -> Result<(), StorageError> {
        type_rows::lock_derived(&mut self.tx, machine, keys).await
    }

    async fn ensure_derived(&mut self, record: PlcTypeRecord) -> Result<Ensured, StorageError> {
        type_rows::ensure_derived(&mut self.tx, record).await
    }

    async fn insert_struct(&mut self, row: &PlcType) -> Result<RowImage, StorageError> {
        type_rows::insert_struct(&mut self.tx, row).await
    }
}
