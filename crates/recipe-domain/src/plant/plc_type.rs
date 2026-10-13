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
mod tests {
    use super::*;
    use crate::plant::ArrayDimension;
    use uuid::Uuid;

    fn type_id(n: u128) -> PlcTypeId {
        PlcTypeId::from(Uuid::from_u128(n))
    }

    fn machine_id() -> MachineId {
        MachineId::from(Uuid::from_u128(99))
    }

    fn platforms(list: &[PlcPlatform]) -> BTreeSet<PlcPlatform> {
        list.iter().copied().collect()
    }

    fn all() -> BTreeSet<PlcPlatform> {
        PlcPlatform::ALL.into_iter().collect()
    }

    fn s7() -> BTreeSet<PlcPlatform> {
        platforms(&[
            PlcPlatform::S7_300,
            PlcPlatform::S7_1200,
            PlcPlatform::S7_1500,
        ])
    }

    fn primitive(
        primitive: Primitive,
        size_bytes: u32,
        string_length: Option<u16>,
        machine: Option<MachineId>,
        platforms: BTreeSet<PlcPlatform>,
    ) -> PlcTypeRecord {
        PlcTypeRecord {
            id: type_id(1),
            machine,
            platforms,
            shape: RecordShape::Primitive {
                primitive,
                leaf_class: primitive.leaf_class(),
                size_bytes,
                string_length,
            },
        }
    }

    /// Return the fields of the `E-PLT-013` error that a bad record gives.
    fn parts(record: PlcTypeRecord) -> (String, DerivedTypeReason) {
        let error = PlcType::try_from(record).unwrap_err();
        assert_eq!(error.code(), crate::error_codes::ErrorCode::PLT_013);
        match error {
            PlantError::DerivedTypeInvalid { type_expr, reason } => (type_expr, reason),
            _ => (String::new(), DerivedTypeReason::Grammar),
        }
    }

    fn reason(record: PlcTypeRecord) -> DerivedTypeReason {
        parts(record).1
    }

    fn type_expr(record: PlcTypeRecord) -> String {
        parts(record).0
    }

    #[test]
    fn seed_int_on_five_platforms_is_valid() {
        let record = primitive(Primitive::Int, 2, None, None, all());
        let built = PlcType::try_from(record.clone()).unwrap();
        assert_eq!(built.id(), type_id(1));
        assert_eq!(built.kind(), TypeKind::Primitive);
        assert_eq!(built.machine(), None);
        assert_eq!(built.platforms(), &all());
        assert_eq!(built.shape(), &record.shape);
    }

    #[test]
    fn seed_string_254_on_the_s7_platforms_is_valid() {
        let record = primitive(Primitive::String, 256, Some(254), None, s7());
        assert!(PlcType::try_from(record).is_ok());
    }

    #[test]
    fn machine_string_20_on_twincat3_has_size_22() {
        let one = platforms(&[PlcPlatform::TwinCat3]);
        let record = primitive(Primitive::String, 22, Some(20), Some(machine_id()), one);
        let built = PlcType::try_from(record).unwrap();
        assert_eq!(built.machine(), Some(machine_id()));
    }

    #[test]
    fn machine_wstring_80_has_size_164() {
        let one = platforms(&[PlcPlatform::S7_1500]);
        let record = primitive(Primitive::Wstring, 164, Some(80), Some(machine_id()), one);
        assert!(PlcType::try_from(record).is_ok());
    }

    #[test]
    fn wrong_string_size_is_a_row_shape_error() {
        let one = platforms(&[PlcPlatform::TwinCat3]);
        let record = primitive(Primitive::String, 20, Some(20), Some(machine_id()), one);
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
        let one = platforms(&[PlcPlatform::TwinCat3]);
        let record = primitive(Primitive::Wstring, 82, Some(40), Some(machine_id()), one);
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
    }

    #[test]
    fn wrong_fixed_size_is_a_row_shape_error() {
        let record = primitive(Primitive::Int, 4, None, None, all());
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
    }

    #[test]
    fn empty_platforms_is_a_row_shape_error() {
        let record = primitive(Primitive::Int, 2, None, None, BTreeSet::new());
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
    }

    #[test]
    fn wrong_leaf_class_is_a_row_shape_error() {
        let mut record = primitive(Primitive::Int, 2, None, None, all());
        record.shape = RecordShape::Primitive {
            primitive: Primitive::Int,
            leaf_class: LeafClass::String,
            size_bytes: 2,
            string_length: None,
        };
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
    }

    #[test]
    fn string_length_is_required_for_strings_only() {
        let record = primitive(Primitive::String, 256, None, None, s7());
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
        let record = primitive(Primitive::Wstring, 164, None, None, s7());
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
        let record = primitive(Primitive::Int, 2, Some(20), None, all());
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
    }

    #[test]
    fn string_255_on_s7_1500_is_out_of_range() {
        let one = platforms(&[PlcPlatform::S7_1500]);
        let record = primitive(Primitive::String, 257, Some(255), Some(machine_id()), one);
        assert_eq!(reason(record), DerivedTypeReason::OutOfRange);
    }

    #[test]
    fn string_length_is_checked_on_every_platform() {
        let both = platforms(&[PlcPlatform::TwinCat3, PlcPlatform::S7_1500]);
        let record = primitive(Primitive::String, 257, Some(255), None, both);
        assert_eq!(reason(record), DerivedTypeReason::OutOfRange);
    }

    #[test]
    fn string_length_zero_is_out_of_range() {
        let record = primitive(Primitive::String, 2, Some(0), None, s7());
        assert_eq!(reason(record), DerivedTypeReason::OutOfRange);
    }

    #[test]
    fn wstring_10_on_s7_300_is_not_on_platform() {
        let one = platforms(&[PlcPlatform::S7_300]);
        let record = primitive(Primitive::Wstring, 24, Some(10), Some(machine_id()), one);
        assert_eq!(reason(record), DerivedTypeReason::NotOnPlatform);
    }

    #[test]
    fn machine_on_a_non_string_primitive_is_a_row_shape_error() {
        let one = platforms(&[PlcPlatform::TwinCat3]);
        let record = primitive(Primitive::Int, 2, None, Some(machine_id()), one);
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
    }

    #[test]
    fn row_with_a_machine_needs_exactly_one_platform() {
        let two = platforms(&[PlcPlatform::TwinCat3, PlcPlatform::CodesysIec]);
        let record = primitive(Primitive::String, 22, Some(20), Some(machine_id()), two);
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
    }

    #[test]
    fn failure_names_a_sized_string_and_a_plain_primitive() {
        let one = platforms(&[PlcPlatform::S7_1500]);
        let record = primitive(Primitive::String, 257, Some(255), Some(machine_id()), one);
        assert_eq!(type_expr(record), "STRING(255)");
        let record = primitive(Primitive::Int, 4, None, None, all());
        assert_eq!(type_expr(record), "INT");
    }

    fn array(machine: Option<MachineId>, platforms: BTreeSet<PlcPlatform>) -> PlcTypeRecord {
        PlcTypeRecord {
            id: type_id(2),
            machine,
            platforms,
            shape: RecordShape::Array {
                element: type_id(1),
                dimensions: ArrayDimensions::parse(vec![ArrayDimension::parse(1, 3).unwrap()])
                    .unwrap(),
            },
        }
    }

    #[test]
    fn array_of_a_machine_is_valid() {
        let one = platforms(&[PlcPlatform::TwinCat3]);
        let built = PlcType::try_from(array(Some(machine_id()), one)).unwrap();
        assert_eq!(built.kind(), TypeKind::Array);
    }

    #[test]
    fn array_without_machine_is_a_row_shape_error() {
        let one = platforms(&[PlcPlatform::TwinCat3]);
        assert_eq!(reason(array(None, one)), DerivedTypeReason::RowShape);
    }

    #[test]
    fn array_needs_exactly_one_platform() {
        let two = platforms(&[PlcPlatform::TwinCat3, PlcPlatform::CodesysIec]);
        assert_eq!(
            reason(array(Some(machine_id()), two)),
            DerivedTypeReason::RowShape
        );
        assert_eq!(
            reason(array(Some(machine_id()), BTreeSet::new())),
            DerivedTypeReason::RowShape
        );
    }

    #[test]
    fn array_failure_names_dimensions_and_element() {
        assert_eq!(
            type_expr(array(None, all())),
            format!("ARRAY[1..3] OF {}", type_id(1))
        );
    }

    fn name(text: &str) -> TypeName {
        TypeName::parse(text).unwrap()
    }

    fn member(text: &str, type_number: u128) -> TypeMember {
        TypeMember {
            name: MemberName::parse(text).unwrap(),
            type_id: type_id(type_number),
        }
    }

    fn udt(revision: u32, members: Vec<TypeMember>) -> PlcTypeRecord {
        PlcTypeRecord {
            id: type_id(10),
            machine: Some(machine_id()),
            platforms: platforms(&[PlcPlatform::TwinCat3]),
            shape: RecordShape::Struct {
                name: name("ST_Zone"),
                revision,
                supersedes: None,
                members,
            },
        }
    }

    fn members_of(count: usize) -> Vec<TypeMember> {
        (0..count)
            .map(|index| member(&format!("m{index}"), 1))
            .collect()
    }

    #[test]
    fn valid_struct_keeps_the_member_order() {
        let members = vec![member("zeta", 1), member("alpha", 2), member("mid", 3)];
        let built = PlcType::try_from(udt(3, members.clone())).unwrap();
        assert_eq!(built.kind(), TypeKind::Struct);
        assert_eq!(built.shape(), &udt(3, members).shape);
    }

    #[test]
    fn struct_without_member_is_plt_016() {
        let error = PlcType::try_from(udt(1, Vec::new())).unwrap_err();
        assert!(matches!(error, PlantError::StructHasNoMembers { .. }));
        assert_eq!(error.code().as_str(), "E-PLT-016");
        assert_eq!(
            error.to_string(),
            "Type ST_Zone has no members. Add at least one member."
        );
    }

    #[test]
    fn struct_with_1024_members_is_valid() {
        assert!(PlcType::try_from(udt(1, members_of(1024))).is_ok());
    }

    #[test]
    fn struct_with_1025_members_is_plt_018_with_the_members_limit() {
        let error = PlcType::try_from(udt(1, members_of(1025))).unwrap_err();
        assert!(matches!(
            error,
            PlantError::TypeTooLarge {
                limit: crate::plant::TypeLimit::Members,
                ..
            }
        ));
        assert_eq!(error.code().as_str(), "E-PLT-018");
        assert_eq!(
            error.to_string(),
            "Type ST_Zone is larger than the limit 1024 members. Use fewer elements, fewer members, or fewer levels."
        );
    }

    #[test]
    fn duplicate_member_name_is_plt_021_and_names_the_first_duplicate() {
        let members = vec![
            member("a", 1),
            member("b", 1),
            member("b", 2),
            member("a", 2),
        ];
        let error = PlcType::try_from(udt(1, members)).unwrap_err();
        assert!(
            matches!(&error, PlantError::DuplicateMemberName { member, .. }
            if member.as_str() == "b")
        );
        assert_eq!(error.code().as_str(), "E-PLT-021");
    }

    #[test]
    fn member_of_the_own_type_is_plt_005() {
        let members = vec![member("a", 1), member("self_ref", 10)];
        let error = PlcType::try_from(udt(1, members)).unwrap_err();
        assert!(matches!(error, PlantError::TypeCycle { .. }));
        assert_eq!(error.code().as_str(), "E-PLT-005");
        assert_eq!(
            error.to_string(),
            "Type ST_Zone contains itself. Delete the circular member."
        );
    }

    #[test]
    fn struct_without_machine_is_a_row_shape_error() {
        let mut record = udt(1, members_of(1));
        record.machine = None;
        let (expr, why) = parts(record);
        assert_eq!(why, DerivedTypeReason::RowShape);
        assert_eq!(expr, "ST_Zone@1");
    }

    #[test]
    fn struct_needs_exactly_one_platform() {
        let mut record = udt(1, members_of(1));
        record.platforms = all();
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
        let mut record = udt(1, members_of(1));
        record.platforms = BTreeSet::new();
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
    }

    #[test]
    fn struct_revision_must_be_in_1_to_i32_max() {
        assert_eq!(reason(udt(0, members_of(1))), DerivedTypeReason::RowShape);
        assert_eq!(
            reason(udt(2_147_483_648, members_of(1))),
            DerivedTypeReason::RowShape
        );
        assert!(PlcType::try_from(udt(2_147_483_647, members_of(1))).is_ok());
    }

    #[test]
    fn struct_that_supersedes_itself_is_a_row_shape_error() {
        let mut record = udt(2, members_of(1));
        if let RecordShape::Struct { supersedes, .. } = &mut record.shape {
            *supersedes = Some(type_id(10));
        }
        let (expr, why) = parts(record);
        assert_eq!(why, DerivedTypeReason::RowShape);
        assert_eq!(expr, "ST_Zone@2");
    }

    #[test]
    fn struct_that_supersedes_another_row_is_valid() {
        let mut record = udt(2, members_of(1));
        if let RecordShape::Struct { supersedes, .. } = &mut record.shape {
            *supersedes = Some(type_id(9));
        }
        assert!(PlcType::try_from(record).is_ok());
    }

    #[test]
    fn struct_shape_error_comes_before_the_member_errors() {
        let mut record = udt(1, Vec::new());
        record.machine = None;
        assert_eq!(reason(record), DerivedTypeReason::RowShape);
    }
}
