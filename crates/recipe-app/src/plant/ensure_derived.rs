//! The internal operation `EnsureDerivedType` (`domain-model.md` section 5.1, design D11).
//!
//! The operation is not a use case and has no permission of its own. The calling use case
//! checks its permission first, opens the transaction, takes the locks of
//! [`derived_type_locks`](super::derived_type_locks), and then reads the type rows.

use std::collections::BTreeSet;

use recipe_domain::audit::{AuditContext, AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{
    ArrayDimensions, BaseExpr, DerivedTypeReason, MachineId, PlantError, PlatformTypes,
    PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, StringKind,
    StringLength, TypeExpr, TypeName, TypeTree,
};
use recipe_domain::recipe::ArrayShape;
use uuid::Uuid;

use crate::error::AppError;
use crate::ports::TypeCatalog;

/// The type rows of one machine inside one transaction, and the rows that `EnsureDerivedType` adds.
///
/// The type solves one problem: several callers resolve type expressions in one transaction,
/// and each caller needs the new rows to resolve a tree and the `DerivedTypeCreated` entries
/// to commit. The set holds the rows that the caller read and the rows that it created.
/// It takes no lock. The caller takes the keys of `derived_type_locks` before it reads the
/// rows and before the first call.
#[derive(Clone, Debug)]
pub(crate) struct DerivedTypes {
    machine: MachineId,
    platform: PlcPlatform,
    rows: Vec<PlcType>,
    context: AuditContext,
    entries: Vec<AuditEntry>,
}

impl DerivedTypes {
    /// Build the working set from the rows of `TypeCatalog::type_rows`.
    ///
    /// `platform` is the platform of the machine. `context` is the audit context of the
    /// calling command.
    #[must_use]
    pub(crate) const fn new(
        machine: MachineId,
        platform: PlcPlatform,
        rows: Vec<PlcType>,
        context: AuditContext,
    ) -> Self {
        Self {
            machine,
            platform,
            rows,
            context,
            entries: Vec::new(),
        }
    }

    /// Return the `DerivedTypeCreated` entries so far, in creation order.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Return the rows that the caller read and the rows that the set created.
    #[must_use]
    pub(crate) fn rows(&self) -> &[PlcType] {
        &self.rows
    }

    /// Return the rows of the machine on its platform, including the new rows.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when two rows share an identifier.
    pub(crate) fn platform_types(&self) -> Result<PlatformTypes, PlantError> {
        PlatformTypes::collect(self.machine, self.platform, self.rows.iter().cloned())
    }

    /// Give the rows and the entries to the caller.
    #[must_use]
    pub(crate) fn into_parts(self) -> (Vec<PlcType>, Vec<AuditEntry>) {
        (self.rows, self.entries)
    }

    /// Resolve a type expression to a type row. The innermost element comes first.
    ///
    /// A primitive is the global row. A `STRING` or `WSTRING` without length is the seed row of
    /// the platform. A sized string and each array level are the row of the machine, and a
    /// missing row is created and gets one `DerivedTypeCreated` entry. A struct is the row of
    /// the machine with that name and revision.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` for a type that is not on the platform, a length out of range, or an
    /// unknown struct revision. Returns `E-PLT-018` when the resolved tree is above a size limit.
    /// Returns `AppError::Storage` when the catalog fails.
    pub(crate) async fn ensure_expr(
        &mut self,
        catalog: &mut dyn TypeCatalog,
        expr: &TypeExpr,
    ) -> Result<PlcTypeId, AppError> {
        let base = self.base_id(catalog, expr).await?;
        self.wrap_arrays(catalog, base, expr.arrays()).await
    }

    /// Resolve the array shape of a rebind to a type row.
    ///
    /// The shape names its element type and its nested dimensions. The call creates one array
    /// row for each level, the innermost level first, exactly as the equivalent expression does.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the element is not on the platform and `E-PLT-018` when the
    /// resolved tree is above a size limit. Returns `AppError::Storage` when the catalog fails.
    pub(crate) async fn ensure_shape(
        &mut self,
        catalog: &mut dyn TypeCatalog,
        shape: &ArrayShape,
    ) -> Result<PlcTypeId, AppError> {
        self.wrap_arrays(catalog, shape.element(), shape.dimensions())
            .await
    }

    /// Wrap `element` in the array levels, the innermost level first, and check the final tree.
    async fn wrap_arrays(
        &mut self,
        catalog: &mut dyn TypeCatalog,
        element: PlcTypeId,
        levels: &[ArrayDimensions],
    ) -> Result<PlcTypeId, AppError> {
        let mut current = element;
        for dimensions in levels.iter().rev() {
            current = self
                .ensure_row(
                    catalog,
                    RecordShape::Array {
                        element: current,
                        dimensions: dimensions.clone(),
                    },
                )
                .await?;
        }
        if !levels.is_empty() {
            TypeTree::resolve(current, &self.platform_types()?)?;
        }
        Ok(current)
    }

    /// Resolve the base of an expression: a primitive, a string, or a struct.
    async fn base_id(
        &mut self,
        catalog: &mut dyn TypeCatalog,
        expr: &TypeExpr,
    ) -> Result<PlcTypeId, AppError> {
        match expr.base() {
            BaseExpr::Primitive(primitive) => self
                .global_id(*primitive, None)
                .ok_or_else(|| invalid(expr, DerivedTypeReason::NotOnPlatform)),
            BaseExpr::String { kind, length: None } => {
                StringLength::default_length(*kind, self.platform)
                    .and_then(|length| self.global_id(string_primitive(*kind), Some(length)))
                    .ok_or_else(|| invalid(expr, DerivedTypeReason::NotOnPlatform))
            }
            BaseExpr::String {
                kind,
                length: Some(length),
            } => {
                let parsed = StringLength::parse(*kind, self.platform, u32::from(*length))
                    .map_err(|reason| invalid(expr, reason.into()))?;
                let size_bytes = parsed
                    .size_bytes(*kind)
                    .ok_or_else(|| invalid(expr, DerivedTypeReason::RowShape))?;
                let primitive = string_primitive(*kind);
                self.ensure_row(
                    catalog,
                    RecordShape::Primitive {
                        primitive,
                        leaf_class: primitive.leaf_class(),
                        size_bytes,
                        string_length: Some(parsed.value()),
                    },
                )
                .await
            }
            BaseExpr::Udt { name, revision } => self
                .struct_id(name, *revision)
                .ok_or_else(|| invalid(expr, DerivedTypeReason::UnknownStruct)),
        }
    }

    /// Find the global primitive row on the platform of the machine.
    fn global_id(&self, primitive: Primitive, string_length: Option<u16>) -> Option<PlcTypeId> {
        self.rows
            .iter()
            .find(|row| {
                row.machine().is_none()
                    && row.platforms().contains(&self.platform)
                    && matches!(
                        row.shape(),
                        RecordShape::Primitive { primitive: found, string_length: length, .. }
                            if *found == primitive && *length == string_length
                    )
            })
            .map(PlcType::id)
    }

    /// Find the struct row of the machine with `name` and `revision`.
    fn struct_id(&self, name: &TypeName, revision: u32) -> Option<PlcTypeId> {
        self.rows
            .iter()
            .find(|row| {
                row.machine() == Some(self.machine)
                    && row.platforms().contains(&self.platform)
                    && matches!(
                        row.shape(),
                        RecordShape::Struct { name: found, revision: number, .. }
                            if found == name && *number == revision
                    )
            })
            .map(PlcType::id)
    }

    /// Return the row of the machine with `shape`, and create it when it does not exist.
    ///
    /// `PlcType::try_from` checks the record before the insert. A new row has only the platform
    /// of its machine and gives one `DerivedTypeCreated` entry.
    async fn ensure_row(
        &mut self,
        catalog: &mut dyn TypeCatalog,
        shape: RecordShape,
    ) -> Result<PlcTypeId, AppError> {
        let record = PlcTypeRecord {
            id: PlcTypeId::from(Uuid::new_v4()),
            machine: Some(self.machine),
            platforms: BTreeSet::from([self.platform]),
            shape,
        };
        let candidate = PlcType::try_from(record.clone())?;
        let existing = self
            .rows
            .iter()
            .find(|row| row.machine() == candidate.machine() && row.shape() == candidate.shape());
        if let Some(existing) = existing {
            return Ok(existing.id());
        }
        let ensured = catalog.ensure_derived(record).await?;
        if let Some(image) = ensured.created {
            self.entries.push(AuditEntry::row(
                RowAction::DerivedTypeCreated,
                AuditTarget::plc_type(self.machine, ensured.id),
                RowChange::Create {
                    after: image.into_value(),
                },
                self.context.clone(),
            ));
            self.rows.push(candidate);
        }
        Ok(ensured.id)
    }
}

/// Return the primitive of a string kind.
const fn string_primitive(kind: StringKind) -> Primitive {
    match kind {
        StringKind::String => Primitive::String,
        StringKind::Wstring => Primitive::Wstring,
    }
}

/// Build `E-PLT-013` for an expression.
fn invalid(expr: &TypeExpr, reason: DerivedTypeReason) -> AppError {
    PlantError::DerivedTypeInvalid {
        type_expr: expr.to_string(),
        reason,
    }
    .into()
}

#[cfg(test)]
mod tests;
