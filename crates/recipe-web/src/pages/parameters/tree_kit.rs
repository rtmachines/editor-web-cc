//! Type trees for the tests of the value fields and the leaf grid (design D16).
//!
//! The kit builds the rows of one machine on `TwinCat3` and resolves the tree of a root row. Each
//! call of a builder method adds one row with the next identifier.

use std::collections::BTreeSet;

use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, MachineId, MemberName, PlatformTypes, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, StringKind, StringLength, TypeMember,
    TypeName, TypeTree,
};
use recipe_domain::recipe::{LeafText, ParameterValue};
use uuid::Uuid;

/// The machine of every row of the kit.
const MACHINE: Uuid = Uuid::from_u128(0x7357_0f00_0000_0000_0000_0000_0000_0001);
/// The first identifier of a row of the kit.
const FIRST_ROW: u128 = 0x7357_0f01_0000_0000_0000_0000_0000_0000;
/// The platform of the kit. It has `WSTRING`, `LTIME`, and the dates from 1970.
pub(crate) const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

/// The rows of one machine and the identifier of the next row.
pub(crate) struct Kit {
    rows: Vec<PlcType>,
    next: u128,
}

impl Kit {
    /// Start a kit without rows.
    pub(crate) fn new() -> Self {
        Self {
            rows: Vec::new(),
            next: FIRST_ROW,
        }
    }

    /// Add one row with the next identifier and return the identifier.
    fn add(&mut self, shape: RecordShape) -> PlcTypeId {
        self.next = self.next.checked_add(1).unwrap();
        let id = PlcTypeId::from(Uuid::from_u128(self.next));
        let row = PlcType::try_from(PlcTypeRecord {
            id,
            machine: Some(MachineId::from(MACHINE)),
            platforms: BTreeSet::from([PLATFORM]),
            shape,
        })
        .unwrap();
        self.rows.push(row);
        id
    }

    /// Add the global row of a primitive without a length, once, and return its identifier.
    pub(crate) fn primitive(&mut self, primitive: Primitive) -> PlcTypeId {
        let shape = RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        };
        if let Some(row) = self.rows.iter().find(|row| *row.shape() == shape) {
            return row.id();
        }
        self.next = self.next.checked_add(1).unwrap();
        let id = PlcTypeId::from(Uuid::from_u128(self.next));
        let row = PlcType::try_from(PlcTypeRecord {
            id,
            machine: None,
            platforms: BTreeSet::from(PlcPlatform::ALL),
            shape,
        })
        .unwrap();
        self.rows.push(row);
        id
    }

    /// Add a `STRING(length)` or a `WSTRING(length)`.
    pub(crate) fn string(&mut self, primitive: Primitive, length: u32) -> PlcTypeId {
        let kind = if primitive == Primitive::Wstring {
            StringKind::Wstring
        } else {
            StringKind::String
        };
        let parsed = StringLength::parse(kind, PLATFORM, length).unwrap();
        self.add(RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: parsed.size_bytes(kind).unwrap(),
            string_length: Some(parsed.value()),
        })
    }

    /// Add an array of `element` with one dimension for each pair of bounds.
    pub(crate) fn array(&mut self, element: PlcTypeId, bounds: &[(i32, i32)]) -> PlcTypeId {
        let dimensions = bounds
            .iter()
            .map(|(lower, upper)| ArrayDimension::parse(*lower, *upper).unwrap())
            .collect();
        self.add(RecordShape::Array {
            element,
            dimensions: ArrayDimensions::parse(dimensions).unwrap(),
        })
    }

    /// Add the revision 1 of a struct with the members in their order.
    pub(crate) fn structure(&mut self, name: &str, members: &[(&str, PlcTypeId)]) -> PlcTypeId {
        let members = members
            .iter()
            .map(|(member, type_id)| TypeMember {
                name: MemberName::parse(member).unwrap(),
                type_id: *type_id,
            })
            .collect();
        self.add(RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision: 1,
            supersedes: None,
            members,
        })
    }

    /// Resolve the tree of `root` from the rows of the kit.
    pub(crate) fn tree(&self, root: PlcTypeId) -> TypeTree {
        let types =
            PlatformTypes::collect(MachineId::from(MACHINE), PLATFORM, self.rows.clone()).unwrap();
        TypeTree::resolve(root, &types).unwrap()
    }
}

/// Resolve the tree of one primitive.
pub(crate) fn primitive_tree(primitive: Primitive) -> TypeTree {
    let mut kit = Kit::new();
    let root = kit.primitive(primitive);
    kit.tree(root)
}

/// Resolve the tree of one string type of `length` characters.
pub(crate) fn string_tree(primitive: Primitive, length: u32) -> TypeTree {
    let mut kit = Kit::new();
    let root = kit.string(primitive, length);
    kit.tree(root)
}

/// Resolve the tree of an array of `primitive` with the dimensions `bounds`.
pub(crate) fn array_tree(primitive: Primitive, bounds: &[(i32, i32)]) -> TypeTree {
    let mut kit = Kit::new();
    let element = kit.primitive(primitive);
    let root = kit.array(element, bounds);
    kit.tree(root)
}

/// Resolve `ST_Line`: `name` (`STRING(20)`), `speed` (`INT`), and the struct `zone` of
/// `ST_Zone` with `setpoint` (`REAL`), `hold` (`TIME`), and `enabled` (`BOOL`).
pub(crate) fn line_tree() -> TypeTree {
    let mut kit = Kit::new();
    let zone = zone_row(&mut kit);
    let name = kit.string(Primitive::String, 20);
    let int = kit.primitive(Primitive::Int);
    let root = kit.structure("ST_Line", &[("name", name), ("speed", int), ("zone", zone)]);
    kit.tree(root)
}

/// Resolve `ARRAY[1..4] OF ST_Zone`.
pub(crate) fn zones_tree() -> TypeTree {
    let mut kit = Kit::new();
    let zone = zone_row(&mut kit);
    let root = kit.array(zone, &[(1, 4)]);
    kit.tree(root)
}

/// Add the struct `ST_Zone` with `setpoint` (`REAL`), `hold` (`TIME`), and `enabled` (`BOOL`).
fn zone_row(kit: &mut Kit) -> PlcTypeId {
    let real = kit.primitive(Primitive::Real);
    let time = kit.primitive(Primitive::Time);
    let flag = kit.primitive(Primitive::Bool);
    kit.structure(
        "ST_Zone",
        &[("setpoint", real), ("hold", time), ("enabled", flag)],
    )
}

/// Write the texts of the leaves of the JSON value `json` of `tree`.
pub(crate) fn stored_texts(tree: &TypeTree, json: &str) -> Vec<LeafText> {
    ParameterValue::parse(json, tree)
        .unwrap()
        .texts(tree)
        .unwrap()
}
