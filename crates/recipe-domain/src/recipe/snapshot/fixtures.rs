//! Shared builders of the tests of the snapshot check and the approval check.

use std::collections::BTreeSet;

use rust_decimal::Decimal;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::plant::fixtures::{
    PLATFORM, array_row, id, machine, other_machine, primitive_row, string_row, struct_row,
    types_for, wstring_row,
};
use crate::plant::{
    LeafPath, LiveUnits, Machine, MachineCode, MachineId, MemberName, PlcPlatform, PlcType,
    PlcTypeRecord, Primitive, RecordShape, StoredMachine, StoredUnit, TypeMember, TypeName,
    TypeTrees, UnitId,
};
use crate::recipe::draft::support::{stored_parameter, tag};
use crate::recipe::{
    Recipe, RecipeId, RecipeVersionId, ReferenceNumber, StoredLimit, StoredParameter, StoredRecipe,
    StoredVersionHeader, VersionState,
};
use crate::text::Text;
use crate::timestamp::Timestamp;

pub(in crate::recipe) const INT: u128 = 1;
pub(in crate::recipe) const BOOL: u128 = 2;
pub(in crate::recipe) const STRING_20: u128 = 3;
pub(in crate::recipe) const WSTRING_10: u128 = 4;
/// `ARRAY[1..3] OF INT`.
pub(in crate::recipe) const RAMPS: u128 = 5;
/// A struct with `temp_sp`, `ramps`, `label`, and `active`.
pub(in crate::recipe) const ST_RECIPE: u128 = 6;
/// Revision 2 of `ST_Zone` with the member `temp`.
pub(in crate::recipe) const ZONE_2: u128 = 20;
/// Revision 3 of `ST_Zone`.
pub(in crate::recipe) const ZONE_3: u128 = 21;
/// A type identifier that no row has.
pub(in crate::recipe) const UNKNOWN: u128 = 99;

/// The identifier of the live unit `U1`.
pub(in crate::recipe) const LIVE_UNIT: u128 = 700;
/// The identifier of the archived unit `U1.2`.
pub(in crate::recipe) const ARCHIVED_UNIT: u128 = 701;

fn zone_row(n: u128, revision: u32, supersedes: Option<u128>) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: id(n),
        machine: Some(machine()),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Zone").unwrap(),
            revision,
            supersedes: supersedes.map(id),
            members: vec![TypeMember {
                name: MemberName::parse("temp").unwrap(),
                type_id: id(INT),
            }],
        },
    })
    .unwrap()
}

fn rows() -> Vec<PlcType> {
    vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(BOOL, Primitive::Bool),
        string_row(STRING_20, 20),
        wstring_row(WSTRING_10, 10),
        array_row(RAMPS, INT, &[(1, 3)]),
        struct_row(
            ST_RECIPE,
            "ST_Recipe",
            &[
                ("temp_sp", INT),
                ("ramps", RAMPS),
                ("label", STRING_20),
                ("active", BOOL),
            ],
        ),
        zone_row(ZONE_2, 2, None),
        zone_row(ZONE_3, 3, Some(ZONE_2)),
    ]
}

/// The trees of every test type on `platform`.
pub(in crate::recipe) fn trees_on(platform: PlcPlatform) -> TypeTrees {
    trees_of(machine(), platform)
}

/// The trees of every test type for `owner` on `platform`.
pub(in crate::recipe) fn trees_of(owner: MachineId, platform: PlcPlatform) -> TypeTrees {
    let ids = [
        INT, BOOL, STRING_20, WSTRING_10, RAMPS, ST_RECIPE, ZONE_2, ZONE_3, UNKNOWN,
    ]
    .map(id);
    TypeTrees::resolve(ids, &types_for(owner, platform, rows()))
}

/// The trees of the machine of the fixtures that were asked for only the types `asked`.
pub(in crate::recipe) fn trees_asking(asked: &[u128]) -> TypeTrees {
    let ids = asked.iter().copied().map(id);
    TypeTrees::resolve(ids, &types_for(machine(), PLATFORM, rows()))
}

/// The trees of every test type on the platform of the fixtures.
pub(in crate::recipe) fn trees() -> TypeTrees {
    trees_on(PLATFORM)
}

/// The machine `M7` with the live unit `U1` and the archived unit `U1.2`.
pub(in crate::recipe) fn units(is_machine_archived: bool) -> LiveUnits {
    units_of(machine(), is_machine_archived, 0)
}

/// The units of another machine. Its unit identifiers differ from the identifiers of `units`.
pub(in crate::recipe) fn foreign_units() -> LiveUnits {
    units_of(other_machine(), false, 1_000)
}

/// The machine `owner` with the live unit `U1` and the archived unit `U1.2`.
///
/// `shift` is added to the identifiers of the units.
fn units_of(owner: MachineId, is_machine_archived: bool, shift: u128) -> LiveUnits {
    let at = |seconds| Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap());
    let unit = |n: u128, path: &str, is_archived: bool| StoredUnit {
        id: UnitId::from(Uuid::from_u128(n.checked_add(shift).unwrap())),
        path: crate::plant::UnitPath::parse(path).unwrap(),
        name: Text::parse(path).unwrap(),
        archived_at: is_archived.then(|| at(5)),
    };
    Machine::try_from(StoredMachine {
        id: owner,
        code: MachineCode::parse("M7").unwrap(),
        name: Text::parse("Press").unwrap(),
        platform: PLATFORM,
        archived_at: is_machine_archived.then(|| at(9)),
        units: vec![
            unit(LIVE_UNIT, "U1", false),
            unit(ARCHIVED_UNIT, "U1.2", true),
        ],
    })
    .unwrap()
    .live_units()
}

/// Recipe `R-1` with the version 3. `is_deleted` marks the recipe as deleted.
pub(in crate::recipe) fn recipe(is_deleted: bool) -> Recipe {
    Recipe::try_from(StoredRecipe {
        id: RecipeId::from(Uuid::from_u128(1)),
        machine: machine(),
        reference: ReferenceNumber::parse("R-1").unwrap(),
        name: Text::parse("Recipe").unwrap(),
        deleted_at: is_deleted
            .then(|| Timestamp::from(OffsetDateTime::from_unix_timestamp(7).unwrap())),
        versions: [(10, 1), (20, 2), (30, 3)]
            .map(|(n, number)| StoredVersionHeader {
                id: RecipeVersionId::from(Uuid::from_u128(n)),
                number,
                state: VersionState::Retired,
                row_version: 7,
            })
            .to_vec(),
    })
    .unwrap()
}

/// A stored parameter on the live unit with the type `type_n` and the JSON `value`.
///
/// The key and the identifier derive from `n`.
pub(in crate::recipe) fn param(
    n: u128,
    tag_text: &str,
    type_n: u128,
    value: Value,
) -> StoredParameter {
    let mut stored = stored_parameter(n, tag_text, id(type_n), None);
    stored.unit = UnitId::from(Uuid::from_u128(LIVE_UNIT));
    stored.value = value.into();
    stored
}

/// The same parameter moved to another unit.
pub(in crate::recipe) fn on_unit(mut stored: StoredParameter, unit_n: u128) -> StoredParameter {
    stored.unit = UnitId::from(Uuid::from_u128(unit_n));
    stored
}

/// The same parameter with the stored limits.
pub(in crate::recipe) fn limited(
    mut stored: StoredParameter,
    limits: Vec<StoredLimit>,
) -> StoredParameter {
    stored.limits = limits;
    stored
}

/// A numeric limit row on `path` from `min` to `max`.
pub(in crate::recipe) fn range(path: &str, min: i64, max: i64) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: Some(Decimal::from(min)),
        max: Some(Decimal::from(max)),
        max_length: None,
    }
}

/// A string limit row on `path` with the largest length `length`.
pub(in crate::recipe) fn length(path: &str, length: u16) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: None,
        max: None,
        max_length: Some(length),
    }
}

/// The tag of the text. Re-exported for the tests of this context.
pub(in crate::recipe) fn parameter_tag(text: &str) -> crate::recipe::ParameterTag {
    tag(text)
}
