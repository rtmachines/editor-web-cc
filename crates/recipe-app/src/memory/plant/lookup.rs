//! In-memory `UnitLookup`.

use std::collections::BTreeSet;

use async_trait::async_trait;
use recipe_domain::plant::{LiveUnits, MachineId, Unit, UnitId, UnitPath};

use super::UNIT_TABLE;
use crate::error::StorageError;
use crate::memory::rows::corrupt;
use crate::memory::{LockMode, MemoryTransaction};
use crate::ports::UnitLookup;

#[async_trait]
impl UnitLookup for MemoryTransaction {
    async fn live_units(&mut self, machine: MachineId) -> Result<Option<LiveUnits>, StorageError> {
        let found = self.state().try_machine(machine).map_err(corrupt)?;
        Ok(found.map(|machine| machine.live_units()))
    }

    async fn lock_unit_shared(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Option<Unit>, StorageError> {
        let Some(row) = self.unit_row_at(machine, path) else {
            return Ok(None);
        };
        let id = row.id;
        let Some(unit) = self.unit_of(row)? else {
            return Ok(None);
        };
        self.lock(UNIT_TABLE, LockMode::ForShare, &id.to_string());
        Ok(Some(unit))
    }

    async fn lock_units_shared(&mut self, units: &[UnitId]) -> Result<Vec<Unit>, StorageError> {
        let wanted: BTreeSet<UnitId> = units.iter().copied().collect();
        let mut found = Vec::new();
        for id in wanted {
            let Some(row) = self.state().units.get(&id) else {
                continue;
            };
            if let Some(unit) = self.unit_of(row)? {
                found.push(unit);
            }
        }
        for unit in &found {
            self.lock(UNIT_TABLE, LockMode::ForShare, &unit.id().to_string());
        }
        Ok(found)
    }
}
