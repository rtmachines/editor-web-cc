//! Shared builders of the tests of the rebind mapping.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::plant::fixtures::{
    array_row, id, machine, primitive_row, string_row, struct_row_revision, types_for,
};
use crate::plant::{LeafPath, PlcPlatform, Primitive, TypeTrees};
use crate::recipe::draft::support::{stored_draft_with, stored_parameter};
use crate::recipe::{
    Draft, ParameterKey, RebindPlan, RebindRequest, StoredLimit, StoredParameter, VersionState,
};

pub(super) const INT: u128 = 1;
pub(super) const DINT: u128 = 2;
pub(super) const STR: u128 = 4;
pub(super) const ZONE_2: u128 = 20;
pub(super) const ZONE_3: u128 = 21;
pub(super) const RAMP_1: u128 = 22;
pub(super) const RAMP_2: u128 = 23;
pub(super) const MODE_1: u128 = 24;
pub(super) const MODE_2: u128 = 25;
pub(super) const OLD_1: u128 = 26;
pub(super) const OLD_2: u128 = 27;
pub(super) const NAME_1: u128 = 28;
pub(super) const NAME_2: u128 = 29;
pub(super) const INT_3: u128 = 30;
pub(super) const ZONES_2: u128 = 31;
pub(super) const ZONES_4: u128 = 32;
pub(super) const BIG_1: u128 = 50;
pub(super) const BIG_2: u128 = 51;
pub(super) const BIG_ARRAY: u128 = 52;
pub(super) const FILL: u128 = 53;
pub(super) const OLDS_1: u128 = 54;
/// `ARRAY[1..2] OF ST_Zone@3`, the new type of `ZONES_2`.
pub(super) const ZONES_2_NEW: u128 = 34;
/// `ARRAY[1..4] OF ST_Zone@3`, the new type of `ZONES_4`.
pub(super) const ZONES_4_NEW: u128 = 35;
/// `ARRAY[1..8192] OF ST_Big@2`, the new type of `BIG_ARRAY`.
pub(super) const BIG_ARRAY_NEW: u128 = 55;

const ALL_TYPES: [u128; 28] = [
    INT,
    DINT,
    STR,
    ZONE_2,
    ZONE_3,
    RAMP_1,
    RAMP_2,
    MODE_1,
    MODE_2,
    OLD_1,
    OLD_2,
    NAME_1,
    NAME_2,
    INT_3,
    ZONES_2,
    ZONES_4,
    ZONES_2_NEW,
    ZONES_4_NEW,
    BIG_1,
    BIG_2,
    BIG_ARRAY,
    BIG_ARRAY_NEW,
    FILL,
    OLDS_1,
    99,
    98,
    97,
    96,
];

/// The trees of the fixture catalog.
///
/// `ST_Zone` gets `temp_max`. `ST_Ramp` keeps an array member and gets `extra`. `ST_Mode`
/// changes the type of `mode`. `ST_Old` drops `old_max`. `ST_Name` gets a string member.
pub(super) fn trees() -> TypeTrees {
    trees_with_zone_3(&[("setpoint", INT), ("temp_max", INT)])
}

/// The trees of the fixture catalog, with the given members for `ST_Zone@3`.
pub(super) fn trees_with_zone_3(zone_3: &[(&str, u128)]) -> TypeTrees {
    let rows = vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(DINT, Primitive::Dint),
        string_row(STR, 10),
        struct_row_revision(ZONE_2, "ST_Zone", 2, &[("setpoint", INT)]),
        struct_row_revision(ZONE_3, "ST_Zone", 3, zone_3),
        struct_row_revision(RAMP_1, "ST_Ramp", 1, &[("setpoint", INT), ("ramps", INT_3)]),
        struct_row_revision(
            RAMP_2,
            "ST_Ramp",
            2,
            &[("setpoint", INT), ("ramps", INT_3), ("extra", INT)],
        ),
        struct_row_revision(MODE_1, "ST_Mode", 1, &[("setpoint", INT), ("mode", INT)]),
        struct_row_revision(MODE_2, "ST_Mode", 2, &[("setpoint", INT), ("mode", DINT)]),
        struct_row_revision(OLD_1, "ST_Old", 1, &[("setpoint", INT), ("old_max", INT)]),
        struct_row_revision(OLD_2, "ST_Old", 2, &[("setpoint", INT)]),
        struct_row_revision(NAME_1, "ST_Name", 1, &[("setpoint", INT)]),
        struct_row_revision(NAME_2, "ST_Name", 2, &[("setpoint", INT), ("label", STR)]),
        array_row(INT_3, INT, &[(1, 3)]),
        array_row(ZONES_2, ZONE_2, &[(1, 2)]),
        array_row(ZONES_4, ZONE_2, &[(1, 4)]),
        struct_row_revision(BIG_1, "ST_Big", 1, &[("a", INT)]),
        struct_row_revision(BIG_2, "ST_Big", 2, &[("a", INT), ("b", INT)]),
        array_row(BIG_ARRAY, BIG_1, &[(1, 8_192)]),
        array_row(ZONES_2_NEW, ZONE_3, &[(1, 2)]),
        array_row(ZONES_4_NEW, ZONE_3, &[(1, 4)]),
        array_row(BIG_ARRAY_NEW, BIG_2, &[(1, 8_192)]),
        array_row(FILL, INT, &[(1, 16_384)]),
        array_row(OLDS_1, OLD_1, &[(1, 2)]),
    ];
    TypeTrees::resolve(
        ALL_TYPES.map(id),
        &types_for(machine(), PlcPlatform::TwinCat3, rows),
    )
}

/// The key of the parameter `n`.
pub(super) fn key(n: u128) -> ParameterKey {
    ParameterKey::from(uuid::Uuid::from_u128(n))
}

/// A stored parameter with a value and limits.
pub(super) fn param(
    n: u128,
    tag: &str,
    type_n: u128,
    value: Value,
    limits: Vec<StoredLimit>,
) -> StoredParameter {
    let mut stored = stored_parameter(n, tag, id(type_n), None);
    stored.value = value.into();
    stored.limits = limits;
    stored
}

/// A live draft with the given parameters.
pub(super) fn draft_with(parameters: Vec<StoredParameter>) -> Draft {
    Draft::load(stored_draft_with(
        VersionState::LiveDraft,
        PlcPlatform::TwinCat3,
        parameters,
    ))
    .unwrap()
}

/// A limit from 0 to `max` on `path`.
pub(super) fn limit(path: &str, max: i64) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: Some(rust_decimal::Decimal::from(0)),
        max: Some(rust_decimal::Decimal::from(max)),
        max_length: None,
    }
}

/// A request without values for new members.
pub(super) fn request(from: u128, to: u128) -> RebindRequest {
    RebindRequest {
        from: id(from),
        to: id(to),
        new_member_values: BTreeMap::new(),
    }
}

/// A request with values for new members. A pair is parameter number, leaf path, and value.
pub(super) fn request_with(from: u128, to: u128, values: &[(u128, &str, Value)]) -> RebindRequest {
    let mut built = request(from, to);
    for (n, path, value) in values {
        built
            .new_member_values
            .insert((key(*n), LeafPath::parse(path).unwrap()), value.clone());
    }
    built
}

/// Plan a rebind and return the plan. The plan must succeed.
pub(super) fn plan_of(draft: &Draft, request: &RebindRequest) -> RebindPlan {
    draft.plan_rebind(request, &trees()).unwrap()
}

/// The JSON of the value of the plan for parameter `n`.
pub(super) fn planned_json(plan: &RebindPlan, n: u128) -> Value {
    plan.value(key(n)).unwrap().as_json().clone()
}

/// A plain number value.
pub(super) fn number(n: i64) -> Value {
    json!(n)
}
