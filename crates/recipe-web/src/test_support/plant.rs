//! The plant data of the page tests: `Fixture::seed_plant` (design D12).
//!
//! The function solves one problem: the tests of the machine, unit, and type pages need one store
//! with the same machines, units, types, and recipe. A fixed identifier lets a test name a row, and
//! lets the matrix of the routes replace the placeholder `{uuid}` with the identifier of `M12`.
//!
//! The data:
//!
//! - The primitive types and the live machine `M7` (S7-300) with 64 live units in five levels.
//!   The chain `U1`, `U1.2`, `U1.2.4`, `U1.2.4.5`, and `U1.2.4.5.6` is part of them.
//! - The struct `ST_Zone` in the revisions 1 and 2, the array `ARRAY[1..4] OF ST_Zone@2`, and the
//!   struct `ST_Line@1` with the member `zones` of that array type. All of them belong to `M7`.
//! - The recipe `R-100` of `M7` with three versions. Version 3 is a live draft that holds the
//!   parameter `line` of the type `ST_Line@1` on the main unit `U1`.
//! - The archived machine `M12` with the live unit `U1` and the archived unit `U2`.

use std::collections::BTreeSet;

use recipe_app::memory::seed_type_id;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, Machine, MachineCode, MachineId, MemberName, PlatformTypes,
    PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, S7Address, TypeMember,
    TypeName, TypeTree, UnitId, UnitPath, UnitUsage,
};
use recipe_domain::recipe::{
    LangCode, Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, Recipe, RecipeId,
    RecipeVersionId, ReferenceNumber, StoredParameter, StoredRecipe, StoredVersionHeader,
    Translation, VersionState,
};
use recipe_domain::text::{LongText, Text};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::Fixture;

mod world;
pub(crate) use world::{
    ENGLISH, HTMX, HTMX_CONTENT, POLISH, TARGET_CONTENT, World, cookie_of, main_of, text_with,
    world,
};

/// The identifier of the live machine `M7`.
pub(crate) const M7_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0000_0007);
/// The identifier of the archived machine `M12`. The matrix of the routes replaces `{uuid}` with it.
pub(crate) const M12_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0000_000c);
/// The identifier of the revision 1 of the struct `ST_Zone`.
const ZONE_V1_ID: Uuid = Uuid::from_u128(0x7357_0003_0000_0000_0000_0000_0000_0001);
/// The identifier of the revision 2 of the struct `ST_Zone`.
const ZONE_V2_ID: Uuid = Uuid::from_u128(0x7357_0003_0000_0000_0000_0000_0000_0002);
/// The identifier of the array type `ARRAY[1..4] OF ST_Zone@2`.
const ZONES_ARRAY_ID: Uuid = Uuid::from_u128(0x7357_0003_0000_0000_0000_0000_0000_0003);
/// The identifier of the revision 1 of the struct `ST_Line`.
const LINE_V1_ID: Uuid = Uuid::from_u128(0x7357_0003_0000_0000_0000_0000_0000_0004);
/// The identifier of the recipe `R-100`.
const RECIPE_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0000_0100);
/// The identifier of the version 1 of `R-100`, a retired version.
const RETIRED_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0001);
/// The identifier of the version 2 of `R-100`, an approved version.
const APPROVED_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0002);
/// The identifier of the version 3 of `R-100`, the live draft.
const DRAFT_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0003);
/// The identifier of the parameter `line`.
const PARAMETER_ID: Uuid = Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0000_0001);
/// The key of the parameter `line`.
const PARAMETER_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0001);

/// The first field of the identifier of a unit. The next field holds the number of the machine.
const UNIT_TAG: u32 = 0x7357_0002;
/// The machine number in the identifiers of the units of `M7`.
const M7_NUMBER: u16 = 7;
/// The machine number in the identifiers of the units of `M12`.
const M12_NUMBER: u16 = 12;
/// The number of the user who created the recipe `R-100`. It is the Administrator `user1`.
const CREATOR_NUMBER: u128 = 1;
/// The instant of the archive of `M12`, in Unix seconds.
const ARCHIVED_AT_SECONDS: i64 = 500_000;

/// The units of `M7` below `U1`: a parent and the count of its children.
///
/// A child has the segments 1 to the count. A parent comes after the unit that creates it, so the
/// list builds the tree from the top. The list gives 63 units, and `U1` is the 64th.
const M7_PLAN: [(&str, u8); 11] = [
    ("U1", 4),
    ("U1.1", 4),
    ("U1.2", 4),
    ("U1.3", 4),
    ("U1.4", 4),
    ("U1.2.4", 5),
    ("U1.1.1", 8),
    ("U1.3.2", 8),
    ("U1.2.4.5", 6),
    ("U1.1.1.3", 8),
    ("U1.3.2.4", 8),
];

/// The fixed identifiers that `Fixture::seed_plant` returns.
///
/// The type solves one problem: a test needs the identifiers of the seeded rows to assert on them,
/// and each call of the function must give the same values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlantIds {
    /// The live machine `M7`.
    pub(crate) m7: MachineId,
    /// The archived machine `M12`.
    pub(crate) m12: MachineId,
    /// The live unit `U1` of `M12`.
    pub(crate) m12_live_unit: UnitId,
    /// The archived unit `U2` of `M12`.
    pub(crate) m12_archived_unit: UnitId,
    /// The revision 1 of the struct `ST_Zone`.
    pub(crate) zone_v1: PlcTypeId,
    /// The revision 2 of the struct `ST_Zone`.
    pub(crate) zone_v2: PlcTypeId,
    /// The type `ARRAY[1..4] OF ST_Zone@2`.
    pub(crate) zones_array: PlcTypeId,
    /// The revision 1 of the struct `ST_Line`.
    pub(crate) line_v1: PlcTypeId,
    /// The recipe `R-100`.
    pub(crate) recipe: RecipeId,
    /// The live draft, version 3 of `R-100`.
    pub(crate) draft: RecipeVersionId,
    /// The parameter `line` of the draft.
    pub(crate) parameter: ParameterId,
}

impl PlantIds {
    /// Return the fixed identifiers.
    fn fixed() -> Self {
        Self {
            m7: MachineId::from(M7_ID),
            m12: MachineId::from(M12_ID),
            m12_live_unit: unit_id(M12_NUMBER, &path("U1")),
            m12_archived_unit: unit_id(M12_NUMBER, &path("U2")),
            zone_v1: PlcTypeId::from(ZONE_V1_ID),
            zone_v2: PlcTypeId::from(ZONE_V2_ID),
            zones_array: PlcTypeId::from(ZONES_ARRAY_ID),
            line_v1: PlcTypeId::from(LINE_V1_ID),
            recipe: RecipeId::from(RECIPE_ID),
            draft: RecipeVersionId::from(DRAFT_ID),
            parameter: ParameterId::from(PARAMETER_ID),
        }
    }
}

impl Fixture {
    /// Seed the plant data of design D12 and return its fixed identifiers.
    ///
    /// The store gets the primitive types, the machines `M7` and `M12`, the types of `M7`, and the
    /// recipe `R-100`. A second call writes the same rows again.
    pub(crate) fn seed_plant(&self) -> PlantIds {
        let ids = PlantIds::fixed();
        self.store.seed_primitive_types();
        self.store.seed_machine(&m7_machine());
        for row in m7_type_rows() {
            self.store.seed_type(&row);
        }
        let recipe = recipe_r100();
        let creator = UserId::from(Uuid::from_u128(CREATOR_NUMBER));
        self.store
            .seed_recipe(&recipe, PlcPlatform::S7_300, creator);
        self.store.seed_parameter(ids.draft, &draft_parameter());
        self.store.seed_machine(&m12_machine());
        ids
    }
}

/// Parse a unit path.
fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

/// Parse a short text.
fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// Return the identifier of the unit `path` of the machine with the number `machine`.
///
/// The identifier holds the machine number and the segments of the path, so each unit has one
/// fixed identifier that shows where the unit belongs.
fn unit_id(machine: u16, unit_path: &UnitPath) -> UnitId {
    let mut segments = [0_u8; 8];
    for (slot, segment) in segments.iter_mut().zip(unit_path.segments()) {
        *slot = segment;
    }
    UnitId::from(Uuid::from_fields(UNIT_TAG, machine, 0, &segments))
}

/// Build the live machine `M7` with 64 live units in five levels.
fn m7_machine() -> Machine {
    let mut machine = Machine::create(
        MachineId::from(M7_ID),
        MachineCode::parse("M7").unwrap(),
        text("Press line"),
        PlcPlatform::S7_300,
        (unit_id(M7_NUMBER, &path("U1")), text("Infeed")),
    );
    for (parent_text, count) in M7_PLAN {
        let parent = path(parent_text);
        for segment in 1..=count {
            let child = path(&format!("{parent_text}.{segment}"));
            let name = text(&format!("Unit {child}"));
            let id = unit_id(M7_NUMBER, &child);
            machine
                .add_unit(id, Some(&parent), &segment.to_string(), name)
                .unwrap();
        }
    }
    machine
}

/// Build the archived machine `M12` with the live unit `U1` and the archived unit `U2`.
fn m12_machine() -> Machine {
    let archived_at =
        Timestamp::from(OffsetDateTime::from_unix_timestamp(ARCHIVED_AT_SECONDS).unwrap());
    let mut machine = Machine::create(
        MachineId::from(M12_ID),
        MachineCode::parse("M12").unwrap(),
        text("Old press line"),
        PlcPlatform::TwinCat3,
        (unit_id(M12_NUMBER, &path("U1")), text("Infeed")),
    );
    let spare = path("U2");
    machine
        .add_unit(unit_id(M12_NUMBER, &spare), None, "2", text("Spare"))
        .unwrap();
    machine
        .archive_unit(&spare, archived_at, &UnitUsage::default())
        .unwrap();
    machine.archive(archived_at, None).unwrap();
    machine
}

/// Return the platforms of the types of `M7`: only the S7-300.
fn s7_300() -> BTreeSet<PlcPlatform> {
    BTreeSet::from([PlcPlatform::S7_300])
}

/// Build a member of a struct.
fn member(name: &str, type_id: PlcTypeId) -> TypeMember {
    TypeMember {
        name: MemberName::parse(name).unwrap(),
        type_id,
    }
}

/// Build a struct row of `M7`.
fn struct_row(
    id: Uuid,
    (name, revision): (&str, u32),
    supersedes: Option<Uuid>,
    members: Vec<TypeMember>,
) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(id),
        machine: Some(MachineId::from(M7_ID)),
        platforms: s7_300(),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision,
            supersedes: supersedes.map(PlcTypeId::from),
            members,
        },
    })
    .unwrap()
}

/// Build the four type rows of `M7`: both revisions of `ST_Zone`, the array of the zones, and
/// `ST_Line@1`.
fn m7_type_rows() -> [PlcType; 4] {
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let real = seed_type_id(Primitive::Real, None).unwrap();
    let zones = PlcTypeId::from(ZONES_ARRAY_ID);
    let zone_v1 = struct_row(ZONE_V1_ID, ("ST_Zone", 1), None, vec![member("speed", int)]);
    let zone_v2 = struct_row(
        ZONE_V2_ID,
        ("ST_Zone", 2),
        Some(ZONE_V1_ID),
        vec![member("speed", int), member("temp_max", real)],
    );
    let array = PlcType::try_from(PlcTypeRecord {
        id: zones,
        machine: Some(MachineId::from(M7_ID)),
        platforms: s7_300(),
        shape: RecordShape::Array {
            element: PlcTypeId::from(ZONE_V2_ID),
            dimensions: ArrayDimensions::parse(vec![ArrayDimension::parse(1, 4).unwrap()]).unwrap(),
        },
    })
    .unwrap();
    let line = struct_row(
        LINE_V1_ID,
        ("ST_Line", 1),
        None,
        vec![member("zones", zones)],
    );
    [zone_v1, zone_v2, array, line]
}

/// Build the row of a seed primitive that the tree of `ST_Line@1` needs.
fn primitive_row(primitive: Primitive) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: seed_type_id(primitive, None).unwrap(),
        machine: None,
        platforms: BTreeSet::from(PlcPlatform::ALL),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap()
}

/// Resolve the type tree of `ST_Line@1` on the S7-300 of `M7`.
fn line_tree() -> TypeTree {
    let mut rows = vec![
        primitive_row(Primitive::Int),
        primitive_row(Primitive::Real),
    ];
    rows.extend(m7_type_rows());
    let types = PlatformTypes::collect(MachineId::from(M7_ID), PlcPlatform::S7_300, rows).unwrap();
    TypeTree::resolve(PlcTypeId::from(LINE_V1_ID), &types).unwrap()
}

/// Build the recipe `R-100` of `M7`: a retired version, an approved version, and a live draft.
fn recipe_r100() -> Recipe {
    let header = |id: Uuid, number: u32, state: VersionState| StoredVersionHeader {
        id: RecipeVersionId::from(id),
        number,
        state,
        row_version: 1,
    };
    Recipe::try_from(StoredRecipe {
        id: RecipeId::from(RECIPE_ID),
        machine: MachineId::from(M7_ID),
        reference: ReferenceNumber::parse("R-100").unwrap(),
        name: text("Press recipe"),
        deleted_at: None,
        versions: vec![
            header(RETIRED_ID, 1, VersionState::Retired),
            header(APPROVED_ID, 2, VersionState::Approved),
            header(DRAFT_ID, 3, VersionState::LiveDraft),
        ],
    })
    .unwrap()
}

/// The tag of the parameter `line`. The grammar of a tag has capital letters only (`tag.rs`), so
/// the tag is `LINE` and the English name of the parameter is `line`.
pub(crate) const LINE_TAG: &str = "LINE";

/// The JSON value of the parameter `line`: four zones with the value 0 in each leaf.
const LINE_VALUE: &str = r#"{"zones":[{"speed":0,"temp_max":0},{"speed":0,"temp_max":0},{"speed":0,"temp_max":0},{"speed":0,"temp_max":0}]}"#;

/// Build the parameter `line` of the draft. It stands on the main unit `U1` of `M7`.
fn draft_parameter() -> StoredParameter {
    let value = ParameterValue::parse(LINE_VALUE, &line_tree()).unwrap();
    StoredParameter {
        id: ParameterId::from(PARAMETER_ID),
        key: ParameterKey::from(PARAMETER_KEY),
        unit: unit_id(M7_NUMBER, &path("U1")),
        tag: ParameterTag::parse(LINE_TAG).unwrap(),
        name_en: text("line"),
        description_en: LongText::parse("The zones of the press line").unwrap(),
        translations: vec![Translation {
            lang: LangCode::parse("pl").unwrap(),
            name: text("Linia"),
            description: Some(LongText::parse("Strefy linii prasy").unwrap()),
        }],
        plc_type: PlcTypeId::from(LINE_V1_ID),
        s7_address: Some(S7Address::parse("DB1.DBB0").unwrap()),
        limits: Vec::new(),
        value: value.to_stored(),
        ordinal: Ordinal::parse(0).unwrap(),
    }
}

#[cfg(test)]
mod tests;
