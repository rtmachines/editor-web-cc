//! In-memory `PlantRepository` and `UnitLookup` (task 4.6).

mod list;
mod lookup;
mod seed;

/// Refuse a page offset that the SQL adapter cannot bind as `bigint`.
///
/// The check solves one problem: the memory adapter and the SQL adapter must answer the same
/// request in the same way. An offset above `i64::MAX` gives the `StorageError` that
/// `recipe-db` gives (`E-DB-008`), not an empty list.
///
/// # Errors
///
/// Returns `StorageError::Other` when `offset` is above `i64::MAX`.
pub(super) fn check_offset(offset: u64) -> Result<(), StorageError> {
    i64::try_from(offset)
        .map(drop)
        .map_err(|_| StorageError::invariant("the page offset does not fit the type bigint"))
}

use async_trait::async_trait;
use recipe_domain::plant::{Machine, MachineCode, MachineId, Unit, UnitId, UnitPath, UnitUsage};
use recipe_domain::recipe::VersionState;
use recipe_domain::text::Text;

use super::images::{machine_image, unit_image};
use super::rows::{
    MachineRow, UnitRow, corrupt, corrupt_rows, duplicate, missing_reference, missing_row,
};
use super::{LockMode, MemoryTransaction};
use crate::error::{MACHINE_CODE_LIVE_UNIQUE, StorageError, UNIT_PARENT_EXISTS, UNIT_PATH_UNIQUE};
use crate::ports::{
    InsertedMachine, Locked, MachineFilter, MachineSummary, PlantRepository, RowImage,
};

/// The table name of the journal for a machine lock.
const MACHINE_TABLE: &str = "machine";
/// The table name of the journal for a unit lock.
const UNIT_TABLE: &str = "unit";
/// The name of the primary key of `machine`.
const MACHINE_PKEY: &str = "machine_pkey";
/// The name of the primary key of `unit`.
const UNIT_PKEY: &str = "unit_pkey";
/// The name of the foreign key from `unit` to `machine`.
const UNIT_MACHINE_FKEY: &str = "unit_machine_id_fkey";

impl MemoryTransaction {
    /// Lock a machine and return its state with the row image before the change.
    fn lock_machine(
        &self,
        id: MachineId,
        mode: LockMode,
    ) -> Result<Option<Locked<Machine>>, StorageError> {
        let Some(row) = self.state().machines.get(&id) else {
            return Ok(None);
        };
        let before = machine_image(row);
        let Some(state) = self.state().try_machine(id).map_err(corrupt)? else {
            return Ok(None);
        };
        self.lock(MACHINE_TABLE, mode, &id.to_string());
        Ok(Some(Locked { state, before }))
    }

    /// Find the unit row of a machine at `path`.
    fn unit_row_at(&self, machine: MachineId, path: &UnitPath) -> Option<&UnitRow> {
        self.state()
            .units
            .values()
            .find(|row| row.machine == machine && row.path == *path)
    }

    /// Build the domain unit of a row.
    fn unit_of(&self, row: &UnitRow) -> Result<Option<Unit>, StorageError> {
        let machine = self.state().try_machine(row.machine).map_err(corrupt)?;
        Ok(machine.and_then(|machine| machine.unit(&row.path).cloned()))
    }

    fn unit_row_mut(&mut self, id: UnitId) -> Result<&mut UnitRow, StorageError> {
        self.state_mut()
            .units
            .get_mut(&id)
            .ok_or_else(|| missing_row(UNIT_TABLE))
    }

    fn machine_row_mut(&mut self, id: MachineId) -> Result<&mut MachineRow, StorageError> {
        self.state_mut()
            .machines
            .get_mut(&id)
            .ok_or_else(|| missing_row(MACHINE_TABLE))
    }

    /// Add a unit row after the checks of the constraints of `unit`.
    fn put_unit(&mut self, machine: MachineId, unit: &Unit) -> Result<RowImage, StorageError> {
        let state = self.state();
        if !state.machines.contains_key(&machine) {
            return Err(missing_reference(UNIT_MACHINE_FKEY));
        }
        if state.units.contains_key(&unit.id()) {
            return Err(duplicate(UNIT_PKEY));
        }
        if self.unit_row_at(machine, unit.path()).is_some() {
            return Err(duplicate(UNIT_PATH_UNIQUE));
        }
        let has_parent = unit
            .path()
            .parent()
            .is_none_or(|parent| self.unit_row_at(machine, &parent).is_some());
        if !has_parent {
            return Err(missing_reference(UNIT_PARENT_EXISTS));
        }
        let now = self.now();
        let row = UnitRow {
            id: unit.id(),
            machine,
            path: unit.path().clone(),
            name: unit.name().clone(),
            archived_at: unit.archived_at(),
            created_at: now,
            updated_at: now,
        };
        let image = unit_image(&row);
        self.state_mut().units.insert(row.id, row);
        Ok(image)
    }
}

#[async_trait]
impl PlantRepository for MemoryTransaction {
    async fn machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError> {
        let found = self
            .state()
            .machines
            .values()
            .find(|row| row.code == code && row.archived_at.is_none())
            .map(|row| row.id);
        match found {
            Some(id) => self.state().try_machine(id).map_err(corrupt),
            None => Ok(None),
        }
    }

    async fn archived_machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError> {
        // The machines come from a `BTreeMap` keyed by id, so the first match has the lowest id.
        let found = self
            .state()
            .machines
            .values()
            .find(|row| row.code == code && row.archived_at.is_some())
            .map(|row| row.id);
        match found {
            Some(id) => self.state().try_machine(id).map_err(corrupt),
            None => Ok(None),
        }
    }

    async fn machine_by_id(&mut self, id: MachineId) -> Result<Option<Machine>, StorageError> {
        self.state().try_machine(id).map_err(corrupt)
    }

    async fn list_machines(
        &mut self,
        filter: &MachineFilter,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<MachineSummary>, StorageError> {
        check_offset(offset)?;
        Ok(list::list(self.state(), filter, offset, limit))
    }

    async fn lock_machine_shared(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError> {
        self.lock_machine(id, LockMode::ForShare)
    }

    async fn lock_machine_for_no_key_update(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError> {
        self.lock_machine(id, LockMode::ForNoKeyUpdate)
    }

    async fn insert_machine(&mut self, machine: &Machine) -> Result<InsertedMachine, StorageError> {
        let state = self.state();
        if state.machines.contains_key(&machine.id()) {
            return Err(duplicate(MACHINE_PKEY));
        }
        let code_taken = state
            .machines
            .values()
            .any(|row| row.code == machine.code() && row.archived_at.is_none());
        if machine.is_live() && code_taken {
            return Err(duplicate(MACHINE_CODE_LIVE_UNIQUE));
        }
        let now = self.now();
        let row = MachineRow {
            id: machine.id(),
            code: machine.code(),
            name: machine.name().clone(),
            platform: machine.platform(),
            archived_at: machine.archived_at(),
            created_at: now,
            updated_at: now,
        };
        let image = machine_image(&row);
        self.state_mut().machines.insert(row.id, row);
        let mut units = Vec::new();
        for unit in machine.units() {
            units.push(self.put_unit(machine.id(), unit)?);
        }
        Ok(InsertedMachine {
            machine: image,
            units,
        })
    }

    async fn rename_machine(
        &mut self,
        id: MachineId,
        name: &Text,
    ) -> Result<RowImage, StorageError> {
        let now = self.now();
        let row = self.machine_row_mut(id)?;
        row.name = name.clone();
        row.updated_at = now;
        Ok(machine_image(row))
    }

    async fn archive_machine(&mut self, id: MachineId) -> Result<RowImage, StorageError> {
        let now = self.now();
        let row = self.machine_row_mut(id)?;
        row.archived_at = Some(now);
        row.updated_at = now;
        Ok(machine_image(row))
    }

    async fn insert_unit(
        &mut self,
        machine: MachineId,
        unit: &Unit,
    ) -> Result<RowImage, StorageError> {
        if let Some(fault) = self.insert_unit_fault() {
            return Err(fault.into());
        }
        self.put_unit(machine, unit)
    }

    async fn lock_unit_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Option<Locked<Unit>>, StorageError> {
        let Some(row) = self.unit_row_at(machine, path) else {
            return Ok(None);
        };
        let before = unit_image(row);
        let id = row.id;
        let Some(state) = self.unit_of(row)? else {
            return Ok(None);
        };
        self.lock(UNIT_TABLE, LockMode::ForUpdate, &id.to_string());
        Ok(Some(Locked { state, before }))
    }

    async fn rename_unit(&mut self, unit: UnitId, name: &Text) -> Result<RowImage, StorageError> {
        let now = self.now();
        let row = self.unit_row_mut(unit)?;
        row.name = name.clone();
        row.updated_at = now;
        Ok(unit_image(row))
    }

    async fn lock_unit_tree_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Vec<Locked<Unit>>, StorageError> {
        // The units come from a `BTreeMap` keyed by id, so they are ordered by id.
        let rows: Vec<&UnitRow> = self
            .state()
            .units
            .values()
            .filter(|row| row.machine == machine)
            .filter(|row| row.path == *path || path.is_ancestor_of(&row.path))
            .collect();
        let mut locked = Vec::new();
        for row in rows {
            if let Some(state) = self.unit_of(row)? {
                locked.push((
                    row.id,
                    Locked {
                        state,
                        before: unit_image(row),
                    },
                ));
            }
        }
        for (id, _) in &locked {
            self.lock(UNIT_TABLE, LockMode::ForUpdate, &id.to_string());
        }
        Ok(locked.into_iter().map(|(_, entry)| entry).collect())
    }

    async fn archive_units(&mut self, units: &[UnitId]) -> Result<Vec<RowImage>, StorageError> {
        let now = self.now();
        let mut images = Vec::new();
        for id in units {
            let row = self.unit_row_mut(*id)?;
            row.archived_at = Some(now);
            row.updated_at = now;
            images.push(unit_image(row));
        }
        Ok(images)
    }

    async fn unit_usage(
        &mut self,
        machine: MachineId,
        units: &[UnitId],
    ) -> Result<UnitUsage, StorageError> {
        let state = self.state();
        let mut usage = UnitUsage::default();
        for parameter in state.parameters.values() {
            let unit = parameter.unit;
            let in_machine = state
                .units
                .get(&unit)
                .is_some_and(|row| row.machine == machine);
            if !in_machine || !units.contains(&unit) {
                continue;
            }
            let Some(version) = state.versions.get(&parameter.version) else {
                continue;
            };
            let Some(recipe) = state.recipes.get(&version.recipe) else {
                continue;
            };
            let reference = recipe.reference.clone();
            match version.state().map_err(|reason| corrupt_rows(&reason))? {
                VersionState::LiveDraft => usage.add_draft(unit, reference),
                VersionState::Approved => usage.add_approved(unit, reference),
                VersionState::Discarded | VersionState::Retired => {}
            }
        }
        Ok(usage)
    }
}

#[cfg(test)]
mod tests;
