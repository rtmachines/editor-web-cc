//! Tests of the read check of a stored parameter (ADR-0118, design D7).
//!
//! A `DATE` count of 72 700 is `D#2169-01-17`. The earlier set allows it and the current set
//! refuses it. A `REAL` number with 8 significant digits is the same kind of case.

use rust_decimal::Decimal;
use serde_json::json;

use super::{RuleFault, RuleField, check_stored_parameter};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{
    TIME_TREES_ROOT, id, machine, on_platform, primitive_row, struct_row, time_trees_on, types_for,
};
use crate::plant::{LeafPath, PlcPlatform, Primitive, S7Address, TypeTree, TypeTrees};
use crate::recipe::draft::support::stored_parameter;
use crate::recipe::{Parameter, StoredLimit};

/// The type identifier of the `DATE` row of these tests.
const DATE: u128 = 7;

/// The type identifier of the `REAL` row of these tests.
const REAL: u128 = 8;

/// The stored `DATE` count of `D#2169-01-17`. Only the earlier rule set allows it.
const EARLIER_ONLY_DAY: i64 = 72_700;

/// The stored `DATE` count of `D#2168-12-31`. Both rule sets allow it.
const LAST_DAY: i64 = 72_683;

/// A stored `DATE` count above the end of the earlier rule set. It is a storage defect.
const BEYOND_EARLIER: i64 = 72_900;

/// A `REAL` number with 8 significant digits. IEEE 754 single precision cannot hold it.
const LONG_REAL: i64 = 16_777_217;

/// The type identifier of the `DINT` row of these tests.
const DINT: u128 = 9;

/// The type identifier of the `INT` row of these tests.
const INT: u128 = 10;

/// The type identifier of the struct with the one `DATE` member `due`.
const INNER: u128 = 11;

/// The type identifier of the struct with the two `INNER` members `a` and `b`.
const TWO_DAYS: u128 = 12;

/// The type identifier of the struct with a `DATE` member `day` and a `REAL` member `ratio`.
const MIXED: u128 = 13;

/// The trees of the test types on `S7_1500`.
fn trees() -> TypeTrees {
    trees_on(PlcPlatform::S7_1500)
}

/// The trees of the test types on `platform`.
fn trees_on(platform: PlcPlatform) -> TypeTrees {
    let rows = vec![
        primitive_row(DATE, Primitive::Date),
        primitive_row(REAL, Primitive::Real),
        primitive_row(DINT, Primitive::Dint),
        primitive_row(INT, Primitive::Int),
        struct_row(INNER, "Inner", &[("due", DATE)]),
        struct_row(TWO_DAYS, "TwoDays", &[("a", INNER), ("b", INNER)]),
        struct_row(MIXED, "Mixed", &[("day", DATE), ("ratio", REAL)]),
    ];
    TypeTrees::resolve(
        [
            id(DATE),
            id(REAL),
            id(DINT),
            id(INT),
            id(INNER),
            id(TWO_DAYS),
            id(MIXED),
        ],
        &types_for(machine(), platform, on_platform(platform, rows)),
    )
}

/// The type tree of the test type `n`.
fn tree_of(trees: &TypeTrees, n: u128) -> &TypeTree {
    trees.get(id(n)).unwrap().as_ref().unwrap()
}

/// A limit row on the whole value with the given bounds.
fn bounds(min: Option<Decimal>, max: Option<Decimal>) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min,
        max,
        max_length: None,
    }
}

/// The parameter `PARAM` of the type `n` with the stored value `value` and the stored limits.
fn parameter(n: u128, value: i64, limits: Vec<StoredLimit>) -> Parameter {
    parameter_with(n, json!(value), None, limits)
}

/// The parameter `PARAM` of the type `n` with a stored JSON value, an address, and limits.
fn parameter_with(
    n: u128,
    value: serde_json::Value,
    address: Option<&str>,
    limits: Vec<StoredLimit>,
) -> Parameter {
    let address = address.map(|text| S7Address::parse(text).unwrap());
    let mut stored = stored_parameter(1, "PARAM", id(n), address);
    stored.value = value.into();
    stored.limits = limits;
    Parameter::from(stored)
}

/// The code and the field of each fault, in the order of the list.
fn found(faults: &[RuleFault]) -> Vec<(ErrorCode, RuleField)> {
    faults
        .iter()
        .map(|fault| (fault.code(), fault.field().clone()))
        .collect()
}

#[test]
fn a_stored_part_that_passes_both_rule_sets_gives_no_fault() {
    let trees = trees();
    let limits = vec![bounds(None, Some(Decimal::from(LAST_DAY)))];
    let parameter = parameter(DATE, LAST_DAY, limits);
    let faults = check_stored_parameter(&parameter, tree_of(&trees, DATE)).unwrap();
    assert!(faults.is_empty(), "{faults:?}");
}

#[test]
fn a_stored_date_of_the_earlier_set_only_gives_the_rule_fault_e_rcp_005() {
    let trees = trees();
    let parameter = parameter(DATE, EARLIER_ONLY_DAY, Vec::new());
    let faults = check_stored_parameter(&parameter, tree_of(&trees, DATE)).unwrap();
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert_eq!(faults[0].code(), ErrorCode::RCP_005);
    assert_eq!(faults[0].field(), &RuleField::Value(LeafPath::whole()));
}

#[test]
fn a_stored_limit_max_of_the_earlier_set_only_gives_the_rule_fault_e_rcp_006() {
    let trees = trees();
    let limits = vec![bounds(None, Some(Decimal::from(EARLIER_ONLY_DAY)))];
    let parameter = parameter(DATE, LAST_DAY, limits);
    let faults = check_stored_parameter(&parameter, tree_of(&trees, DATE)).unwrap();
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert_eq!(faults[0].code(), ErrorCode::RCP_006);
    assert_eq!(faults[0].field(), &RuleField::LimitMax(LeafPath::whole()));
}

#[test]
fn a_stored_limit_min_of_the_earlier_set_only_gives_the_rule_fault_on_the_min() {
    let trees = trees();
    let limits = vec![bounds(Some(Decimal::from(EARLIER_ONLY_DAY)), None)];
    let parameter = parameter(DATE, EARLIER_ONLY_DAY, limits);
    let faults = check_stored_parameter(&parameter, tree_of(&trees, DATE)).unwrap();
    let found: Vec<_> = faults
        .iter()
        .map(|f| (f.code(), f.field().clone()))
        .collect();
    assert_eq!(
        found,
        [
            (ErrorCode::RCP_005, RuleField::Value(LeafPath::whole())),
            (ErrorCode::RCP_006, RuleField::LimitMin(LeafPath::whole())),
        ]
    );
}

#[test]
fn a_stored_real_with_8_digits_gives_the_rule_fault_e_rcp_055() {
    let trees = trees();
    let parameter = parameter(REAL, LONG_REAL, Vec::new());
    let faults = check_stored_parameter(&parameter, tree_of(&trees, REAL)).unwrap();
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert_eq!(faults[0].code(), ErrorCode::RCP_055);
    assert_eq!(faults[0].field(), &RuleField::Value(LeafPath::whole()));
}

#[test]
fn a_stored_date_beyond_the_earlier_set_is_a_structure_fault() {
    let trees = trees();
    let parameter = parameter(DATE, BEYOND_EARLIER, Vec::new());
    let fault = check_stored_parameter(&parameter, tree_of(&trees, DATE))
        .expect_err("a count above the earlier end is a storage defect");
    assert_eq!(fault.code(), ErrorCode::RCP_005);
}

#[test]
fn a_stored_real_limit_min_with_8_digits_gives_the_rule_fault_e_rcp_055_on_the_min() {
    let trees = trees();
    let limits = vec![bounds(Some(Decimal::new(-10_000_001, 5)), None)];
    let parameter = parameter(REAL, 0, limits);
    let faults = check_stored_parameter(&parameter, tree_of(&trees, REAL)).unwrap();
    assert_eq!(
        found(&faults),
        [(ErrorCode::RCP_055, RuleField::LimitMin(LeafPath::whole()))]
    );
}

#[test]
fn a_stored_real_limit_max_with_8_digits_gives_the_rule_fault_e_rcp_055_on_the_max() {
    let trees = trees();
    let limits = vec![bounds(None, Some(Decimal::new(10_000_001, 5)))];
    let parameter = parameter(REAL, 0, limits);
    let faults = check_stored_parameter(&parameter, tree_of(&trees, REAL)).unwrap();
    assert_eq!(
        found(&faults),
        [(ErrorCode::RCP_055, RuleField::LimitMax(LeafPath::whole()))]
    );
}

#[test]
fn a_stored_real_row_with_8_digits_in_both_bounds_gives_a_rule_fault_for_each_bound() {
    let trees = trees();
    let limits = vec![bounds(
        Some(Decimal::new(-10_000_001, 5)),
        Some(Decimal::new(10_000_001, 5)),
    )];
    let parameter = parameter(REAL, 0, limits);
    let faults = check_stored_parameter(&parameter, tree_of(&trees, REAL)).unwrap();
    assert_eq!(
        found(&faults),
        [
            (ErrorCode::RCP_055, RuleField::LimitMin(LeafPath::whole())),
            (ErrorCode::RCP_055, RuleField::LimitMax(LeafPath::whole())),
        ]
    );
}

#[test]
fn a_stored_dint_past_the_block_end_gives_the_rule_fault_e_rcp_056_on_the_address() {
    let trees = trees();
    let parameter = parameter_with(DINT, json!(0), Some("DB1.DBD65534"), Vec::new());
    let faults = check_stored_parameter(&parameter, tree_of(&trees, DINT)).unwrap();
    assert_eq!(found(&faults), [(ErrorCode::RCP_056, RuleField::Address)]);
}

#[test]
fn a_stored_data_block_above_16000_on_s7_300_gives_the_rule_fault_e_rcp_057() {
    let trees = trees_on(PlcPlatform::S7_300);
    let parameter = parameter_with(INT, json!(0), Some("DB16001.DBW0"), Vec::new());
    let faults = check_stored_parameter(&parameter, tree_of(&trees, INT)).unwrap();
    assert_eq!(found(&faults), [(ErrorCode::RCP_057, RuleField::Address)]);
}

#[test]
fn a_stored_data_block_above_16000_on_s7_1500_gives_no_fault() {
    let trees = trees();
    let parameter = parameter_with(INT, json!(0), Some("DB16001.DBW0"), Vec::new());
    let faults = check_stored_parameter(&parameter, tree_of(&trees, INT)).unwrap();
    assert!(faults.is_empty(), "{faults:?}");
}

#[test]
fn a_stored_text_for_a_date_leaf_is_a_structure_fault() {
    let trees = trees();
    let parameter = parameter_with(DATE, json!("abc"), None, Vec::new());
    let result = check_stored_parameter(&parameter, tree_of(&trees, DATE));
    assert!(result.is_err(), "a value of the wrong shape is a defect");
}

/// The leaf path of a text such as `.a.due`.
fn leaf(text: &str) -> LeafPath {
    LeafPath::parse(text).unwrap()
}

#[test]
fn two_date_leaves_of_the_earlier_set_only_give_two_rule_faults() {
    let trees = trees();
    let value = json!({"a": {"due": EARLIER_ONLY_DAY}, "b": {"due": EARLIER_ONLY_DAY + 10}});
    let parameter = parameter_with(TWO_DAYS, value, None, Vec::new());
    let faults = check_stored_parameter(&parameter, tree_of(&trees, TWO_DAYS)).unwrap();
    assert_eq!(
        found(&faults),
        [
            (ErrorCode::RCP_005, RuleField::Value(leaf(".a.due"))),
            (ErrorCode::RCP_005, RuleField::Value(leaf(".b.due"))),
        ]
    );
}

#[test]
fn a_date_leaf_and_a_real_leaf_give_the_codes_e_rcp_005_and_e_rcp_055() {
    let trees = trees();
    let value = json!({"day": EARLIER_ONLY_DAY, "ratio": LONG_REAL});
    let parameter = parameter_with(MIXED, value, None, Vec::new());
    let faults = check_stored_parameter(&parameter, tree_of(&trees, MIXED)).unwrap();
    assert_eq!(
        found(&faults),
        [
            (ErrorCode::RCP_005, RuleField::Value(leaf(".day"))),
            (ErrorCode::RCP_055, RuleField::Value(leaf(".ratio"))),
        ]
    );
}

#[test]
fn a_date_beyond_the_earlier_set_in_a_struct_is_a_structure_fault() {
    let trees = trees();
    let value = json!({"day": BEYOND_EARLIER, "ratio": LONG_REAL});
    let parameter = parameter_with(MIXED, value, None, Vec::new());
    let fault = check_stored_parameter(&parameter, tree_of(&trees, MIXED))
        .expect_err("a count above the earlier end is a storage defect");
    assert_eq!(fault.code(), ErrorCode::RCP_005);
}

#[test]
fn a_parameter_with_several_faults_lists_value_then_limit_then_address() {
    let trees = trees_on(PlcPlatform::S7_300);
    let limits = vec![bounds(Some(Decimal::new(-10_000_001, 5)), None)];
    let parameter = parameter_with(REAL, json!(LONG_REAL), Some("DB16001.DBD65534"), limits);
    let faults = check_stored_parameter(&parameter, tree_of(&trees, REAL)).unwrap();
    assert_eq!(
        found(&faults),
        [
            (ErrorCode::RCP_055, RuleField::Value(LeafPath::whole())),
            (ErrorCode::RCP_055, RuleField::LimitMin(LeafPath::whole())),
            (ErrorCode::RCP_057, RuleField::Address),
            (ErrorCode::RCP_056, RuleField::Address),
        ]
    );
}

/// The type identifier of the `S5TIME` row of these tests.
const S5TIME: u128 = TIME_TREES_ROOT;

#[test]
fn a_stored_s5time_limit_max_of_15_ms_is_a_structure_fault() {
    let trees = time_trees_on(PlcPlatform::S7_1500, Primitive::S5Time);
    let limits = vec![bounds(None, Some(Decimal::from(15)))];
    let parameter = parameter(S5TIME, 0, limits);
    let fault = check_stored_parameter(&parameter, tree_of(&trees, S5TIME))
        .expect_err("a count between two S5TIME steps is a storage defect");
    assert_eq!(fault.code(), ErrorCode::RCP_006);
}
