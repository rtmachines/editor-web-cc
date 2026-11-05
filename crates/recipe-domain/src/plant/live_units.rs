//! The units of one machine as a read value (`domain-model.md` section 2.1, I15).

use std::collections::BTreeMap;

use super::{Machine, MachineCode, MachineId, PlantError, UnitId, UnitPath};

/// A unit that was live when `LiveUnits::require` returned it.
///
/// The type solves one problem: a parameter takes only a unit that exists, is live, and
/// belongs to the machine of the recipe (I15). Only [`LiveUnits::require`] creates it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveUnit {
    machine: MachineId,
    id: UnitId,
    path: UnitPath,
}

impl LiveUnit {
    /// Return the machine of the unit.
    #[must_use]
    pub const fn machine(&self) -> MachineId {
        self.machine
    }

    /// Return the identifier of the unit.
    #[must_use]
    pub const fn id(&self) -> UnitId {
        self.id
    }

    /// Return the path of the unit.
    #[must_use]
    pub const fn path(&self) -> &UnitPath {
        &self.path
    }
}

/// The state of one unit inside [`LiveUnits`].
#[derive(Clone, Copy, Debug)]
struct UnitState {
    id: UnitId,
    is_live: bool,
}

/// The machine and every unit of the machine with its live state.
///
/// Only [`Machine::live_units`] builds this value.
#[derive(Clone, Debug)]
pub struct LiveUnits {
    machine: MachineId,
    code: MachineCode,
    is_archived: bool,
    units: BTreeMap<UnitPath, UnitState>,
}

impl LiveUnits {
    /// Read the units of `machine`.
    pub(super) fn of(machine: &Machine) -> Self {
        let units = machine
            .units()
            .map(|unit| {
                let state = UnitState {
                    id: unit.id(),
                    is_live: unit.is_live(),
                };
                (unit.path().clone(), state)
            })
            .collect();
        Self {
            machine: machine.id(),
            code: machine.code(),
            is_archived: !machine.is_live(),
            units,
        }
    }

    /// Return the identifier of the machine.
    #[must_use]
    pub const fn machine(&self) -> MachineId {
        self.machine
    }

    /// Return the code of the machine.
    #[must_use]
    pub const fn machine_code(&self) -> MachineCode {
        self.code
    }

    /// Return true when the machine is archived.
    #[must_use]
    pub const fn is_machine_archived(&self) -> bool {
        self.is_archived
    }

    /// Return the path of a unit, whether the unit is live or archived.
    #[must_use]
    pub fn path_of(&self, unit: UnitId) -> Option<&UnitPath> {
        self.units
            .iter()
            .find_map(|(path, state)| (state.id == unit).then_some(path))
    }

    /// Return true when the unit exists and is not archived.
    #[must_use]
    pub fn is_live(&self, unit: UnitId) -> bool {
        self.units
            .values()
            .any(|state| state.id == unit && state.is_live)
    }

    /// Return the live unit at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`PlantError::UnitNotFound`] (`E-PLT-020`) when no unit has `path`.
    /// Returns [`PlantError::UnitArchived`] (`E-PLT-009`) when the unit is archived.
    pub fn require(&self, path: &UnitPath) -> Result<LiveUnit, PlantError> {
        let state = self
            .units
            .get(path)
            .ok_or_else(|| PlantError::UnitNotFound { path: path.clone() })?;
        if !state.is_live {
            return Err(PlantError::UnitArchived { path: path.clone() });
        }
        Ok(LiveUnit {
            machine: self.machine,
            id: state.id,
            path: path.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
