//! Data of the in-memory store.

use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;
use std::collections::HashMap;

use recipe_domain::identity::{SessionId, UserId};
use recipe_domain::plant::{
    Machine, MachineId, PlantError, PlcType, PlcTypeId, StoredMachine, StoredUnit, UnitId,
};
use recipe_domain::recipe::{ParameterId, RecipeId, RecipeVersionId};
use recipe_domain::timestamp::Timestamp;

use crate::memory::identity::StoredSession;
use crate::memory::recipe::{ParameterRow, RecipeRow, VersionRow};
use crate::memory::rows::{MachineRow, TypeRow, UnitRow};
use crate::ports::{AttemptId, AttemptRow, UserRow};

/// All rows of the in-memory store.
///
/// The struct solves one problem: a transaction needs one value that it copies at
/// `begin` and swaps in at `finish`. A parameter row holds its limit rows and
/// translation rows.
#[derive(Clone, Debug, Default)]
pub struct MemoryState {
    /// Markers that the tests of this module write to prove commit and rollback.
    #[cfg(test)]
    pub(super) markers: BTreeSet<String>,
    /// The rows of the table `machine`.
    pub(crate) machines: BTreeMap<MachineId, MachineRow>,
    /// The rows of the table `unit`.
    pub(crate) units: BTreeMap<UnitId, UnitRow>,
    /// The rows of the table `plc_type` with their member rows.
    pub(crate) types: BTreeMap<PlcTypeId, TypeRow>,
    /// The rows of the table `recipe`.
    pub(crate) recipes: BTreeMap<RecipeId, RecipeRow>,
    /// The rows of the table `recipe_version`.
    pub(crate) versions: BTreeMap<RecipeVersionId, VersionRow>,
    /// The rows of the table `draft_editor`, with the value of `first_edit_at`.
    pub(crate) editors: BTreeMap<(RecipeVersionId, UserId), Timestamp>,
    /// The rows of the table `parameter` with their limit rows and translation rows.
    pub(crate) parameters: BTreeMap<ParameterId, ParameterRow>,
    /// The rows of the table `app_user`.
    pub(crate) users: BTreeMap<UserId, UserRow>,
    /// The rows of the table `session`.
    pub(crate) sessions: HashMap<SessionId, StoredSession>,
    /// The rows of the table `login_attempt`.
    pub(crate) attempts: BTreeMap<AttemptId, AttemptRow>,
    /// The last key that the identity column of `login_attempt` gave.
    pub(crate) last_attempt_key: i64,
}

impl MemoryState {
    /// Build the machine aggregate from its rows, or `None` when the machine does not exist.
    ///
    /// A machine is live or archived. A failure means that the rows break a domain rule.
    pub(crate) fn try_machine(&self, id: MachineId) -> Result<Option<Machine>, PlantError> {
        let Some(row) = self.machines.get(&id) else {
            return Ok(None);
        };
        let units = self
            .units
            .values()
            .filter(|unit| unit.machine == id)
            .map(|unit| StoredUnit {
                id: unit.id,
                path: unit.path.clone(),
                name: unit.name.clone(),
                archived_at: unit.archived_at,
            })
            .collect();
        Machine::try_from(StoredMachine {
            id: row.id,
            code: row.code,
            name: row.name.clone(),
            platform: row.platform,
            archived_at: row.archived_at,
            units,
        })
        .map(Some)
    }

    /// Return the machine with `id`, live or archived. Corrupt rows give `None`.
    #[must_use]
    pub fn machine(&self, id: MachineId) -> Option<Machine> {
        self.try_machine(id).ok().flatten()
    }

    /// Return the type row with `id`.
    #[must_use]
    pub fn plc_type(&self, id: PlcTypeId) -> Option<&PlcType> {
        self.types.get(&id).map(|row| &row.plc_type)
    }
}
