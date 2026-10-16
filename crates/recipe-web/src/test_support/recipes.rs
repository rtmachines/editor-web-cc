//! The recipe data of the page tests: `Fixture::seed_recipes` and `Fixture::seed_large_draft`
//! (design D12).
//!
//! The functions solve one problem: the tests of the recipe, version, and parameter pages need one
//! store with the same recipes, types, and parameters. A fixed identifier lets a test name a row,
//! and lets the matrix of the routes replace the placeholder `{key}` with the key of `LINE`.
//!
//! The data of `seed_recipes` comes on top of `seed_plant`:
//!
//! - The live machine `M20` (`TwinCat3`) with the units `U1` and `U2`.
//! - The struct `ST_Zone@3` with one more member than `ST_Zone@2`, so `ZONES` is outdated.
//! - In the draft version 3 of `R-100`, seven parameters. `SPEED`, `LABEL`, and `HOLD` stand on
//!   `U1`. The other four stand on `U1.3`.
//!   - `SPEED` is an `INT` with the limit 0 to 3000.
//!   - `LABEL` is a `STRING(20)`.
//!   - `HOLD` is a `TIME`.
//!   - `SPEED_TABLE` is an `ARRAY[1..8] OF INT` with the limit `[*]`.
//!   - `GRID` is an `ARRAY[0..1,1..2] OF REAL`.
//!   - `LINE_A` is a struct in a struct.
//!   - `ZONES` is an `ARRAY[1..4] OF ST_Zone@2`.
//! - The recipe `R-200` of `M20` with one draft, and the deleted recipe `R-300` of `M20`.
//!
//! Each S7 address has the width of its type and does not overlap another address of the draft.
//! A test sends a value command for each parameter to prove it.

#[cfg(test)]
mod invariant_tests;
pub(crate) mod large;
mod parameters;

use std::collections::BTreeSet;

use recipe_app::memory::seed_type_id;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, Machine, MachineCode, MachineId, MemberName, PlcPlatform,
    PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, StringKind, StringLength,
    TypeMember, TypeName, UnitId,
};
use recipe_domain::recipe::{
    Recipe, RecipeId, RecipeVersionId, ReferenceNumber, StoredRecipe, StoredVersionHeader,
    VersionState,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::Fixture;
use super::plant::PlantIds;

/// The identifier of the live machine `M20`.
pub(crate) const M20_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0000_0014);
/// The identifier of the unit `U1` of `M20`.
const M20_U1_ID: Uuid = Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0014_0001);
/// The identifier of the unit `U2` of `M20`.
const M20_U2_ID: Uuid = Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0014_0002);
/// The identifier of the revision 3 of the struct `ST_Zone`.
const ZONE_V3_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0001);
/// The identifier of the type `STRING(20)` of `M7`.
const STRING_20_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0002);
/// The identifier of the type `ARRAY[1..8] OF INT` of `M7`.
const SPEED_TABLE_TYPE_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0003);
/// The identifier of the type `ARRAY[0..1,1..2] OF REAL` of `M7`.
const GRID_TYPE_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0004);
/// The identifier of the revision 1 of the struct `ST_Pair`.
const PAIR_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0005);
/// The identifier of the revision 1 of the struct `ST_Cell`, a struct with a struct member.
const CELL_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0006);
/// The identifier of the recipe `R-200`.
const R200_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0000_0200);
/// The identifier of the draft version 1 of `R-200`.
const R200_DRAFT_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0201);
/// The identifier of the recipe `R-300`, a deleted recipe.
const R300_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0000_0300);
/// The identifier of the retired version 1 of `R-300`.
const R300_VERSION_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0301);

/// The key of the parameter `LINE` of `seed_plant`. The matrix replaces `{key}` with it.
pub(crate) const LINE_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0001);
/// The key of the parameter `SPEED`.
pub(crate) const SPEED_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0011);
/// The key of the parameter `LABEL`.
pub(crate) const LABEL_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0012);
/// The key of the parameter `HOLD`.
pub(crate) const HOLD_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0013);
/// The key of the parameter `SPEED_TABLE`.
pub(crate) const SPEED_TABLE_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0014);
/// The key of the parameter `GRID`.
pub(crate) const GRID_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0015);
/// The key of the parameter `LINE_A`.
pub(crate) const LINE_A_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0016);
/// The key of the parameter `ZONES`.
pub(crate) const ZONES_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0017);

/// The number of the user who created the recipes. It is the Administrator `user1`.
const CREATOR_NUMBER: u128 = 1;
/// The instant of the deletion of `R-300`, in Unix seconds.
const DELETED_AT_SECONDS: i64 = 600_000;

/// The fixed identifiers that `Fixture::seed_recipes` returns.
///
/// The type solves one problem: a test needs the identifiers of the seeded rows to assert on them,
/// and each call of the function must give the same values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RecipeIds {
    /// The identifiers of `seed_plant`.
    pub(crate) plant: PlantIds,
    /// The live machine `M20`.
    pub(crate) m20: MachineId,
    /// The recipe `R-200`.
    pub(crate) r200: RecipeId,
    /// The draft version 1 of `R-200`.
    pub(crate) r200_draft: RecipeVersionId,
    /// The deleted recipe `R-300`.
    pub(crate) r300: RecipeId,
    /// The revision 3 of the struct `ST_Zone`.
    pub(crate) zone_v3: PlcTypeId,
}

impl Fixture {
    /// Seed the recipe data of design D12 and return its fixed identifiers.
    ///
    /// The function calls `seed_plant` first. A second call writes the same rows again.
    pub(crate) fn seed_recipes(&self) -> RecipeIds {
        let plant = self.seed_plant();
        for row in extra_type_rows(&plant) {
            self.store.seed_type(&row);
        }
        for parameter in parameters::draft_parameters(self, &plant) {
            self.store.seed_parameter(plant.draft, &parameter);
        }
        self.store.seed_machine(&m20_machine());
        let creator = UserId::from(Uuid::from_u128(CREATOR_NUMBER));
        self.store
            .seed_recipe(&recipe_r200(), PlcPlatform::TwinCat3, creator);
        self.store
            .seed_recipe(&recipe_r300(), PlcPlatform::TwinCat3, creator);
        RecipeIds {
            plant,
            m20: MachineId::from(M20_ID),
            r200: RecipeId::from(R200_ID),
            r200_draft: RecipeVersionId::from(R200_DRAFT_ID),
            r300: RecipeId::from(R300_ID),
            zone_v3: PlcTypeId::from(ZONE_V3_ID),
        }
    }
}

/// Parse a short text.
fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// Build the live machine `M20` with the units `U1` and `U2`.
fn m20_machine() -> Machine {
    let mut machine = Machine::create(
        MachineId::from(M20_ID),
        MachineCode::parse("M20").unwrap(),
        text("Packing line"),
        PlcPlatform::TwinCat3,
        (UnitId::from(M20_U1_ID), text("Infeed")),
    );
    machine
        .add_unit(UnitId::from(M20_U2_ID), None, "2", text("Outfeed"))
        .unwrap();
    machine
}

/// Build a member of a struct.
fn member(name: &str, type_id: PlcTypeId) -> TypeMember {
    TypeMember {
        name: MemberName::parse(name).unwrap(),
        type_id,
    }
}

/// Build a row of `M7` on the S7-300 with the identifier `id`.
fn m7_row(plant: &PlantIds, id: Uuid, shape: RecordShape) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(id),
        machine: Some(plant.m7),
        platforms: BTreeSet::from([PlcPlatform::S7_300]),
        shape,
    })
    .unwrap()
}

/// Build the shape of a struct without a predecessor.
fn struct_shape(name: &str, revision: u32, members: Vec<TypeMember>) -> RecordShape {
    RecordShape::Struct {
        name: TypeName::parse(name).unwrap(),
        revision,
        supersedes: None,
        members,
    }
}

/// Build the shape of an array of `element` with the dimensions `bounds`.
fn array_shape(element: PlcTypeId, bounds: &[(i32, i32)]) -> RecordShape {
    let dimensions = bounds
        .iter()
        .map(|(lower, upper)| ArrayDimension::parse(*lower, *upper).unwrap())
        .collect();
    RecordShape::Array {
        element,
        dimensions: ArrayDimensions::parse(dimensions).unwrap(),
    }
}

/// Build the shape of `STRING(20)` on the S7-300.
fn string_20_shape() -> RecordShape {
    let length = StringLength::parse(StringKind::String, PlcPlatform::S7_300, 20).unwrap();
    RecordShape::Primitive {
        primitive: Primitive::String,
        leaf_class: Primitive::String.leaf_class(),
        size_bytes: length.size_bytes(StringKind::String).unwrap(),
        string_length: Some(length.value()),
    }
}

/// Build the six type rows that `seed_recipes` adds to the types of `M7`.
///
/// The rows are `ST_Zone@3`, `STRING(20)`, the two arrays, `ST_Pair@1`, and `ST_Cell@1`.
fn extra_type_rows(plant: &PlantIds) -> Vec<PlcType> {
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let real = seed_type_id(Primitive::Real, None).unwrap();
    let pair = PlcTypeId::from(PAIR_ID);
    let zone_v3 = RecordShape::Struct {
        name: TypeName::parse("ST_Zone").unwrap(),
        revision: 3,
        supersedes: Some(plant.zone_v2),
        members: vec![
            member("speed", int),
            member("temp_max", real),
            member("temp_min", real),
        ],
    };
    vec![
        m7_row(plant, ZONE_V3_ID, zone_v3),
        m7_row(plant, STRING_20_ID, string_20_shape()),
        m7_row(plant, SPEED_TABLE_TYPE_ID, array_shape(int, &[(1, 8)])),
        m7_row(plant, GRID_TYPE_ID, array_shape(real, &[(0, 1), (1, 2)])),
        m7_row(
            plant,
            PAIR_ID,
            struct_shape("ST_Pair", 1, vec![member("a", int), member("b", real)]),
        ),
        m7_row(
            plant,
            CELL_ID,
            struct_shape(
                "ST_Cell",
                1,
                vec![member("head", pair), member("count", int)],
            ),
        ),
    ]
}

/// Build the header of a version.
fn header(id: Uuid, number: u32, state: VersionState) -> StoredVersionHeader {
    StoredVersionHeader {
        id: RecipeVersionId::from(id),
        number,
        state,
        row_version: 1,
    }
}

/// Build the recipe `R-200` of `M20` with one live draft.
fn recipe_r200() -> Recipe {
    Recipe::try_from(StoredRecipe {
        id: RecipeId::from(R200_ID),
        machine: MachineId::from(M20_ID),
        reference: ReferenceNumber::parse("R-200").unwrap(),
        name: text("Packing recipe"),
        deleted_at: None,
        versions: vec![header(R200_DRAFT_ID, 1, VersionState::LiveDraft)],
    })
    .unwrap()
}

/// Build the deleted recipe `R-300` of `M20` with one retired version.
fn recipe_r300() -> Recipe {
    let deleted_at =
        Timestamp::from(OffsetDateTime::from_unix_timestamp(DELETED_AT_SECONDS).unwrap());
    Recipe::try_from(StoredRecipe {
        id: RecipeId::from(R300_ID),
        machine: MachineId::from(M20_ID),
        reference: ReferenceNumber::parse("R-300").unwrap(),
        name: text("Old packing recipe"),
        deleted_at: Some(deleted_at),
        versions: vec![header(R300_VERSION_ID, 1, VersionState::Retired)],
    })
    .unwrap()
}

#[cfg(test)]
mod tests;
