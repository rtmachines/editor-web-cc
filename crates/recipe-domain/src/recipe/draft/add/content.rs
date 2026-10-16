//! Tests of the second part of `Draft::add_parameter` (design D6, steps 2 and 3).

use rust_decimal::Decimal;
use serde_json::json;

use super::super::support::{address, stored_parameter};
use super::support::{
    BOOL, INT, ST_ZONE, TABLE, assert_rejected, draft_with, draft_with_speed, limit_row,
    many_parameters, new_parameter, new_parameter_at, snapshot, texts_input, trees_on,
};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::id;
use crate::plant::{LeafPath, PlcPlatform};
use crate::recipe::{Draft, DraftError, LimitList, NewParameter, StoredLimit};

fn limited(value_json: &str, rows: Vec<crate::recipe::LimitInput>) -> NewParameter {
    NewParameter {
        limits: LimitList::parse(rows).unwrap(),
        ..new_parameter(2, "TEMP", INT, value_json)
    }
}

fn reject(new: NewParameter, code: ErrorCode) -> String {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    assert_rejected(&mut draft, &trees, new, code).to_string()
}

/// The message of each entry of a rejected command that returned a list.
fn entry_texts(error: DraftError) -> Vec<String> {
    match error {
        DraftError::List(list) => Some(list.into_vec()),
        DraftError::Recipe(_) | DraftError::Plant(_) => None,
    }
    .unwrap()
    .iter()
    .map(ToString::to_string)
    .collect()
}

#[test]
fn two_bad_leaves_and_two_bad_limit_rows_give_one_list_in_order() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let rows = vec![
        limit_row(".x", None, Some("5")),
        limit_row("[*]", Some("10"), Some("5")),
    ];
    let new = NewParameter {
        value: texts_input(&[
            ("[1]", "x"),
            ("[2]", "2"),
            ("[3]", "1,5"),
            ("[4]", "4"),
            ("[5]", "5"),
            ("[6]", "6"),
            ("[7]", "7"),
            ("[8]", "8"),
        ]),
        limits: LimitList::parse(rows).unwrap(),
        ..new_parameter(2, "TABLE", TABLE, "0")
    };
    let error = assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_005);
    assert_eq!(
        entry_texts(error),
        [
            "Value of TABLE[1] does not match the type. Expected an integer of type INT.",
            "Value of TABLE[3] does not match the type. Expected an integer of type INT.",
            "Path .x does not exist in the type of TABLE.",
            "The minimum 10 of TABLE[*] is greater than the maximum 5.",
        ]
    );
}

#[test]
fn a_bad_leaf_keeps_the_range_faults_of_the_other_leaves_in_preorder() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let rows = vec![
        limit_row("[*]", Some("0"), Some("3000")),
        limit_row(".x", None, Some("5")),
    ];
    let new = NewParameter {
        value: texts_input(&[
            ("[1]", "3500"),
            ("[2]", "2"),
            ("[3]", "x"),
            ("[4]", "4"),
            ("[5]", "4000"),
            ("[6]", "6"),
            ("[7]", "7"),
            ("[8]", "8"),
        ]),
        limits: LimitList::parse(rows).unwrap(),
        ..new_parameter(2, "TABLE", TABLE, "0")
    };
    let error = assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_004);
    assert_eq!(
        entry_texts(error),
        [
            "Value 3500 of TABLE[1] is outside the process limit 0 to 3000.",
            "Value of TABLE[3] does not match the type. Expected an integer of type INT.",
            "Value 4000 of TABLE[5] is outside the process limit 0 to 3000.",
            "Path .x does not exist in the type of TABLE.",
        ]
    );
}

#[test]
fn a_single_fault_of_a_new_parameter_stays_a_list() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = new_parameter(2, "TEMP", INT, "\"fast\"");
    let error = assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_005);
    assert_eq!(entry_texts(error).len(), 1);
}

#[test]
fn a_value_above_the_process_limit_gives_e_rcp_004() {
    let mut draft = draft_with(PlcPlatform::TwinCat3, Vec::new());
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = NewParameter {
        tag: super::super::support::tag("SPEED"),
        ..limited("150", vec![limit_row("", Some("0"), Some("100"))])
    };
    let error = assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_004);
    assert_eq!(
        error.to_string(),
        "Value 150 of SPEED is outside the process limit 0 to 100."
    );
}

#[test]
fn a_json_string_for_an_int_gives_e_rcp_005() {
    let message = reject(
        new_parameter(2, "TEMP", INT, "\"fast\""),
        ErrorCode::RCP_005,
    );
    assert!(
        message.starts_with("Value of TEMP does not match the type. Expected "),
        "{message}"
    );
}

#[test]
fn a_member_of_the_wrong_form_names_the_member_in_e_rcp_005() {
    let new = new_parameter(2, "ZONE", ST_ZONE, r#"{"speed":"x","run":true}"#);
    let message = reject(new, ErrorCode::RCP_005);
    assert!(
        message.starts_with("Value of ZONE.speed does not match the type. Expected "),
        "{message}"
    );
}

#[test]
fn a_maximum_above_the_int_range_gives_e_rcp_006() {
    let message = reject(
        limited("1", vec![limit_row("", None, Some("40000"))]),
        ErrorCode::RCP_006,
    );
    assert_eq!(
        message,
        "Process limit of TEMP is outside the range of the data type."
    );
}

#[test]
fn a_limit_on_a_missing_member_gives_e_rcp_017() {
    let new = NewParameter {
        limits: LimitList::parse(vec![limit_row(".setpoint", None, Some("5"))]).unwrap(),
        ..new_parameter(2, "ZONE", ST_ZONE, r#"{"speed":1,"run":true}"#)
    };
    let message = reject(new, ErrorCode::RCP_017);
    assert_eq!(
        message,
        "Path .setpoint does not exist in the type of ZONE."
    );
}

#[test]
fn a_minimum_above_the_maximum_gives_e_rcp_018() {
    let message = reject(
        limited("7", vec![limit_row("", Some("10"), Some("5"))]),
        ErrorCode::RCP_018,
    );
    assert_eq!(
        message,
        "The minimum 10 of TEMP is greater than the maximum 5."
    );
}

#[test]
fn a_minimum_on_a_bool_gives_e_rcp_019() {
    let new = NewParameter {
        limits: LimitList::parse(vec![limit_row("", Some("0"), None)]).unwrap(),
        ..new_parameter(2, "RUN", BOOL, "true")
    };
    let message = reject(new, ErrorCode::RCP_019);
    assert!(
        message.starts_with("RUN does not accept this limit."),
        "{message}"
    );
}

#[test]
fn the_4001st_parameter_gives_e_rcp_021() {
    let mut draft = draft_with(PlcPlatform::TwinCat3, many_parameters(4_000));
    let trees = trees_on(PlcPlatform::TwinCat3);
    let error = assert_rejected(
        &mut draft,
        &trees,
        new_parameter(2, "TEMP", INT, "0"),
        ErrorCode::RCP_021,
    );
    assert_eq!(
        error.to_string(),
        "The version exceeds the limit of 4000 parameters. Current count: 4001."
    );
}

#[test]
fn the_4000th_parameter_passes() {
    let mut draft = draft_with(PlcPlatform::TwinCat3, many_parameters(3_999));
    let trees = trees_on(PlcPlatform::TwinCat3);
    let result = draft.add_parameter(new_parameter(2, "TEMP", INT, "0"), &trees);
    assert!(result.is_ok());
    assert_eq!(draft.parameters().len(), 4_000);
}

#[test]
fn the_limit_rows_of_the_draft_and_the_new_parameter_count_together() {
    let mut stored = stored_parameter(1, "SPEED", id(INT), None);
    stored.limits = (0..32_768)
        .map(|_| StoredLimit {
            leaf_path: LeafPath::parse("").unwrap(),
            min: None,
            max: None,
            max_length: None,
        })
        .collect();
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![stored]);
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = limited("1", vec![limit_row("", None, Some("5"))]);
    let error = assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_021);
    assert_eq!(
        error.to_string(),
        "The version exceeds the limit of 32768 limit rows. Current count: 32769."
    );
}

#[test]
fn an_address_that_overlaps_another_parameter_gives_e_rcp_022() {
    let mut draft = draft_with(
        PlcPlatform::S7_1200,
        vec![stored_parameter(
            1,
            "SPEED",
            id(INT),
            Some(address("DB1.DBW0")),
        )],
    );
    let trees = trees_on(PlcPlatform::S7_1200);
    let error = assert_rejected(
        &mut draft,
        &trees,
        new_parameter_at(2, "TEMP", INT, "DB1.DBW0"),
        ErrorCode::RCP_022,
    );
    assert_eq!(
        error.to_string(),
        "Address DB1.DBW0 of TEMP overlaps with SPEED."
    );
}

#[test]
fn a_valid_parameter_is_added_with_the_canonical_value_and_the_stored_limits() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = limited("1.5e2", vec![limit_row("", Some("0"), Some("200"))]);
    let expected_id = new.id;
    let expected_key = new.key;
    let expected_unit = new.unit.id();

    let change = draft.add_parameter(new, &trees).unwrap();

    assert!(change.before().is_none());
    let after = change.after().unwrap();
    assert_eq!(after.id(), expected_id);
    assert_eq!(after.key(), expected_key);
    assert_eq!(after.unit(), expected_unit);
    assert_eq!(after.tag().as_str(), "TEMP");
    assert_eq!(after.plc_type(), id(INT));
    assert_eq!(after.value().as_json(), &json!(150));
    assert_eq!(
        after.limits(),
        [StoredLimit {
            leaf_path: LeafPath::parse("").unwrap(),
            min: Some(Decimal::from(0)),
            max: Some(Decimal::from(200)),
            max_length: None,
        }]
    );
    assert!(after.translations().is_empty());
    let tags: Vec<&str> = draft
        .parameters()
        .iter()
        .map(|p| p.tag().as_str())
        .collect();
    assert_eq!(tags, ["SPEED", "TEMP"]);
    assert_eq!(draft.parameter(expected_key).unwrap(), after);
    assert_eq!(draft.row_version(), 7);
}

#[test]
fn a_struct_value_is_added_in_its_canonical_form() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = new_parameter(2, "ZONE", ST_ZONE, r#"{"run":true,"speed":1.0e1}"#);
    let change = draft.add_parameter(new, &trees).unwrap();
    let value = change.after().unwrap().value().as_json().clone();
    assert_eq!(value, json!({"run": true, "speed": 10}));
}

fn draft_with_4000_parameters_on_s7() -> Draft {
    let mut stored = many_parameters(4_000);
    stored[0].s7_address = Some(address("DB1.DBW0"));
    draft_with(PlcPlatform::S7_1200, stored)
}

#[test]
fn a_duplicate_tag_wins_over_a_bad_value() {
    let message = reject(new_parameter(2, "SPEED", INT, "\"x\""), ErrorCode::RCP_014);
    assert!(message.contains("SPEED"));
}

#[test]
fn a_definition_error_wins_over_a_bad_limit() {
    let mut draft = draft_with(PlcPlatform::S7_300, Vec::new());
    let trees = trees_on(PlcPlatform::S7_300);
    let new = NewParameter {
        limits: LimitList::parse(vec![limit_row("", None, Some("40000"))]).unwrap(),
        ..new_parameter_at(2, "SPEED", INT, "DB1.DBD0")
    };
    assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_054);
}

#[test]
fn a_bad_value_and_a_bad_limit_give_a_list_with_the_value_first() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = limited("\"x\"", vec![limit_row("", None, Some("40000"))]);
    let error = assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_005);
    assert_eq!(
        entry_texts(error),
        [
            "Value of TEMP does not match the type. Expected a number of type INT from -32768 to 32767.",
            "Process limit of TEMP is outside the range of the data type.",
        ]
    );
}

#[test]
fn a_bad_value_wins_over_a_value_outside_the_limits() {
    let new = limited("\"x\"", vec![limit_row("", None, Some("100"))]);
    reject(new, ErrorCode::RCP_005);
}

#[test]
fn a_value_outside_the_limits_wins_over_a_budget_error() {
    let mut draft = draft_with(PlcPlatform::TwinCat3, many_parameters(4_000));
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = limited("150", vec![limit_row("", None, Some("100"))]);
    assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_004);
}

#[test]
fn a_budget_error_wins_over_an_overlap() {
    let mut draft = draft_with_4000_parameters_on_s7();
    let trees = trees_on(PlcPlatform::S7_1200);
    let new = new_parameter_at(2, "TEMP", INT, "DB1.DBW0");
    assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_021);
    let before = snapshot(&draft);
    assert_eq!(before.0.len(), 4_000);
}
