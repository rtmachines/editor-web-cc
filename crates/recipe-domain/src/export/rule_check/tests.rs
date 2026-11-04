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
use crate::plant::fixtures::{id, machine, primitive_row, types_for};
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
