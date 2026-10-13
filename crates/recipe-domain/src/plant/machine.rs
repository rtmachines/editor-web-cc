//! The machine aggregate and its unit tree (`domain-model.md` section 2.1).

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use crate::recipe::ReferenceNumber;
use crate::text::Text;
use crate::timestamp::Timestamp;

use super::{
    LiveUnits, MachineCode, MachineId, PlantError, PlcPlatform, UnitId, UnitPath, UnitUsage,
};

/// A unit of a machine, such as `U1.2`.
///
/// Only [`Machine`] creates and changes a unit. This keeps the unit tree valid (I10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    id: UnitId,
    path: UnitPath,
    name: Text,
    archived_at: Option<Timestamp>,
}

impl Unit {
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

    /// Return the name of the unit.
    #[must_use]
    pub const fn name(&self) -> &Text {
        &self.name
    }

    /// Return the instant of archiving, or `None` for a live unit.
    #[must_use]
    pub const fn archived_at(&self) -> Option<Timestamp> {
        self.archived_at
    }

    /// Return true when the unit is not archived.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.archived_at.is_none()
    }
}

/// A unit row as storage loads it.
///
/// Storage builds this record. `TryFrom<StoredMachine>` checks it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredUnit {
    /// The identifier of the unit.
    pub id: UnitId,
    /// The path of the unit.
    pub path: UnitPath,
    /// The name of the unit.
    pub name: Text,
    /// The instant of archiving, or `None` for a live unit.
    pub archived_at: Option<Timestamp>,
}

/// A machine row with its unit rows as storage loads them.
///
/// Storage builds this record. `TryFrom<StoredMachine>` checks it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredMachine {
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
    /// The unit rows of the machine, in any order.
    pub units: Vec<StoredUnit>,
}

/// A machine with its tree of units.
///
/// The type solves one problem: no code outside the aggregate can build an
/// invalid unit tree (I10) or change the platform (I21).
///
/// The fields are private and no method changes the platform. Outside this
/// crate, code cannot assign it:
///
/// ```compile_fail,E0616
/// use recipe_domain::plant::{Machine, PlcPlatform};
///
/// fn change_platform(machine: &mut Machine) {
///     machine.platform = PlcPlatform::TwinCat3;
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    id: MachineId,
    code: MachineCode,
    name: Text,
    platform: PlcPlatform,
    archived_at: Option<Timestamp>,
    units: BTreeMap<UnitPath, Unit>,
}

impl Machine {
    /// Return the identifier of the machine.
    #[must_use]
    pub const fn id(&self) -> MachineId {
        self.id
    }

    /// Return the code of the machine.
    #[must_use]
    pub const fn code(&self) -> MachineCode {
        self.code
    }

    /// Return the name of the machine.
    #[must_use]
    pub const fn name(&self) -> &Text {
        &self.name
    }

    /// Return the PLC platform of the machine.
    #[must_use]
    pub const fn platform(&self) -> PlcPlatform {
        self.platform
    }

    /// Return the instant of archiving, or `None` for a live machine.
    #[must_use]
    pub const fn archived_at(&self) -> Option<Timestamp> {
        self.archived_at
    }

    /// Return true when the machine is not archived.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.archived_at.is_none()
    }

    /// Create a live machine with its main unit `U1` (`domain-model.md` section 5.1, `CreateMachine`).
    ///
    /// The caller supplies the identifiers. The new machine cannot fail.
    #[must_use]
    pub fn create(
        id: MachineId,
        code: MachineCode,
        name: Text,
        platform: PlcPlatform,
        main: (UnitId, Text),
    ) -> Self {
        let (main_id, main_name) = main;
        let path = UnitPath::first_main();
        let main_unit = Unit {
            id: main_id,
            path: path.clone(),
            name: main_name,
            archived_at: None,
        };
        Self {
            id,
            code,
            name,
            platform,
            archived_at: None,
            units: BTreeMap::from([(path, main_unit)]),
        }
    }

    /// Add a live unit (`domain-model.md` section 5.1, `AddUnit`).
    ///
    /// `segment` is one segment such as `2`. Without a `parent` the unit is a
    /// main unit. On an error the unit tree stays as it was.
    ///
    /// # Errors
    ///
    /// The checks run in this order.
    /// Returns [`PlantError::MachineArchived`] for an archived machine.
    /// Returns [`PlantError::InvalidUnitPath`] when `segment` is not one valid segment.
    /// Returns [`PlantError::UnitDepthExceeded`] for a sixth level.
    /// Returns [`PlantError::ParentNotFound`] with the new path when the parent is missing.
    /// Returns [`PlantError::UnitArchived`] with the parent path when the parent is archived.
    /// Returns [`PlantError::DuplicateUnitPath`] when the path or the identifier exists.
    pub fn add_unit(
        &mut self,
        id: UnitId,
        parent: Option<&UnitPath>,
        segment: &str,
        name: Text,
    ) -> Result<&Unit, PlantError> {
        self.ensure_live()?;
        let path = UnitPath::child(parent, segment)?;
        if let Some(parent_path) = parent {
            let parent_unit = self
                .units
                .get(parent_path)
                .ok_or_else(|| PlantError::ParentNotFound { path: path.clone() })?;
            if !parent_unit.is_live() {
                return Err(PlantError::UnitArchived {
                    path: parent_path.clone(),
                });
            }
        }
        // A known identifier is corrupt input of the caller. It reports the code of
        // a repeated path, like `TryFrom<StoredMachine>`.
        let id_known = self.units.values().any(|unit| unit.id == id);
        match self.units.entry(path.clone()) {
            Entry::Occupied(_) => Err(PlantError::DuplicateUnitPath { path }),
            Entry::Vacant(_) if id_known => Err(PlantError::DuplicateUnitPath { path }),
            Entry::Vacant(slot) => Ok(&*slot.insert(Unit {
                id,
                path,
                name,
                archived_at: None,
            })),
        }
    }

    /// Return the unit with `path`, live or archived, or `None`.
    ///
    /// A use case maps `None` for a URL path to `E-WEB-005` (`api.md` section 1).
    #[must_use]
    pub fn unit(&self, path: &UnitPath) -> Option<&Unit> {
        self.units.get(path)
    }

    /// Rename the machine (`domain-model.md` section 5.1, `RenameMachine`).
    ///
    /// # Errors
    ///
    /// Returns [`PlantError::MachineArchived`] for an archived machine.
    pub fn rename(&mut self, name: Text) -> Result<(), PlantError> {
        self.ensure_live()?;
        self.name = name;
        Ok(())
    }

    /// Rename a live unit (`domain-model.md` section 5.1, `RenameUnit`).
    ///
    /// # Errors
    ///
    /// Returns [`PlantError::UnitNotFound`] when no unit has `path`.
    /// The use case checks [`Machine::unit`] first, so this check is a guard.
    /// Returns [`PlantError::UnitArchived`] for an archived unit.
    pub fn rename_unit(&mut self, path: &UnitPath, name: Text) -> Result<(), PlantError> {
        let unit = self
            .units
            .get_mut(path)
            .ok_or_else(|| PlantError::UnitNotFound { path: path.clone() })?;
        if !unit.is_live() {
            return Err(PlantError::UnitArchived { path: path.clone() });
        }
        unit.name = name;
        Ok(())
    }

    /// Archive the machine (`domain-model.md` section 5.1, `ArchiveMachine`).
    ///
    /// `live_draft` is the reference number of a recipe of this machine with a
    /// live draft, or `None`. The units keep their state.
    ///
    /// # Errors
    ///
    /// Returns [`PlantError::MachineArchived`] when the machine is already archived.
    /// A route resolves only live machines, so this check is a guard.
    /// Returns [`PlantError::MachineHasDraft`] when `live_draft` is `Some`.
    pub fn archive(
        &mut self,
        now: Timestamp,
        live_draft: Option<&ReferenceNumber>,
    ) -> Result<(), PlantError> {
        self.ensure_live()?;
        if let Some(reference) = live_draft {
            return Err(PlantError::MachineHasDraft {
                code: self.code,
                reference: reference.clone(),
            });
        }
        self.archived_at = Some(now);
        Ok(())
    }

    /// Archive a unit and its live descendants (`domain-model.md` section 5.1,
    /// `ArchiveUnit`, ADR-0021).
    ///
    /// The result holds the identifiers of the units that this call archived,
    /// in ascending order. This is the lock order of `data-model.md` section 6.
    /// A unit that was already archived keeps its time and is not in the result.
    /// When the unit and all its descendants are already archived, the result is
    /// empty and nothing changes. The use case decides what to do with an empty
    /// result, because the audit commit needs one entry (P2.3).
    ///
    /// # Errors
    ///
    /// The checks run in this order. Only the live units of the set count.
    /// Returns [`PlantError::UnitNotFound`] when no unit has `path`.
    /// Returns [`PlantError::UnitReferencedByDraft`] and then
    /// [`PlantError::UnitReferencedByApproved`] with the path of the unit in use.
    /// The lowest path comes first.
    /// Returns [`PlantError::LastMainUnit`] when `path` is the last live main unit.
    pub fn archive_unit(
        &mut self,
        path: &UnitPath,
        now: Timestamp,
        usage: &UnitUsage,
    ) -> Result<Vec<UnitId>, PlantError> {
        if !self.units.contains_key(path) {
            return Err(PlantError::UnitNotFound { path: path.clone() });
        }
        let in_set =
            |unit: &Unit| unit.is_live() && (unit.path == *path || path.is_ancestor_of(&unit.path));
        let set: Vec<&Unit> = self.units.values().filter(|unit| in_set(unit)).collect();
        for unit in &set {
            if let Some(reference) = usage.draft_of(unit.id) {
                return Err(PlantError::UnitReferencedByDraft {
                    path: unit.path.clone(),
                    reference: reference.clone(),
                });
            }
        }
        for unit in &set {
            if let Some(reference) = usage.approved_of(unit.id) {
                return Err(PlantError::UnitReferencedByApproved {
                    path: unit.path.clone(),
                    reference: reference.clone(),
                });
            }
        }
        if path.depth() == 1 && !set.is_empty() && !self.has_other_live_main_unit(path) {
            return Err(PlantError::LastMainUnit {
                code: self.code,
                path: path.clone(),
            });
        }
        let mut ids = Vec::new();
        for unit in self.units.values_mut().filter(|unit| in_set(unit)) {
            unit.archived_at = Some(now);
            ids.push(unit.id);
        }
        ids.sort();
        Ok(ids)
    }

    /// Return true when a live unit of depth 1 other than `path` exists.
    fn has_other_live_main_unit(&self, path: &UnitPath) -> bool {
        self.units
            .values()
            .any(|unit| unit.path.depth() == 1 && unit.is_live() && unit.path != *path)
    }

    /// Return an error when the machine is archived.
    fn ensure_live(&self) -> Result<(), PlantError> {
        if self.is_live() {
            Ok(())
        } else {
            Err(PlantError::MachineArchived { code: self.code })
        }
    }

    /// Read the machine and the state of every unit as a [`LiveUnits`] value.
    #[must_use]
    pub fn live_units(&self) -> LiveUnits {
        LiveUnits::of(self)
    }

    /// Return the units in path order. A parent comes before its children.
    pub fn units(&self) -> impl Iterator<Item = &Unit> {
        self.units.values()
    }
}

impl TryFrom<StoredMachine> for Machine {
    type Error = PlantError;

    /// Check the unit rows that storage loaded and build the machine.
    ///
    /// A failure means corrupt storage, because the commands keep the tree valid.
    /// The checks run in this order: duplicate path, missing parent,
    /// archived parent of a live unit, and no live main unit.
    fn try_from(stored: StoredMachine) -> Result<Self, PlantError> {
        let units = index_units(stored.units)?;
        check_parents(&units)?;
        check_live_main_unit(&units, stored.code)?;
        Ok(Self {
            id: stored.id,
            code: stored.code,
            name: stored.name,
            platform: stored.platform,
            archived_at: stored.archived_at,
            units,
        })
    }
}

/// Index the stored units by path. A second unit with a known path or a known id fails.
fn index_units(stored: Vec<StoredUnit>) -> Result<BTreeMap<UnitPath, Unit>, PlantError> {
    let mut ids = BTreeSet::new();
    let mut units = BTreeMap::new();
    for row in stored {
        // A repeated identifier is also corrupt storage. It reports the same code
        // as a repeated path, because P2.4 maps every failure to a storage error.
        let is_new_id = ids.insert(row.id);
        match units.entry(row.path.clone()) {
            Entry::Occupied(_) => return Err(PlantError::DuplicateUnitPath { path: row.path }),
            Entry::Vacant(_) if !is_new_id => {
                return Err(PlantError::DuplicateUnitPath { path: row.path });
            }
            Entry::Vacant(slot) => {
                slot.insert(Unit {
                    id: row.id,
                    path: row.path,
                    name: row.name,
                    archived_at: row.archived_at,
                });
            }
        }
    }
    Ok(units)
}

/// Check that each unit has its parent (I10) and that a live unit has a live parent.
fn check_parents(units: &BTreeMap<UnitPath, Unit>) -> Result<(), PlantError> {
    let parent_of = |unit: &Unit| unit.path.parent().map(|path| (units.get(&path), path));
    for unit in units.values() {
        if let Some((None, _)) = parent_of(unit) {
            return Err(PlantError::ParentNotFound {
                path: unit.path.clone(),
            });
        }
    }
    for unit in units.values().filter(|unit| unit.is_live()) {
        if let Some((Some(parent), path)) = parent_of(unit)
            && !parent.is_live()
        {
            return Err(PlantError::UnitArchived { path });
        }
    }
    Ok(())
}

/// Check that the machine has a live unit of depth 1.
fn check_live_main_unit(
    units: &BTreeMap<UnitPath, Unit>,
    code: MachineCode,
) -> Result<(), PlantError> {
    if units
        .values()
        .any(|unit| unit.path.depth() == 1 && unit.is_live())
    {
        return Ok(());
    }
    Err(PlantError::LastMainUnit {
        code,
        path: UnitPath::first_main(),
    })
}

#[cfg(test)]
mod add_unit_tests;
#[cfg(test)]
mod archive_unit_tests;
#[cfg(test)]
mod edit_tests;
#[cfg(test)]
mod tests;
