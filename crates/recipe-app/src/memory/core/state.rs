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
use recipe_domain::setting::{SettingId, SettingKey, SettingValue};
use recipe_domain::timestamp::Timestamp;
use uuid::Uuid;

use crate::error::StorageError;
use crate::memory::audit_chain::ChainRow;
use crate::memory::identity::StoredSession;
use crate::memory::recipe::{ParameterRow, RecipeRow, VersionRow};
use crate::memory::rows::{MachineRow, TypeRow, UnitRow};
use crate::ports::{AttemptId, AttemptRow, ChainPosition, UserRow};

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
    /// The rows of the table `app_setting`, in the byte order of the keys.
    pub(crate) settings: BTreeMap<SettingKey, StoredSetting>,
    /// The number of the last identifier that an insert into `app_setting` took.
    pub(crate) last_setting_serial: u128,
    /// The rows of the table `audit_log` that the audit chain check reads, in insertion order.
    pub(crate) chain: Vec<ChainRow>,
    /// The head that a test plants into `audit_chain_head`. `None` means the last row of `chain`.
    pub(crate) chain_head: Option<ChainPosition>,
    /// The number of `security_event` rows that `security_event_purge()` can still delete.
    pub(crate) expired_security_events: u64,
}

/// The columns of an `app_setting` row except the key.
///
/// The type solves one problem: the in-memory table needs the columns that the row image shows,
/// and the key is the key of the map. `updated_by` is always `NULL`, because both writers have
/// no actor. The writers are the startup settings comparison and the memory `MaintenanceStore`
/// of task 3.10.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredSetting {
    /// The primary key.
    pub(crate) id: SettingId,
    /// The stored text of the value.
    pub(crate) value: SettingValue,
    /// The time of the insert.
    pub(crate) created_at: Timestamp,
    /// The time of the last change.
    pub(crate) updated_at: Timestamp,
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

    /// Take the next identifier of an `app_setting` row.
    ///
    /// The method solves one problem: the identifier rule must live in one place. The
    /// `SettingRepository` and `upsert_setting` both call it.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` when the identifier serial overflows. The serial stays unchanged.
    pub(crate) fn next_setting_id(&mut self) -> Result<SettingId, StorageError> {
        let serial = self
            .last_setting_serial
            .checked_add(1)
            .ok_or_else(|| StorageError::invariant("the setting serial overflows"))?;
        self.last_setting_serial = serial;
        Ok(SettingId::from(Uuid::from_u128(serial)))
    }

    /// Insert the `app_setting` row of `key`, or change its value and its `updated_at`.
    ///
    /// The method solves one problem: the upsert rule must live in one place. A change keeps the
    /// identifier and `created_at`. `updated_by` stays `NULL`. The method returns the row that
    /// it stored.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` when a new row needs an identifier and the serial overflows.
    /// The table stays unchanged.
    pub(crate) fn put_setting(
        &mut self,
        key: &SettingKey,
        value: &SettingValue,
        now: Timestamp,
    ) -> Result<StoredSetting, StorageError> {
        let existing = self.settings.get(key).map(|row| (row.id, row.created_at));
        let (id, created_at) = match existing {
            Some(found) => found,
            None => (self.next_setting_id()?, now),
        };
        let row = StoredSetting {
            id,
            value: value.clone(),
            created_at,
            updated_at: now,
        };
        self.settings.insert(key.clone(), row.clone());
        Ok(row)
    }

    /// Insert the `app_setting` row of `key`, or change its value and its `updated_at`.
    ///
    /// The method solves one problem: the memory `MaintenanceStore` writes a state key with an
    /// upsert, like `SettingRepository`, and `StoredSetting` is not visible outside this module.
    /// It applies the rule of `put_setting`.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` when the identifier serial overflows.
    pub(crate) fn upsert_setting(
        &mut self,
        key: SettingKey,
        value: SettingValue,
        now: Timestamp,
    ) -> Result<(), StorageError> {
        self.put_setting(&key, &value, now).map(|_| ())
    }
}
