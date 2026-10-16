//! Preview of a type expression without a write (design D4, ADR-0039 (h)).
//!
//! The form of a parameter shows the leaf rows of a `TypeExpr` before the user saves. The
//! expression can name a derived row that the catalog does not hold yet. `EnsureDerivedType`
//! creates such a row, but a query writes nothing (`reliability.md` section 7.4). The preview
//! resolves the expression over the type rows of the machine and adds each missing derived row
//! in memory only. Its rules copy `plant/ensure_derived.rs`, so the leaves of the preview equal
//! the leaves of the tree after `EnsureDerivedType`.

use std::collections::BTreeSet;

use recipe_domain::plant::{
    ArrayDimensions, BaseExpr, DerivedTypeReason, MachineId, PlantError, PlatformTypes,
    PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, StringKind,
    StringLength, TypeExpr, TypeName, TypeTree,
};
use uuid::Uuid;

use crate::error::{AppError, StorageError};

/// The base of each identifier of a preview row.
///
/// A preview row lives only in the memory of one query. Its identifier never reaches the
/// database, so the base only has to stay apart from the identifiers of the rows that it joins.
const PREVIEW_ID_BASE: u128 = 0xFFFF_FFFF_FFFF_FFFF_0000_0000_0000_0000;

/// The type rows of one machine with the rows that the preview added in memory.
///
/// The type solves one problem: the preview must resolve a `TypeExpr` with the rules of
/// `EnsureDerivedType` and must not call the catalog. Each missing derived row gets a preview
/// identifier and joins the list. No row leaves this value.
struct PreviewTypes {
    machine: MachineId,
    platform: PlcPlatform,
    rows: Vec<PlcType>,
    added: u64,
}

impl PreviewTypes {
    /// Resolve a type expression to a row. The innermost element comes first.
    fn resolve(&mut self, expr: &TypeExpr) -> Result<PlcTypeId, AppError> {
        let base = self.base_id(expr)?;
        self.wrap_arrays(base, expr.arrays())
    }

    /// Wrap `element` in the array levels, the innermost level first.
    fn wrap_arrays(
        &mut self,
        element: PlcTypeId,
        levels: &[ArrayDimensions],
    ) -> Result<PlcTypeId, AppError> {
        let mut current = element;
        for dimensions in levels.iter().rev() {
            current = self.ensure_row(RecordShape::Array {
                element: current,
                dimensions: dimensions.clone(),
            })?;
        }
        Ok(current)
    }

    /// Resolve the base of an expression: a primitive, a string, or a struct.
    fn base_id(&mut self, expr: &TypeExpr) -> Result<PlcTypeId, AppError> {
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
                self.ensure_row(RecordShape::Primitive {
                    primitive,
                    leaf_class: primitive.leaf_class(),
                    size_bytes,
                    string_length: Some(parsed.value()),
                })
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

    /// Return the row of the machine with `shape`, and add it in memory when it does not exist.
    fn ensure_row(&mut self, shape: RecordShape) -> Result<PlcTypeId, AppError> {
        let existing = self
            .rows
            .iter()
            .find(|row| row.machine() == Some(self.machine) && *row.shape() == shape);
        if let Some(existing) = existing {
            return Ok(existing.id());
        }
        let id = self.next_id()?;
        let row = PlcType::try_from(PlcTypeRecord {
            id,
            machine: Some(self.machine),
            platforms: BTreeSet::from([self.platform]),
            shape,
        })?;
        self.rows.push(row);
        Ok(id)
    }

    /// Return the next preview identifier.
    fn next_id(&mut self) -> Result<PlcTypeId, AppError> {
        let no_next = || StorageError::invariant("the preview has no next identifier");
        self.added = self.added.checked_add(1).ok_or_else(no_next)?;
        let value = PREVIEW_ID_BASE
            .checked_add(u128::from(self.added))
            .ok_or_else(no_next)?;
        Ok(PlcTypeId::from(Uuid::from_u128(value)))
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

/// Resolve the tree of `expr` over `rows` without a write.
///
/// `rows` are the rows of `TypeCatalog::type_rows` for `machine`. The function adds each missing
/// derived row in memory, then resolves the tree of the root row.
///
/// # Errors
///
/// Returns `E-PLT-013` for a type that is not on the platform, a length out of range, or an
/// unknown struct revision. Returns `E-PLT-018` when the tree is above a size limit.
pub(super) fn preview_tree(
    machine: MachineId,
    platform: PlcPlatform,
    rows: Vec<PlcType>,
    expr: &TypeExpr,
) -> Result<TypeTree, AppError> {
    let mut types = PreviewTypes {
        machine,
        platform,
        rows,
        added: 0,
    };
    let root = types.resolve(expr)?;
    let platform_types = PlatformTypes::collect(machine, platform, types.rows)?;
    Ok(TypeTree::resolve(root, &platform_types)?)
}
