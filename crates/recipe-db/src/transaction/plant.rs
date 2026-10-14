//! `PlantRepository` on the transaction. The machine methods call `machines` (task 9.2). The unit methods call `unit_rows` (task 9.3).

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{InsertedMachine, Locked, PlantRepository, RowImage};
use recipe_domain::plant::{Machine, MachineCode, MachineId, Unit, UnitId, UnitPath, UnitUsage};
use recipe_domain::text::Text;

use super::PgTransaction;
use super::{machines, unit_rows};

#[async_trait]
impl PlantRepository for PgTransaction {
    async fn machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError> {
        machines::by_code(&mut self.tx, code).await
    }

    async fn archived_machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError> {
        machines::archived_by_code(&mut self.tx, code).await
    }

    async fn machine_by_id(&mut self, id: MachineId) -> Result<Option<Machine>, StorageError> {
        machines::by_id(&mut self.tx, id).await
    }

    async fn lock_machine_shared(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError> {
        machines::lock_shared(&mut self.tx, id).await
    }

    async fn lock_machine_for_no_key_update(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError> {
        machines::lock_no_key_update(&mut self.tx, id).await
    }

    async fn insert_machine(&mut self, machine: &Machine) -> Result<InsertedMachine, StorageError> {
        machines::insert(&mut self.tx, machine).await
    }

    async fn rename_machine(
        &mut self,
        id: MachineId,
        name: &Text,
    ) -> Result<RowImage, StorageError> {
        machines::rename(&mut self.tx, id, name).await
    }

    async fn archive_machine(&mut self, id: MachineId) -> Result<RowImage, StorageError> {
        machines::archive(&mut self.tx, id).await
    }

    async fn insert_unit(
        &mut self,
        machine: MachineId,
        unit: &Unit,
    ) -> Result<RowImage, StorageError> {
        machines::insert_unit(&mut self.tx, machine, unit).await
    }

    async fn lock_unit_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Option<Locked<Unit>>, StorageError> {
        unit_rows::lock_for_update(&mut self.tx, machine, path).await
    }

    async fn rename_unit(&mut self, unit: UnitId, name: &Text) -> Result<RowImage, StorageError> {
        unit_rows::rename(&mut self.tx, unit, name).await
    }

    async fn lock_unit_tree_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Vec<Locked<Unit>>, StorageError> {
        unit_rows::lock_tree(&mut self.tx, machine, path).await
    }

    async fn archive_units(&mut self, units: &[UnitId]) -> Result<Vec<RowImage>, StorageError> {
        unit_rows::archive(&mut self.tx, units).await
    }

    async fn unit_usage(
        &mut self,
        machine: MachineId,
        units: &[UnitId],
    ) -> Result<UnitUsage, StorageError> {
        unit_rows::usage(&mut self.tx, machine, units).await
    }
}
