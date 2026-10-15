//! Port of the Machine aggregate (`data-model.md` section 6, design D5).

use async_trait::async_trait;
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlcPlatform, Unit, UnitId, UnitPath, UnitUsage,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;

use super::{Locked, RowImage};
use crate::error::StorageError;

/// The filter of `PlantRepository::list_machines`.
///
/// The type solves one problem: the list of machines has two independent choices, and a
/// struct names each choice at the call site.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MachineFilter {
    /// A text that the code or the name must contain, with no difference of case. `None` keeps every machine.
    pub text: Option<Text>,
    /// Whether the list also holds the archived machines.
    pub include_archived: bool,
}

/// One row of the list of machines.
///
/// The type solves one problem: the list page needs five columns of a machine and not the
/// whole aggregate with its unit tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineSummary {
    /// The identifier of the machine.
    pub id: MachineId,
    /// The code of the machine.
    pub code: MachineCode,
    /// The name of the machine.
    pub name: Text,
    /// The PLC platform of the machine.
    pub platform: PlcPlatform,
    /// The instant of archiving, or `None` for a live machine.
    pub archived_at: Option<Timestamp>,
}

/// The row images that `CreateMachine` writes.
///
/// The type solves one problem: `CreateMachine` writes one `MachineCreated` entry
/// and one `UnitAdded` entry for `U1`, and each entry needs its own image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertedMachine {
    /// The image of the new machine row.
    pub machine: RowImage,
    /// The images of the new unit rows, in path order. A new machine has the unit `U1`.
    pub units: Vec<RowImage>,
}

/// Repository of the Machine aggregate: machines and their units.
///
/// The port solves one problem: the Plant Model commands need the locks, the writes, and
/// the row images of `data-model.md` section 6 without a database in the use case.
/// A lock method takes the SQL lock in its name and returns the locked state with the row
/// image before the change. A read method takes no lock. A write method names its intent
/// and returns the row image after the change.
#[async_trait]
pub trait PlantRepository: Send {
    /// Read the live machine with `code` without a lock (A32, `/machines/{code}`).
    async fn machine_by_code(&mut self, code: MachineCode)
    -> Result<Option<Machine>, StorageError>;

    /// Read an archived machine with `code` without a lock (`CreateRecipe`).
    ///
    /// The command needs it to tell `E-PLT-010` from `E-PLT-019`. A live machine gives `None`.
    /// Several archived machines can share a code. Then the adapter returns the first by `id`.
    async fn archived_machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError>;

    /// Read a machine, live or archived, without a lock (`/machines/by-id/{uuid}`).
    async fn machine_by_id(&mut self, id: MachineId) -> Result<Option<Machine>, StorageError>;

    /// Read one page of the machines without a lock (`/machines`).
    ///
    /// A row matches when `filter.text` is `None`, or when the code or the name contains the text
    /// with no difference of case. An archived machine matches only with `filter.include_archived`.
    /// The rows come in the order of the number of the code. For one code, the live machine comes
    /// first. Then come the archived machines, the latest archiving first. The identifier ends the order.
    /// The page starts after `offset` rows and holds at most `limit` rows.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Other` (`E-DB-008`) when `offset` is above `i64::MAX`.
    /// Both adapters return this error and never an empty list for such an offset.
    async fn list_machines(
        &mut self,
        filter: &MachineFilter,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<MachineSummary>, StorageError>;

    /// Take `machine FOR SHARE` (`AddUnit`, `CreateRecipe`, and the other commands of section 6).
    ///
    /// A missing machine gives `None` and takes no lock.
    async fn lock_machine_shared(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError>;

    /// Take `machine FOR NO KEY UPDATE` (`RenameMachine`, `ArchiveMachine`, `RenameUnit`, `ArchiveUnit`).
    ///
    /// A missing machine gives `None` and takes no lock.
    async fn lock_machine_for_no_key_update(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError>;

    /// Run `INSERT machine` and `INSERT unit` for every unit of `machine` (`CreateMachine`).
    ///
    /// A live machine with the same code gives `Duplicate` with `machine_code_live_unique`.
    async fn insert_machine(&mut self, machine: &Machine) -> Result<InsertedMachine, StorageError>;

    /// Run `UPDATE machine SET name` (`RenameMachine`) and return the row after the change.
    async fn rename_machine(
        &mut self,
        id: MachineId,
        name: &Text,
    ) -> Result<RowImage, StorageError>;

    /// Run `UPDATE machine SET archived_at = now()` (`ArchiveMachine`) and return the row after the change.
    async fn archive_machine(&mut self, id: MachineId) -> Result<RowImage, StorageError>;

    /// Run `INSERT unit` (`AddUnit`) and return the new row.
    ///
    /// A known path gives `Duplicate` with `unit_path_unique`.
    /// A missing parent gives `MissingReference` with `unit_parent_exists`.
    async fn insert_unit(
        &mut self,
        machine: MachineId,
        unit: &Unit,
    ) -> Result<RowImage, StorageError>;

    /// Take `unit FOR UPDATE` on the unit at `path` (`RenameUnit`).
    ///
    /// A missing unit gives `None` and takes no lock.
    async fn lock_unit_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Option<Locked<Unit>>, StorageError>;

    /// Run `UPDATE unit SET name` (`RenameUnit`) and return the row after the change.
    async fn rename_unit(&mut self, unit: UnitId, name: &Text) -> Result<RowImage, StorageError>;

    /// Take `unit FOR UPDATE` on the unit at `path` and its descendants, ordered by `id` (`ArchiveUnit`).
    async fn lock_unit_tree_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Vec<Locked<Unit>>, StorageError>;

    /// Run `UPDATE unit SET archived_at = now()` for `units` (`ArchiveUnit`).
    ///
    /// The result has one row image after the change for each unit, in the order of `units`.
    async fn archive_units(&mut self, units: &[UnitId]) -> Result<Vec<RowImage>, StorageError>;

    /// Read the live drafts and the approved versions that use `units` (`ArchiveUnit`).
    ///
    /// The statement takes no lock. `machine` names the machine of the units.
    async fn unit_usage(
        &mut self,
        machine: MachineId,
        units: &[UnitId],
    ) -> Result<UnitUsage, StorageError>;
}
