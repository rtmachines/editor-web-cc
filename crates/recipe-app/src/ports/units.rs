//! Read port of the units for Recipe Management (`domain-model.md` section 1).

use async_trait::async_trait;
use recipe_domain::plant::{LiveUnits, MachineId, Unit, UnitId, UnitPath};

use crate::error::StorageError;

/// Read port of the units of a machine.
///
/// The port solves one problem: Recipe Management reads units and takes `FOR SHARE`
/// on them, but it must not change the Machine aggregate or hold its repository.
#[async_trait]
pub trait UnitLookup: Send {
    /// Read the machine and every unit with its live state, without a lock.
    ///
    /// A missing machine gives `None`.
    async fn live_units(&mut self, machine: MachineId) -> Result<Option<LiveUnits>, StorageError>;

    /// Take `unit FOR SHARE` on the unit at `path` (`AddUnit` parent, `AddParameter` unit).
    ///
    /// A missing unit gives `None` and takes no lock.
    async fn lock_unit_shared(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Option<Unit>, StorageError>;

    /// Take `unit FOR SHARE` on `units`, ordered by `id` (`CreateVersion`).
    ///
    /// An unknown identifier is not in the result.
    async fn lock_units_shared(&mut self, units: &[UnitId]) -> Result<Vec<Unit>, StorageError>;
}
