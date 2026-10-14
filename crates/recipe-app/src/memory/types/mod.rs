//! In-memory `TypeCatalog` (task 4.7).

mod seed;

use async_trait::async_trait;
use recipe_domain::plant::{MachineId, PlcType, PlcTypeRecord, Primitive, RecordShape};

use super::images::type_image;
use super::rows::{TypeRow, check_violation, duplicate, missing_reference};
use super::{LockMode, MemoryTransaction};
use crate::error::{PLC_TYPE_NAMED_UNIQUE, StorageError};
use crate::ports::{DerivedTypeLock, Ensured, RowImage, TypeCatalog};

pub use seed::seed_type_id;

/// The journal table of the class-2 advisory locks of derived types (`data-model.md` section 1.4).
pub const DERIVED_TYPE_LOCK_TABLE: &str = "advisory_class_2";

/// The name of the primary key of `plc_type`.
const TYPE_PKEY: &str = "plc_type_pkey";
/// The name of the foreign key from `plc_type` to `machine`.
const TYPE_MACHINE_FKEY: &str = "plc_type_machine_id_fkey";
/// The name of the foreign key from an array row to its element row.
const TYPE_ELEMENT_FKEY: &str = "plc_type_element_type_id_fkey";
/// The name of the foreign key from a struct row to the row that it supersedes.
const TYPE_SUPERSEDES_FKEY: &str = "plc_type_supersedes_type_id_fkey";
/// The name of the foreign key from a member row to its type row.
const MEMBER_TYPE_FKEY: &str = "plc_type_member_member_type_id_fkey";

impl MemoryTransaction {
    /// Check that the machine and the referenced types of a new row exist.
    fn check_type_references(&self, row: &PlcType) -> Result<(), StorageError> {
        let state = self.state();
        let machine_exists = row
            .machine()
            .is_none_or(|machine| state.machines.contains_key(&machine));
        if !machine_exists {
            return Err(missing_reference(TYPE_MACHINE_FKEY));
        }
        match row.shape() {
            RecordShape::Primitive { .. } => {}
            RecordShape::Array { element, .. } => {
                if !state.types.contains_key(element) {
                    return Err(missing_reference(TYPE_ELEMENT_FKEY));
                }
            }
            RecordShape::Struct {
                supersedes,
                members,
                ..
            } => {
                if supersedes.is_some_and(|id| !state.types.contains_key(&id)) {
                    return Err(missing_reference(TYPE_SUPERSEDES_FKEY));
                }
                if members
                    .iter()
                    .any(|member| !state.types.contains_key(&member.type_id))
                {
                    return Err(missing_reference(MEMBER_TYPE_FKEY));
                }
            }
        }
        Ok(())
    }

    /// Add a checked type row and return its image.
    fn put_type(&mut self, row: PlcType) -> RowImage {
        let row = TypeRow {
            plc_type: row,
            created_at: self.now(),
        };
        let image = type_image(&row);
        self.state_mut().types.insert(row.plc_type.id(), row);
        image
    }
}

#[async_trait]
impl TypeCatalog for MemoryTransaction {
    async fn type_rows(&mut self, machine: MachineId) -> Result<Vec<PlcType>, StorageError> {
        Ok(self
            .state()
            .types
            .values()
            .filter(|row| row.plc_type.machine().is_none_or(|owner| owner == machine))
            .map(|row| row.plc_type.clone())
            .collect())
    }

    async fn lock_derived_types(
        &mut self,
        machine: MachineId,
        keys: &[DerivedTypeLock],
    ) -> Result<(), StorageError> {
        for key in keys {
            let text = format!("{machine}:{}", key.key_text());
            self.lock(DERIVED_TYPE_LOCK_TABLE, LockMode::Advisory, &text);
        }
        Ok(())
    }

    async fn ensure_derived(&mut self, record: PlcTypeRecord) -> Result<Ensured, StorageError> {
        let candidate = PlcType::try_from(record).map_err(|_| check_violation())?;
        // The port accepts a `STRING(n)`, a `WSTRING(n)`, and an array row only.
        let is_derived = match candidate.shape() {
            RecordShape::Primitive { primitive, .. } => {
                matches!(primitive, Primitive::String | Primitive::Wstring)
            }
            RecordShape::Array { .. } => true,
            RecordShape::Struct { .. } => false,
        };
        if !is_derived {
            return Err(check_violation());
        }
        let existing = self
            .state()
            .types
            .values()
            .map(|row| &row.plc_type)
            .find(|row| row.machine() == candidate.machine() && row.shape() == candidate.shape());
        if let Some(existing) = existing {
            return Ok(Ensured {
                id: existing.id(),
                created: None,
            });
        }
        if self.state().types.contains_key(&candidate.id()) {
            return Err(duplicate(TYPE_PKEY));
        }
        self.check_type_references(&candidate)?;
        let id = candidate.id();
        let image = self.put_type(candidate);
        Ok(Ensured {
            id,
            created: Some(image),
        })
    }

    async fn insert_struct(&mut self, row: &PlcType) -> Result<RowImage, StorageError> {
        if let Some(fault) = self.insert_struct_fault() {
            return Err(fault.into());
        }
        let RecordShape::Struct { name, revision, .. } = row.shape() else {
            return Err(check_violation());
        };
        let name_taken = self.state().types.values().any(|stored| {
            matches!(
                stored.plc_type.shape(),
                RecordShape::Struct { name: other, revision: other_revision, .. }
                    if other == name && other_revision == revision
            ) && stored.plc_type.machine() == row.machine()
        });
        if name_taken {
            return Err(duplicate(PLC_TYPE_NAMED_UNIQUE));
        }
        if self.state().types.contains_key(&row.id()) {
            return Err(duplicate(TYPE_PKEY));
        }
        self.check_type_references(row)?;
        Ok(self.put_type(row.clone()))
    }
}

#[cfg(test)]
mod tests;
