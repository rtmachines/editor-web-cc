//! Shared builders of the tests of `Draft::add_parameter`.

use uuid::Uuid;

use super::super::support::{address, stored_draft_with, stored_parameter, tag};
use crate::plant::fixtures::{
    array_row, id, machine, other_machine, primitive_row, struct_row, types_for,
};
use crate::plant::{
    LeafPath, LiveUnit, Machine, MachineCode, MachineId, PlcPlatform, PlcType, Primitive,
    StoredMachine, StoredUnit, TypeTrees, UnitId, UnitPath,
};
use crate::recipe::{
    Draft, DraftError, LeafTexts, LimitInput, LimitList, NewParameter, Ordinal, Parameter,
    ParameterId, ParameterKey, StoredParameter, ValueInput, VersionState,
};
use crate::text::{LongText, Text};

pub(in crate::recipe::draft) const INT: u128 = 1;
pub(in crate::recipe::draft) const REAL: u128 = 2;
pub(in crate::recipe::draft) const BOOL: u128 = 3;
/// A struct with the members `speed` (INT) and `run` (BOOL).
pub(in crate::recipe::draft) const ST_ZONE: u128 = 4;
pub(in crate::recipe::draft) const DINT: u128 = 5;
/// The array `ARRAY[1..8] OF INT`.
pub(in crate::recipe::draft) const TABLE: u128 = 6;
/// A type identifier that `TypeTrees` was asked for and could not resolve.
pub(in crate::recipe::draft) const UNRESOLVED: u128 = 99;
/// A type identifier that `TypeTrees` was never asked for.
pub(in crate::recipe::draft) const NOT_ASKED: u128 = 98;

fn rows() -> Vec<PlcType> {
    vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(REAL, Primitive::Real),
        primitive_row(BOOL, Primitive::Bool),
        primitive_row(DINT, Primitive::Dint),
        struct_row(ST_ZONE, "ST_Zone", &[("speed", INT), ("run", BOOL)]),
        array_row(TABLE, INT, &[(1, 8)]),
    ]
}

/// The trees of the test types for `owner` on `platform`.
pub(in crate::recipe::draft) fn trees_of(owner: MachineId, platform: PlcPlatform) -> TypeTrees {
    let ids = [INT, REAL, BOOL, DINT, ST_ZONE, TABLE, UNRESOLVED].map(id);
    TypeTrees::resolve(ids, &types_for(owner, platform, rows()))
}

/// The trees of the machine of the test draft on `TwinCat3`, asked for only the types `asked`.
pub(in crate::recipe::draft) fn trees_asking(asked: &[u128]) -> TypeTrees {
    let ids = asked.iter().copied().map(id);
    TypeTrees::resolve(ids, &types_for(machine(), PlcPlatform::TwinCat3, rows()))
}

/// The trees of the machine of the test draft on `platform`.
pub(in crate::recipe::draft) fn trees_on(platform: PlcPlatform) -> TypeTrees {
    trees_of(machine(), platform)
}

/// A live unit `U1` of `owner`.
pub(in crate::recipe::draft) fn live_unit_of(owner: MachineId) -> LiveUnit {
    let path = UnitPath::parse("U1").unwrap();
    let stored = StoredMachine {
        id: owner,
        code: MachineCode::parse("M7").unwrap(),
        name: Text::parse("Press").unwrap(),
        platform: PlcPlatform::TwinCat3,
        archived_at: None,
        units: vec![StoredUnit {
            id: UnitId::from(Uuid::from_u128(700)),
            path: path.clone(),
            name: Text::parse("Unit").unwrap(),
            archived_at: None,
        }],
    };
    Machine::try_from(stored)
        .unwrap()
        .live_units()
        .require(&path)
        .unwrap()
}

/// A live unit of the machine of the test draft.
pub(in crate::recipe::draft) fn live_unit() -> LiveUnit {
    live_unit_of(machine())
}

/// A live unit of another machine.
pub(in crate::recipe::draft) fn foreign_unit() -> LiveUnit {
    live_unit_of(other_machine())
}

/// A new parameter with the value `value_json`, no address, and no limits.
pub(in crate::recipe::draft) fn new_parameter(
    n: u128,
    tag_text: &str,
    type_n: u128,
    value_json: &str,
) -> NewParameter {
    NewParameter {
        id: ParameterId::from(Uuid::from_u128(n)),
        key: ParameterKey::from(Uuid::from_u128(n)),
        unit: live_unit(),
        tag: tag(tag_text),
        name_en: Text::parse("Name").unwrap(),
        description_en: LongText::parse("Description").unwrap(),
        plc_type: id(type_n),
        s7_address: None,
        ordinal: Ordinal::parse(u32::try_from(n).unwrap()).unwrap(),
        value: ValueInput::Json(value_json.to_owned()),
        limits: LimitList::parse(Vec::new()).unwrap(),
    }
}

/// The value input of a form: one text per leaf path.
pub(in crate::recipe::draft) fn texts_input(pairs: &[(&str, &str)]) -> ValueInput {
    let pairs = pairs
        .iter()
        .map(|(path, text)| (LeafPath::parse(path).unwrap(), (*text).to_owned()))
        .collect();
    ValueInput::Texts(LeafTexts::parse(pairs).unwrap())
}

/// A new parameter with an address.
pub(in crate::recipe::draft) fn new_parameter_at(
    n: u128,
    tag_text: &str,
    type_n: u128,
    address_text: &str,
) -> NewParameter {
    NewParameter {
        s7_address: Some(address(address_text)),
        ..new_parameter(n, tag_text, type_n, "0")
    }
}

/// A live draft with the given stored parameters.
pub(in crate::recipe::draft) fn draft_with(
    platform: PlcPlatform,
    parameters: Vec<StoredParameter>,
) -> Draft {
    Draft::load(stored_draft_with(
        VersionState::LiveDraft,
        platform,
        parameters,
    ))
    .unwrap()
}

/// A live draft on `TwinCat3` with the parameter `SPEED` of type INT. Its key derives from 1.
pub(in crate::recipe::draft) fn draft_with_speed() -> Draft {
    draft_with(
        PlcPlatform::TwinCat3,
        vec![stored_parameter(1, "SPEED", id(INT), None)],
    )
}

/// What a failed command must leave behind.
pub(in crate::recipe::draft) fn snapshot(draft: &Draft) -> (Vec<Parameter>, i64) {
    let parameters = draft.parameters().into_iter().cloned().collect();
    (parameters, draft.row_version())
}

/// Run the command under test and drop the change.
pub(in crate::recipe::draft) fn try_add(
    draft: &mut Draft,
    trees: &TypeTrees,
    new: NewParameter,
) -> Result<(), DraftError> {
    draft.add_parameter(new, trees).map(|_| ())
}

/// A limit row for the leaf at `path`, with the bounds `min` and `max` as text.
pub(in crate::recipe::draft) fn limit_row(
    path: &str,
    min: Option<&str>,
    max: Option<&str>,
) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: min.map(str::to_owned),
        max: max.map(str::to_owned),
        max_length: None,
    }
}

/// The stored parameters `P1` to `P<count>` of type INT without an address.
///
/// The keys and the identifiers start at 10 000, so they never meet the keys of the new
/// parameters of the tests.
pub(in crate::recipe::draft) fn many_parameters(count: u32) -> Vec<StoredParameter> {
    (1..=count)
        .map(|index| {
            let n = u128::from(index).checked_add(10_000).unwrap();
            stored_parameter(n, &format!("P{index}"), id(INT), None)
        })
        .collect()
}

/// Run `try_add` and assert that it fails with `code` and leaves the draft unchanged.
pub(in crate::recipe::draft) fn assert_rejected(
    draft: &mut Draft,
    trees: &TypeTrees,
    new: NewParameter,
    code: crate::error_codes::ErrorCode,
) -> DraftError {
    let before = snapshot(draft);
    let error = try_add(draft, trees, new).expect_err("the command must fail");
    assert_eq!(error.code(), code, "{error}");
    assert_eq!(
        snapshot(draft),
        before,
        "a failed command changed the draft"
    );
    error
}
