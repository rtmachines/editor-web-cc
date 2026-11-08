//! Tests of the rule check of the parameters of one version (ADR-0119, design D8).
//!
//! The machine is on `S7_1500`, except where a test names `S7_300`. A `DATE` count of 72 700 is
//! `D#2169-01-17`. The earlier rule set allows it and the current rule set refuses it. A `REAL`
//! number with 8 significant digits is the same kind of case. An address that ends after byte
//! 65 535 or that uses a data block above 16 000 on `S7_300` is a third kind of case.

use rust_decimal::Decimal;
use serde_json::json;

use super::check_version_parameters;
use crate::error_codes::ErrorCode;
use crate::export::test_rows::stored_parameter;
use crate::export::test_support::INT;
use crate::plant::fixtures::{
    TIME_TREES_ROOT, array_row, id, machine, on_platform, primitive_row, struct_row, time_trees_on,
    types_for,
};
use crate::plant::{LeafPath, PlcPlatform, Primitive, S7Address, TypeTrees};
use crate::recipe::{Parameter, ParameterKey, RuleFault, RuleField, StoredLimit};

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
const SHORT: u128 = 10;

/// A type identifier that no row of these tests holds.
const UNKNOWN: u128 = 99;

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
        primitive_row(SHORT, Primitive::Int),
    ];
    TypeTrees::resolve(
        [id(DATE), id(REAL), id(DINT), id(SHORT)],
        &types_for(machine(), platform, rows),
    )
}

/// The parameter `n` of the type `plc_type` with the value 0 and the S7 address `address`.
fn parameter_at(n: u128, plc_type: u128, address: &str) -> Parameter {
    let mut stored = stored_parameter(n, ("U1", &format!("P{n}")), INT, json!(0));
    stored.plc_type = id(plc_type);
    stored.s7_address = Some(S7Address::parse(address).unwrap());
    Parameter::from(stored)
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

/// The parameter `n` of the type `plc_type` with a stored value and stored limits.
fn parameter(n: u128, plc_type: u128, value: i64, limits: Vec<StoredLimit>) -> Parameter {
    let mut stored = stored_parameter(n, ("U1", &format!("P{n}")), INT, json!(value));
    stored.plc_type = id(plc_type);
    stored.limits = limits;
    Parameter::from(stored)
}

/// A parameter that passes both rule sets. It must never appear in a result.
fn clean_parameter() -> Parameter {
    parameter(1, DATE, LAST_DAY, Vec::new())
}

/// The key, the code, and the field of each fault, in the order of the result.
fn found(result: &[(ParameterKey, Vec<RuleFault>)]) -> Vec<(ParameterKey, ErrorCode, RuleField)> {
    result
        .iter()
        .flat_map(|(key, faults)| {
            faults
                .iter()
                .map(|fault| (*key, fault.code(), fault.field().clone()))
        })
        .collect()
}

#[test]
fn a_stored_date_of_the_earlier_set_only_gives_a_rule_fault() {
    let trees = trees();
    let faulty = parameter(2, DATE, EARLIER_ONLY_DAY, Vec::new());
    let parameters = vec![clean_parameter(), faulty.clone()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert_eq!(
        found(&result),
        vec![(
            faulty.key(),
            ErrorCode::RCP_005,
            RuleField::Value(LeafPath::whole())
        )]
    );
}

#[test]
fn a_stored_limit_max_of_the_earlier_set_only_gives_a_rule_fault() {
    let trees = trees();
    let limits = vec![bounds(None, Some(Decimal::from(EARLIER_ONLY_DAY)))];
    let faulty = parameter(2, DATE, LAST_DAY, limits);
    let parameters = vec![clean_parameter(), faulty.clone()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert_eq!(
        found(&result),
        vec![(
            faulty.key(),
            ErrorCode::RCP_006,
            RuleField::LimitMax(LeafPath::whole())
        )]
    );
}

#[test]
fn a_stored_real_with_8_digits_gives_a_rule_fault() {
    let trees = trees();
    let faulty = parameter(2, REAL, LONG_REAL, Vec::new());
    let parameters = vec![clean_parameter(), faulty.clone()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert_eq!(
        found(&result),
        vec![(
            faulty.key(),
            ErrorCode::RCP_055,
            RuleField::Value(LeafPath::whole())
        )]
    );
}

#[test]
fn a_stored_real_limit_min_with_8_digits_gives_a_rule_fault() {
    let trees = trees();
    let limits = vec![bounds(Some(Decimal::new(-10_000_001, 5)), None)];
    let faulty = parameter(2, REAL, 0, limits);
    let parameters = vec![clean_parameter(), faulty.clone()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert_eq!(
        found(&result),
        vec![(
            faulty.key(),
            ErrorCode::RCP_055,
            RuleField::LimitMin(LeafPath::whole())
        )]
    );
}

#[test]
fn a_stored_dint_past_the_block_end_gives_a_rule_fault_e_rcp_056_on_the_address() {
    let trees = trees_on(PlcPlatform::S7_300);
    let faulty = parameter_at(2, DINT, "DB1.DBD65534");
    let parameters = vec![clean_parameter(), faulty.clone()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert_eq!(
        found(&result),
        vec![(faulty.key(), ErrorCode::RCP_056, RuleField::Address)]
    );
}

#[test]
fn a_stored_data_block_above_16000_on_s7_300_gives_a_rule_fault_e_rcp_057_on_the_address() {
    let trees = trees_on(PlcPlatform::S7_300);
    let faulty = parameter_at(2, SHORT, "DB16001.DBW0");
    let parameters = vec![clean_parameter(), faulty.clone()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert_eq!(
        found(&result),
        vec![(faulty.key(), ErrorCode::RCP_057, RuleField::Address)]
    );
}

#[test]
fn a_stored_data_block_above_16000_on_s7_1500_gives_no_fault() {
    let trees = trees();
    let parameters = vec![clean_parameter(), parameter_at(2, SHORT, "DB16001.DBW0")];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert!(result.is_empty(), "{:?}", found(&result));
}

#[test]
fn the_result_keeps_the_order_of_the_parameters() {
    let trees = trees_on(PlcPlatform::S7_300);
    let past_end = parameter_at(3, DINT, "DB1.DBD65534");
    let block = parameter_at(2, SHORT, "DB16001.DBW0");
    let parameters = vec![past_end.clone(), clean_parameter(), block.clone()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert_eq!(
        found(&result),
        vec![
            (past_end.key(), ErrorCode::RCP_056, RuleField::Address),
            (block.key(), ErrorCode::RCP_057, RuleField::Address),
        ]
    );
}

#[test]
fn a_stored_date_beyond_the_earlier_set_is_a_structure_fault() {
    let trees = trees();
    let parameters = vec![
        clean_parameter(),
        parameter(2, DATE, BEYOND_EARLIER, Vec::new()),
    ];
    let broken = check_version_parameters(&parameters, &trees)
        .expect_err("a count above the earlier end is a storage defect");
    assert_eq!(broken.key(), parameters[1].key());
    assert_eq!(broken.fault().code(), ErrorCode::RCP_005);
}

#[test]
fn the_first_structure_fault_stops_the_check() {
    let trees = trees();
    let parameters = vec![
        parameter(2, DATE, BEYOND_EARLIER, Vec::new()),
        parameter(3, DATE, BEYOND_EARLIER + 1, Vec::new()),
    ];
    let broken = check_version_parameters(&parameters, &trees).unwrap_err();
    assert_eq!(broken.key(), parameters[0].key());
    assert_eq!(broken.code(), ErrorCode::RCP_005);
    assert!(
        broken.fault().error().to_string().contains("P2"),
        "{}",
        broken.fault().error()
    );
}

#[test]
fn a_broken_parameter_gives_the_code_of_its_structure_fault() {
    let trees = trees();
    let parameters = vec![parameter(2, DATE, BEYOND_EARLIER, Vec::new())];
    let broken = check_version_parameters(&parameters, &trees).unwrap_err();
    assert_eq!(broken.code(), broken.fault().code());
    assert_eq!(broken.code(), ErrorCode::RCP_005);
}

#[test]
fn a_broken_parameter_gives_its_key_and_fault_by_value() {
    let trees = trees();
    let parameters = vec![parameter(2, DATE, BEYOND_EARLIER, Vec::new())];
    let broken = check_version_parameters(&parameters, &trees).unwrap_err();
    let (key, fault) = broken.into_parts();
    assert_eq!(key, parameters[0].key());
    assert_eq!(fault.code(), ErrorCode::RCP_005);
}

#[test]
fn a_parameter_that_the_trees_do_not_hold_is_a_structure_fault_with_its_key() {
    let trees = trees();
    let parameters = vec![clean_parameter(), parameter(2, UNKNOWN, 0, Vec::new())];
    let broken = check_version_parameters(&parameters, &trees)
        .expect_err("a parameter without a tree is a storage defect");
    assert_eq!(broken.key(), parameters[1].key());
    assert_eq!(broken.fault().code(), ErrorCode::PLT_013);
}

#[test]
fn a_parameter_whose_type_failed_to_resolve_is_a_structure_fault_with_its_key() {
    let rows = vec![primitive_row(DATE, Primitive::Date)];
    let trees = TypeTrees::resolve(
        [id(DATE), id(UNKNOWN)],
        &types_for(machine(), PlcPlatform::S7_1500, rows),
    );
    let parameters = vec![clean_parameter(), parameter(2, UNKNOWN, 0, Vec::new())];
    let broken = check_version_parameters(&parameters, &trees)
        .expect_err("a type that does not resolve is a storage defect");
    assert_eq!(broken.key(), parameters[1].key());
    assert_eq!(broken.fault().code(), ErrorCode::PLT_013);
}

/// The type identifier of the `S5TIME` row of the test below.
const S5TIME: u128 = TIME_TREES_ROOT;

#[test]
fn a_stored_s5time_limit_max_of_15_ms_is_a_structure_fault_with_its_key() {
    let trees = time_trees_on(PlcPlatform::S7_1500, Primitive::S5Time);
    let limits = vec![bounds(None, Some(Decimal::from(15)))];
    let parameters = vec![parameter(2, S5TIME, 0, limits)];
    let broken = check_version_parameters(&parameters, &trees)
        .expect_err("a count between two S5TIME steps is a storage defect");
    assert_eq!(broken.key(), parameters[0].key());
    assert_eq!(broken.fault().code(), ErrorCode::RCP_006);
}

/// The stored `LDT` count of `LDT#2300-01-01-00:00:00`. Only the earlier rule set allows it
/// (ADR-0139).
const LDT_AFTER_CURRENT_END: &str = "10413792000000000000";

#[test]
fn adr_0139_a_stored_ldt_after_the_current_end_on_twincat3_makes_the_version_not_loadable() {
    let trees = time_trees_on(PlcPlatform::TwinCat3, Primitive::Ldt);
    let count: serde_json::Value = serde_json::from_str(LDT_AFTER_CURRENT_END).unwrap();
    let mut stored = stored_parameter(2, ("U1", "STAMP"), INT, count);
    stored.plc_type = id(TIME_TREES_ROOT);
    let faulty = Parameter::from(stored);
    let result = check_version_parameters(std::slice::from_ref(&faulty), &trees)
        .expect("the earlier set allows the count, so it is no structure fault");
    assert_eq!(
        found(&result),
        vec![(
            faulty.key(),
            ErrorCode::RCP_005,
            RuleField::Value(LeafPath::whole())
        )]
    );
    let is_loadable = result.is_empty();
    assert!(!is_loadable, "a rule fault makes the version not loadable");
}

#[test]
fn adr_0140_a_stored_data_block_above_59999_on_s7_1500_makes_the_version_not_loadable() {
    let trees = trees();
    let faulty = parameter_at(2, SHORT, "DB60000.DBW0");
    let parameters = vec![clean_parameter(), faulty.clone()];
    let result = check_version_parameters(&parameters, &trees)
        .expect("the grammar allows the number, so it is no structure fault");
    assert_eq!(
        found(&result),
        vec![(faulty.key(), ErrorCode::RCP_057, RuleField::Address)]
    );
    let is_loadable = result.is_empty();
    assert!(!is_loadable, "a rule fault makes the version not loadable");
}

// ADR-0149 (e) to (j), design D10. The two stored overlap pairs are the pairs of the tests of the
// draft in `recipe/draft/s7/written_overlap_tests.rs`. They overlap under the layout of ADR-0147.

const BYTE: u128 = 21;
const CHAR: u128 = 22;
const SINT: u128 = 23;
const USINT: u128 = 24;
const BOOL: u128 = 25;
const ST_BYTES: u128 = 26;
const GRID: u128 = 27;

/// The trees of the overlap tests on `S7_1500`. `ST_Bytes@1` has 6 bytes and `GRID` has 4 bytes.
///
/// `SINT` and `USINT` do not exist on `S7_300`, so the whole world runs on `S7_1500`.
fn overlap_trees() -> TypeTrees {
    let rows = vec![
        primitive_row(SHORT, Primitive::Int),
        primitive_row(BYTE, Primitive::Byte),
        primitive_row(CHAR, Primitive::Char),
        primitive_row(SINT, Primitive::Sint),
        primitive_row(USINT, Primitive::Usint),
        primitive_row(BOOL, Primitive::Bool),
        struct_row(
            ST_BYTES,
            "ST_Bytes",
            &[("a", BYTE), ("b", CHAR), ("c", SINT), ("d", USINT)],
        ),
        array_row(GRID, BOOL, &[(1, 3), (1, 3)]),
    ];
    let platform = PlcPlatform::S7_1500;
    let ids = [SHORT, BYTE, CHAR, SINT, USINT, BOOL, ST_BYTES, GRID].map(id);
    TypeTrees::resolve(
        ids,
        &types_for(machine(), platform, on_platform(platform, rows)),
    )
}

/// The parameter `n` with the tag `tag`, the type `plc_type`, the value `value`, and the address.
fn valued_at(
    n: u128,
    tag: &str,
    plc_type: u128,
    value: serde_json::Value,
    address: &str,
) -> Parameter {
    let mut stored = stored_parameter(n, ("U1", tag), INT, value);
    stored.plc_type = id(plc_type);
    stored.s7_address = Some(S7Address::parse(address).unwrap());
    Parameter::from(stored)
}

/// `BYTES` of the type `ST_Bytes@1` at `DB1.DBB0`. The type has 6 bytes.
fn bytes() -> Parameter {
    let value = json!({"a": 0, "b": "A", "c": 0, "d": 0});
    valued_at(1, "BYTES", ST_BYTES, value, "DB1.DBB0")
}

/// `MODE` of the type `BYTE` at `DB1.DBB4`. It lies inside `BYTES`.
fn mode() -> Parameter {
    valued_at(2, "MODE", BYTE, json!(0), "DB1.DBB4")
}

/// `GRID` of the type `ARRAY[1..3,1..3] OF BOOL` at `DB2.DBB0`. The type has 4 bytes.
fn grid() -> Parameter {
    let value = json!([
        [false, false, false],
        [false, false, false],
        [false, false, false]
    ]);
    valued_at(3, "GRID", GRID, value, "DB2.DBB0")
}

/// `X` of the type `BYTE` at `DB2.DBB2`. It lies inside `GRID`.
fn x() -> Parameter {
    valued_at(4, "X", BYTE, json!(0), "DB2.DBB2")
}

/// `SPEED` of the type `INT` at `DB1.DBW8`. It overlaps nothing.
fn speed() -> Parameter {
    valued_at(5, "SPEED", SHORT, json!(0), "DB1.DBW8")
}

#[test]
fn adr_0149_a_stored_overlap_pair_gives_a_rule_fault_e_rcp_022_on_the_address_of_both() {
    let trees = overlap_trees();
    let parameters = vec![bytes(), mode(), speed()];
    let result = check_version_parameters(&parameters, &trees)
        .expect("an overlap is a rule fault and no structure fault");
    assert_eq!(
        found(&result),
        vec![
            (parameters[0].key(), ErrorCode::RCP_022, RuleField::Address),
            (parameters[1].key(), ErrorCode::RCP_022, RuleField::Address),
        ]
    );
    let is_loadable = result.is_empty();
    assert!(!is_loadable, "a rule fault makes the version not loadable");
}

#[test]
fn adr_0149_two_stored_overlap_pairs_give_the_fault_for_all_four_parameters_in_parameter_order() {
    let trees = overlap_trees();
    let parameters = vec![grid(), speed(), bytes(), x(), mode()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    let keys: Vec<_> = result.iter().map(|(key, _)| *key).collect();
    assert_eq!(
        keys,
        vec![
            parameters[0].key(),
            parameters[2].key(),
            parameters[3].key(),
            parameters[4].key(),
        ]
    );
    assert_eq!(
        found(&result)
            .into_iter()
            .map(|(_, code, field)| (code, field))
            .collect::<Vec<_>>(),
        vec![(ErrorCode::RCP_022, RuleField::Address); 4]
    );
}

#[test]
fn adr_0149_only_the_two_parameters_of_the_pair_get_the_fault_and_parameters_apart_get_none() {
    let trees = overlap_trees();
    let low = valued_at(6, "LOW", SHORT, json!(0), "DB1.DBW12");
    let high = valued_at(7, "HIGH", SHORT, json!(0), "DB1.DBW14");
    let parameters = vec![bytes(), mode(), low.clone(), high.clone(), speed()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    let keys: Vec<_> = result.iter().map(|(key, _)| *key).collect();
    assert_eq!(keys, vec![parameters[0].key(), parameters[1].key()]);
    assert!(!keys.contains(&low.key()) && !keys.contains(&high.key()));
}

#[test]
fn adr_0149_a_parameter_after_a_narrow_one_inside_a_wide_one_still_gets_the_fault() {
    let trees = overlap_trees();
    let small = valued_at(2, "SMALL", BYTE, json!(0), "DB1.DBB2");
    let late = valued_at(6, "LATE", BYTE, json!(0), "DB1.DBB4");
    let parameters = vec![bytes(), small, late];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert_eq!(
        found(&result),
        vec![
            (parameters[0].key(), ErrorCode::RCP_022, RuleField::Address),
            (parameters[1].key(), ErrorCode::RCP_022, RuleField::Address),
            (parameters[2].key(), ErrorCode::RCP_022, RuleField::Address),
        ]
    );
    let (_, faults) = &result[2];
    let message = faults[0].error().to_string();
    assert!(message.contains("BYTES"), "{message}");
}

#[test]
fn adr_0149_the_fault_names_the_other_parameter_of_the_pair() {
    let trees = overlap_trees();
    let parameters = vec![bytes(), mode()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    let message = |position: usize| {
        let (_, faults) = &result[position];
        faults[0].error().to_string()
    };
    assert!(message(0).contains("MODE"), "{}", message(0));
    assert!(message(1).contains("BYTES"), "{}", message(1));
}

#[test]
fn adr_0149_an_overlap_keeps_the_fault_of_the_end_rule_of_the_same_parameter() {
    let trees = overlap_trees();
    let wide = valued_at(
        1,
        "BYTES",
        ST_BYTES,
        json!({"a": 0, "b": "A", "c": 0, "d": 0}),
        "DB1.DBB65532",
    );
    let inside = valued_at(2, "MODE", BYTE, json!(0), "DB1.DBB65534");
    let parameters = vec![wide, inside];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert_eq!(
        found(&result),
        vec![
            (parameters[0].key(), ErrorCode::RCP_056, RuleField::Address),
            (parameters[0].key(), ErrorCode::RCP_022, RuleField::Address),
            (parameters[1].key(), ErrorCode::RCP_022, RuleField::Address),
        ]
    );
}

#[test]
fn adr_0149_a_version_without_an_overlap_gives_no_new_fault() {
    let trees = overlap_trees();
    let apart = valued_at(2, "MODE", BYTE, json!(0), "DB1.DBB6");
    let other_block = valued_at(4, "X", BYTE, json!(0), "DB2.DBB4");
    let parameters = vec![bytes(), apart, grid(), other_block, speed()];
    let result = check_version_parameters(&parameters, &trees).unwrap();
    assert!(result.is_empty(), "{:?}", found(&result));
}

#[test]
fn adr_0149_two_bits_of_one_byte_do_not_overlap() {
    let trees = overlap_trees();
    let first = valued_at(1, "FLAG1", BOOL, json!(false), "DB1.DBX0.0");
    let second = valued_at(2, "FLAG2", BOOL, json!(false), "DB1.DBX0.1");
    let result = check_version_parameters(&[first, second], &trees).unwrap();
    assert!(result.is_empty(), "{:?}", found(&result));
}

#[test]
fn adr_0149_an_overlap_gives_no_broken_parameter() {
    let trees = overlap_trees();
    let parameters = vec![bytes(), mode(), grid(), x()];
    let result = check_version_parameters(&parameters, &trees);
    assert!(
        result.is_ok(),
        "{:?}",
        result.err().map(|broken| broken.code())
    );
}
