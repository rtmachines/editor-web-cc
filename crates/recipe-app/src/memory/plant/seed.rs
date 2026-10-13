//! Seed helpers for the plant tables. They need no transaction.

use recipe_domain::plant::Machine;

use crate::memory::MemoryStore;
use crate::memory::rows::{MachineRow, UnitRow};

impl MemoryStore {
    /// Insert a machine and all its units into the committed state, without a constraint check.
    ///
    /// The time of the clock is `created_at` and `updated_at` of every row. A fixture uses
    /// this method to prepare the state that a command starts from.
    pub fn seed_machine(&self, machine: &Machine) {
        self.update_state(|state, now| {
            let row = MachineRow {
                id: machine.id(),
                code: machine.code(),
                name: machine.name().clone(),
                platform: machine.platform(),
                archived_at: machine.archived_at(),
                created_at: now,
                updated_at: now,
            };
            state.machines.insert(row.id, row);
            for unit in machine.units() {
                let row = UnitRow {
                    id: unit.id(),
                    machine: machine.id(),
                    path: unit.path().clone(),
                    name: unit.name().clone(),
                    archived_at: unit.archived_at(),
                    created_at: now,
                    updated_at: now,
                };
                state.units.insert(row.id, row);
            }
        });
    }
}
