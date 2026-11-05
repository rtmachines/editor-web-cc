//! PLC type rows of the Plant Model (`domain-model.md` section 2.1,
//! `data-model.md` section 3.3).

use std::collections::BTreeSet;

use super::limits::MAX_MEMBERS;
use super::{
    ArrayDimensions, DerivedTypeReason, LeafClass, MachineId, MemberName, PlantError, PlcPlatform,
    PlcTypeId, Primitive, StringKind, StringLength, TypeLimit, TypeName,
};

/// Kind of a type row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypeKind {
    /// A primitive type, such as `INT` or `STRING(20)`.
    Primitive,
    /// An array type.
    Array,
    /// A struct type (a UDT revision).
    Struct,
}

/// A member of a struct as storage or `DefineUdt` builds it.
///
/// A member alone has no invariant. The struct that holds it checks the
/// count, the names, and the self reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeMember {
    /// The name of the member.
    pub name: MemberName,
    /// The type of the member.
    pub type_id: PlcTypeId,
}

/// The shape of a type row as storage or a use case builds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordShape {
    /// A primitive row.
    Primitive {
        /// The primitive name.
        primitive: Primitive,
        /// The class of the leaf.
        leaf_class: LeafClass,
        /// The S7 size of one element in bytes.
        size_bytes: u32,
        /// The length of a `STRING` or a `WSTRING`.
        string_length: Option<u16>,
    },
    /// An array row.
    Array {
        /// The element type.
        element: PlcTypeId,
        /// The dimensions.
        dimensions: ArrayDimensions,
    },
    /// A struct row.
    Struct {
        /// The name of the struct.
        name: TypeName,
        /// The revision of the struct, from 1.
        revision: u32,
        /// The revision that this revision replaces.
        supersedes: Option<PlcTypeId>,
        /// The members in their order.
        members: Vec<TypeMember>,
    },
}

/// A type row as storage or a use case builds it.
///
/// `TryFrom<PlcTypeRecord>` checks it and builds a [`PlcType`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlcTypeRecord {
    /// The identifier of the type.
    pub id: PlcTypeId,
    /// The machine of the type, or `None` for the global catalog.
    pub machine: Option<MachineId>,
    /// The platforms where the type exists.
    pub platforms: BTreeSet<PlcPlatform>,
    /// The shape of the row.
    pub shape: RecordShape,
}

/// A valid type row.
///
/// The fields are private and the constructor checks the shape rules, so
/// the public API cannot build or change an invalid row (I22).
///
/// This example shows that no field can change after construction:
///
/// ```compile_fail,E0616
/// use std::collections::BTreeSet;
/// use recipe_domain::plant::{
///     LeafClass, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape,
/// };
///
/// let record = PlcTypeRecord {
///     id: PlcTypeId::from(uuid::Uuid::nil()),
///     machine: None,
///     platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
///     shape: RecordShape::Primitive {
///         primitive: Primitive::Int,
///         leaf_class: LeafClass::Numeric,
///         size_bytes: 2,
///         string_length: None,
///     },
/// };
/// let mut plc_type = PlcType::try_from(record).unwrap();
/// plc_type.platforms = BTreeSet::new();
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlcType {
    id: PlcTypeId,
    machine: Option<MachineId>,
    platforms: BTreeSet<PlcPlatform>,
    shape: RecordShape,
}

impl TryFrom<PlcTypeRecord> for PlcType {
    type Error = PlantError;

    fn try_from(record: PlcTypeRecord) -> Result<Self, Self::Error> {
        match &record.shape {
            RecordShape::Primitive {
                primitive,
                leaf_class,
                size_bytes,
                string_length,
            } => check_primitive(
                &record,
                *primitive,
                *leaf_class,
                *size_bytes,
                *string_length,
            )?,
            RecordShape::Array {
                element,
                dimensions,
            } => check_array(&record, *element, dimensions)?,
            RecordShape::Struct { .. } => check_struct(&record)?,
        }
        Ok(Self {
            id: record.id,
            machine: record.machine,
            platforms: record.platforms,
            shape: record.shape,
        })
    }
}

/// Build the error of a row that breaks a rule of `data-model.md` section 3.3.
fn invalid(type_expr: String, reason: DerivedTypeReason) -> PlantError {
    PlantError::DerivedTypeInvalid { type_expr, reason }
}

/// Return the text of a primitive row, such as `INT` or `STRING(20)`.
fn primitive_expr(primitive: Primitive, string_length: Option<u16>) -> String {
    match string_length {
        Some(length) => format!("{}({length})", primitive.name()),
        None => primitive.name().to_owned(),
    }
}

/// Return the string kind of a primitive, or `None` for a primitive without length.
const fn string_kind(primitive: Primitive) -> Option<StringKind> {
    match primitive {
        Primitive::String => Some(StringKind::String),
        Primitive::Wstring => Some(StringKind::Wstring),
        _ => None,
    }
}

/// Check the rules of a primitive row (`plc_type_primitive_shape` and the string rules).
fn check_primitive(
    record: &PlcTypeRecord,
    primitive: Primitive,
    leaf_class: LeafClass,
    size_bytes: u32,
    string_length: Option<u16>,
) -> Result<(), PlantError> {
    let fail = |reason| invalid(primitive_expr(primitive, string_length), reason);
    if record.platforms.is_empty() || leaf_class != primitive.leaf_class() {
        return Err(fail(DerivedTypeReason::RowShape));
    }
    let expected_size = match (string_kind(primitive), string_length) {
        (Some(kind), Some(length)) => {
            let mut checked = None;
            for platform in &record.platforms {
                let parsed = StringLength::parse(kind, *platform, u32::from(length))
                    .map_err(|reason| fail(reason.into()))?;
                checked = Some(parsed);
            }
            checked.and_then(|parsed| parsed.size_bytes(kind))
        }
        (None, None) => primitive.fixed_size_bytes(),
        _ => None,
    };
    if expected_size != Some(size_bytes) {
        return Err(fail(DerivedTypeReason::RowShape));
    }
    let machine_allowed = record.machine.is_none() || string_kind(primitive).is_some();
    if !machine_allowed || (record.machine.is_some() && record.platforms.len() != 1) {
        return Err(fail(DerivedTypeReason::RowShape));
    }
    Ok(())
}

/// Check the rules of a struct row (`plc_type`, `plc_type_member`).
///
/// The checks run in a fixed order. The row shape comes first, then the
/// member count, the member names, and the self reference.
fn check_struct(record: &PlcTypeRecord) -> Result<(), PlantError> {
    let RecordShape::Struct {
        name,
        revision,
        supersedes,
        members,
    } = &record.shape
    else {
        return Ok(());
    };
    let revision_in_range = i32::try_from(*revision).is_ok_and(|value| value >= 1);
    let supersedes_itself = *supersedes == Some(record.id);
    if record.machine.is_none()
        || record.platforms.len() != 1
        || !revision_in_range
        || supersedes_itself
    {
        return Err(invalid(
            format!("{name}@{revision}"),
            DerivedTypeReason::RowShape,
        ));
    }
    if members.is_empty() {
        return Err(PlantError::StructHasNoMembers {
            type_name: name.clone(),
        });
    }
    if members.len() > MAX_MEMBERS {
        return Err(PlantError::TypeTooLarge {
            type_name: name.to_string(),
            limit: TypeLimit::Members,
        });
    }
    let mut seen = BTreeSet::new();
    for member in members {
        if !seen.insert(&member.name) {
            return Err(PlantError::DuplicateMemberName {
                type_name: name.clone(),
                member: member.name.clone(),
            });
        }
    }
    if members.iter().any(|member| member.type_id == record.id) {
        return Err(PlantError::TypeCycle {
            type_name: name.clone(),
        });
    }
    Ok(())
}

/// Check the rules of an array row (`plc_type_array_shape`).
fn check_array(
    record: &PlcTypeRecord,
    element: PlcTypeId,
    dimensions: &ArrayDimensions,
) -> Result<(), PlantError> {
    if record.machine.is_none() || record.platforms.len() != 1 {
        return Err(invalid(
            format!("ARRAY[{dimensions}] OF {element}"),
            DerivedTypeReason::RowShape,
        ));
    }
    Ok(())
}

impl PlcType {
    /// Return the identifier of the type.
    #[must_use]
    pub const fn id(&self) -> PlcTypeId {
        self.id
    }

    /// Return the machine of the type, or `None` for the global catalog.
    #[must_use]
    pub const fn machine(&self) -> Option<MachineId> {
        self.machine
    }

    /// Return the platforms where the type exists.
    #[must_use]
    pub const fn platforms(&self) -> &BTreeSet<PlcPlatform> {
        &self.platforms
    }

    /// Return the shape of the row. The view is read-only.
    #[must_use]
    pub const fn shape(&self) -> &RecordShape {
        &self.shape
    }

    /// Return the kind of the row.
    #[must_use]
    pub const fn kind(&self) -> TypeKind {
        match self.shape {
            RecordShape::Primitive { .. } => TypeKind::Primitive,
            RecordShape::Array { .. } => TypeKind::Array,
            RecordShape::Struct { .. } => TypeKind::Struct,
        }
    }
}

#[cfg(test)]
mod tests;
