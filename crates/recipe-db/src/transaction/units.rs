//! `UnitLookup` on the transaction (task 9.4). The SQL is in `unit_rows` and `machines`.

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::UnitLookup;
use recipe_domain::plant::{LiveUnits, MachineId, Unit, UnitId, UnitPath};

use super::PgTransaction;
use super::{machines, unit_rows};

#[async_trait]
impl UnitLookup for PgTransaction {
    async fn live_units(&mut self, machine: MachineId) -> Result<Option<LiveUnits>, StorageError> {
        let found = machines::by_id(&mut self.tx, machine).await?;
        Ok(found.map(|machine| machine.live_units()))
    }

    async fn lock_unit_shared(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Option<Unit>, StorageError> {
        unit_rows::lock_shared_at_path(&mut self.tx, machine, path).await
    }

    async fn lock_units_shared(&mut self, units: &[UnitId]) -> Result<Vec<Unit>, StorageError> {
        unit_rows::lock_shared_by_ids(&mut self.tx, units).await
    }
}
