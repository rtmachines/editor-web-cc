//! Tests of the overlap check after a change of the layout rules (ADR-0149 (a) to (d), design D9).
//!
//! The draft holds two stored overlap pairs on `S7_1500`. `SINT` and `USINT` do not exist on
//! `S7_300`, so the whole draft runs on `S7_1500`. Both pairs overlap under the layout of
//! ADR-0147, although the commands accepted them before. A command reports an overlap only when
//! the pair holds a parameter that the command writes. `ApproveVersion` reports every overlap.
//!
//! - Pair 1: `BYTES` of type `ST_Bytes@1` (6 bytes) at `DB1.DBB0` and `MODE` of type `BYTE` at
//!   `DB1.DBB4`.
//! - Pair 2: `GRID` of type `ARRAY[1..3,1..3] OF BOOL` (4 bytes) at `DB2.DBB0` and `X` of type
//!   `BYTE` at `DB2.DBB2`.
//! - `SPEED` of type `INT` at `DB1.DBW8` and `ZONE` of type `ST_Zone@1` at `DB3.DBB0` overlap
//!   nothing.

use std::collections::BTreeMap;

use super::super::add::support::{
    draft_with, limit_row, new_parameter_at, snapshot, try_add as add,
};
use super::super::support::{address, key, stored_parameter, tag};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{
    array_row, id, machine, on_platform, primitive_row, struct_row, struct_row_revision, types_for,
};
use crate::plant::{PlcPlatform, PlcType, PlcTypeId, Primitive, TypeTrees};
use crate::recipe::draft::add::support::live_unit;
use crate::recipe::{
    Draft, DraftError, LimitList, MemberValue, Ordinal, ParameterKey, RebindRequest, RecipeError,
    StoredParameter, ValueInput,
};
use crate::text::{LongText, Text};

use crate::recipe::DefinitionUpdate;

const INT: u128 = 1;
const BYTE: u128 = 2;
const CHAR: u128 = 3;
const SINT: u128 = 4;
const USINT: u128 = 5;
const ST_BYTES: u128 = 6;
const BOOL: u128 = 7;
const GRID: u128 = 8;
const ZONE_1: u128 = 9;
const ZONE_2: u128 = 10;

const BYTES_N: u128 = 1;
const MODE_N: u128 = 2;
const GRID_N: u128 = 3;
const X_N: u128 = 4;
const SPEED_N: u128 = 5;
const ZONE_N: u128 = 6;

fn rows() -> Vec<PlcType> {
    vec![
        primitive_row(BOOL, Primitive::Bool),
        array_row(GRID, BOOL, &[(1, 3), (1, 3)]),
        primitive_row(INT, Primitive::Int),
        primitive_row(BYTE, Primitive::Byte),
        primitive_row(CHAR, Primitive::Char),
        primitive_row(SINT, Primitive::Sint),
        primitive_row(USINT, Primitive::Usint),
        struct_row(
            ST_BYTES,
            "ST_Bytes",
            &[("a", BYTE), ("b", CHAR), ("c", SINT), ("d", USINT)],
        ),
        struct_row_revision(ZONE_1, "ST_Zone", 1, &[("setpoint", INT)]),
        struct_row_revision(
            ZONE_2,
            "ST_Zone",
            2,
            &[("setpoint", INT), ("temp_max", INT)],
        ),
    ]
}

fn trees() -> TypeTrees {
    let platform = PlcPlatform::S7_1500;
    let ids = [
        INT, BYTE, CHAR, SINT, USINT, ST_BYTES, BOOL, GRID, ZONE_1, ZONE_2,
    ]
    .map(id);
    TypeTrees::resolve(
        ids,
        &types_for(machine(), platform, on_platform(platform, rows())),
    )
}

fn parameter_at(n: u128, tag_text: &str, type_n: u128, address_text: &str) -> StoredParameter {
    stored_parameter(n, tag_text, id(type_n), Some(address(address_text)))
}

fn pair_1() -> Vec<StoredParameter> {
    vec![
        parameter_at(BYTES_N, "BYTES", ST_BYTES, "DB1.DBB0"),
        parameter_at(MODE_N, "MODE", BYTE, "DB1.DBB4"),
    ]
}

fn pair_2() -> Vec<StoredParameter> {
    vec![
        parameter_at(GRID_N, "GRID", GRID, "DB2.DBB0"),
        parameter_at(X_N, "X", BYTE, "DB2.DBB2"),
    ]
}

fn speed() -> StoredParameter {
    parameter_at(SPEED_N, "SPEED", INT, "DB1.DBW8")
}

fn zone() -> StoredParameter {
    let mut stored = parameter_at(ZONE_N, "ZONE", ZONE_1, "DB3.DBB0");
    stored.value = serde_json::json!({"setpoint": 5}).into();
    stored
}

fn draft_of(groups: Vec<Vec<StoredParameter>>) -> Draft {
    draft_with(PlcPlatform::S7_1500, groups.into_iter().flatten().collect())
}

/// The draft of the scenarios: both pairs, `SPEED`, and `ZONE`.
fn both_pairs() -> Draft {
    draft_of(vec![pair_1(), pair_2(), vec![speed(), zone()]])
}

/// The code of a rejected command. The draft must stay as it was.
fn rejected_code(
    draft: &mut Draft,
    run: impl FnOnce(&mut Draft) -> Result<(), DraftError>,
) -> ErrorCode {
    let before = snapshot(draft);
    let error = run(draft).expect_err("the command must fail");
    assert_eq!(
        snapshot(draft),
        before,
        "a failed command changed the draft"
    );
    error.code()
}

/// The tag and the other tag of an overlap error.
fn overlap_tags(error: DraftError) -> Option<(String, String)> {
    match error {
        DraftError::Recipe(RecipeError::S7AddressOverlap { tag, other_tag, .. }) => {
            Some((tag.to_string(), other_tag.to_string()))
        }
        _ => None,
    }
}

fn set_value(draft: &mut Draft, n: u128, json: &str) -> Result<(), DraftError> {
    draft
        .set_value(key(n), &ValueInput::Json(json.to_owned()), &trees())
        .map(|_| ())
}

fn replace_limits(draft: &mut Draft, n: u128) -> Result<(), DraftError> {
    let limits = LimitList::parse(vec![limit_row("", Some("0"), Some("100"))]).unwrap();
    draft.replace_limits(key(n), &limits, &trees()).map(|_| ())
}

fn update_to(tag_text: &str, type_n: u128, address_text: &str) -> DefinitionUpdate {
    DefinitionUpdate {
        unit: live_unit(),
        tag: tag(tag_text),
        name_en: Text::parse("Renamed").unwrap(),
        description_en: LongText::parse("Description").unwrap(),
        plc_type: id(type_n),
        s7_address: Some(address(address_text)),
        ordinal: Ordinal::parse(1).unwrap(),
    }
}

fn move_to(
    draft: &mut Draft,
    n: u128,
    tag_text: &str,
    address_text: &str,
) -> Result<(), DraftError> {
    let update = update_to(tag_text, BYTE, address_text);
    draft
        .set_definition(key(n), update, None, &trees())
        .map(|_| ())
}

/// Scenario "Edit of a parameter outside a stored overlap".
#[test]
fn a_value_and_limits_edit_of_a_parameter_outside_both_pairs_passes() {
    let mut draft = both_pairs();
    assert!(set_value(&mut draft, SPEED_N, "100").is_ok());
    assert!(replace_limits(&mut draft, SPEED_N).is_ok());
    let speed = draft.parameter(key(SPEED_N)).unwrap();
    assert_eq!(speed.value().as_json(), &serde_json::json!(100));
    assert_eq!(speed.limits().len(), 1);
}

/// Scenario "Edit of a parameter of a stored overlap".
#[test]
fn a_value_edit_of_a_parameter_of_a_stored_pair_gives_e_rcp_022() {
    let mut draft = both_pairs();
    let code = rejected_code(&mut draft, |draft| set_value(draft, MODE_N, "3"));
    assert_eq!(code, ErrorCode::RCP_022);
}

/// Scenario "Edit of a parameter of a stored overlap", limits of the other pair member.
#[test]
fn a_limits_edit_of_a_parameter_of_a_stored_pair_gives_e_rcp_022() {
    let mut draft = both_pairs();
    let code = rejected_code(&mut draft, |draft| replace_limits(draft, MODE_N));
    assert_eq!(code, ErrorCode::RCP_022);
}

/// The second parameter of the pair also gets the code: a value edit of `X` gives `E-RCP-022`.
#[test]
fn a_value_edit_of_the_other_parameter_of_a_stored_pair_gives_e_rcp_022() {
    let mut draft = both_pairs();
    let code = rejected_code(&mut draft, |draft| set_value(draft, X_N, "3"));
    assert_eq!(code, ErrorCode::RCP_022);
}

/// Scenario "Move of one parameter of a stored overlap". Pair 2 stays and does not refuse.
#[test]
fn a_move_of_one_parameter_of_a_stored_pair_passes_while_the_other_pair_stays() {
    let mut draft = both_pairs();
    assert!(move_to(&mut draft, MODE_N, "MODE", "DB1.DBB6").is_ok());
    let mode = draft.parameter(key(MODE_N)).unwrap();
    assert_eq!(mode.s7_address(), Some(address("DB1.DBB6")));
}

/// The pair filter skips only a pair without a written parameter. A move onto `SPEED` fails.
#[test]
fn a_move_onto_a_parameter_outside_the_pairs_gives_e_rcp_022() {
    let mut draft = both_pairs();
    let code = rejected_code(&mut draft, |draft| {
        move_to(draft, MODE_N, "MODE", "DB1.DBB8")
    });
    assert_eq!(code, ErrorCode::RCP_022);
}

/// ADR-0149 (d): after the move, pair 2 still refuses the approval.
#[test]
fn the_approval_check_reports_the_pair_that_stays_after_a_move() {
    let mut draft = both_pairs();
    move_to(&mut draft, MODE_N, "MODE", "DB1.DBB6").unwrap();
    let error = draft.check_s7_layout_for_approval(&trees()).unwrap_err();
    assert_eq!(
        overlap_tags(error),
        Some(("X".to_owned(), "GRID".to_owned()))
    );
}

/// ADR-0149 (a): `AddParameter` of a parameter that overlaps nothing passes next to a stored pair.
#[test]
fn an_add_of_a_parameter_outside_both_pairs_passes() {
    let mut draft = both_pairs();
    let new = new_parameter_at(10, "TEMP", INT, "DB1.DBW10");
    assert!(add(&mut draft, &trees(), new).is_ok());
    assert!(draft.parameter(key(10)).is_ok());
}

/// ADR-0149 (b): `AddParameter` of a parameter that overlaps a stored parameter still fails.
#[test]
fn an_add_of_a_parameter_that_overlaps_a_stored_parameter_gives_e_rcp_022() {
    let mut draft = both_pairs();
    let new = new_parameter_at(10, "TEMP", INT, "DB1.DBW8");
    let code = rejected_code(&mut draft, |draft| add(draft, &trees(), new));
    assert_eq!(code, ErrorCode::RCP_022);
}

/// ADR-0149 (b): a rebind of `ZONE`, which is outside both pairs, passes.
#[test]
fn a_rebind_of_a_parameter_outside_both_pairs_passes() {
    let mut draft = both_pairs();
    let trees = trees();
    let mut request = RebindRequest {
        from: id(ZONE_1),
        to: id(ZONE_2),
        new_member_values: BTreeMap::new(),
    };
    request.new_member_values.insert(
        (
            key(ZONE_N),
            crate::plant::LeafPath::parse(".temp_max").unwrap(),
        ),
        MemberValue::Json(serde_json::json!(7)),
    );
    let plan = draft.plan_rebind(&request, &trees).unwrap();
    let new_types: BTreeMap<ParameterKey, PlcTypeId> = BTreeMap::new();
    draft.apply_rebind(plan, &new_types, &trees).unwrap();
    assert_eq!(draft.parameter(key(ZONE_N)).unwrap().plc_type(), id(ZONE_2));
}

/// Scenario "Approval of a draft with a stored overlap".
#[test]
fn the_approval_check_of_a_draft_with_pair_1_gives_e_rcp_022() {
    let draft = draft_of(vec![pair_1(), vec![speed()]]);
    let error = draft.check_s7_layout_for_approval(&trees()).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_022);
    assert_eq!(
        overlap_tags(error),
        Some(("MODE".to_owned(), "BYTES".to_owned()))
    );
}

/// The approval check reports every overlap of the draft, also when no command writes a key.
#[test]
fn the_approval_check_of_a_draft_with_pair_2_only_gives_e_rcp_022() {
    let draft = draft_of(vec![pair_2()]);
    let error = draft.check_s7_layout_for_approval(&trees()).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_022);
}

/// The sweep keeps the item with the largest end. `SMALL` overlaps `BYTES` in a stored pair.
/// `SMALL` ends before the new `BYTE` starts. The new `BYTE` still overlaps `BYTES`, so the command
/// fails.
#[test]
fn an_add_is_found_behind_a_small_stored_overlap_inside_a_large_parameter() {
    let small = parameter_at(20, "SMALL", BYTE, "DB1.DBB2");
    let mut draft = draft_of(vec![vec![
        parameter_at(BYTES_N, "BYTES", ST_BYTES, "DB1.DBB0"),
        small,
    ]]);
    let new = new_parameter_at(10, "WRITTEN", BYTE, "DB1.DBB4");
    let code = rejected_code(&mut draft, |draft| add(draft, &trees(), new));
    assert_eq!(code, ErrorCode::RCP_022);
}
