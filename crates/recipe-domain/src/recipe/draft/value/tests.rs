//! Tests of `Draft::set_value` and `Draft::replace_limits` (design D6).

use rust_decimal::Decimal;
use serde_json::json;

use super::super::add::support::{
    INT, NOT_ASKED, draft_with, draft_with_speed, limit_row, snapshot, texts_input, trees_on,
};
use super::super::edit_support::{draft_with_content, draft_with_table, stored_limit_to};
use super::super::support::{key, stored_parameter};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::id;
use crate::plant::{PlcPlatform, Primitive};
use crate::recipe::{
    Draft, DraftError, Expected, LimitInput, LimitList, Parameter, RecipeError, ValueInput,
};

fn limits(rows: Vec<LimitInput>) -> LimitList {
    LimitList::parse(rows).unwrap()
}

fn speed(draft: &Draft) -> Parameter {
    draft.parameter(key(1)).unwrap().clone()
}

fn json(text: &str) -> ValueInput {
    ValueInput::Json(text.to_owned())
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

fn set_value_rejected(draft: &mut Draft, json_text: &str, code: ErrorCode) -> DraftError {
    let trees = trees_on(PlcPlatform::TwinCat3);
    let before = snapshot(draft);
    let error = draft
        .set_value(key(1), &json(json_text), &trees)
        .expect_err("the command must fail");
    assert_eq!(error.code(), code, "{error}");
    assert_eq!(
        snapshot(draft),
        before,
        "a failed command changed the draft"
    );
    error
}

fn replace_rejected(draft: &mut Draft, rows: Vec<LimitInput>, code: ErrorCode) -> DraftError {
    let trees = trees_on(PlcPlatform::TwinCat3);
    let before = snapshot(draft);
    let error = draft
        .replace_limits(key(1), &limits(rows), &trees)
        .expect_err("the command must fail");
    assert_eq!(error.code(), code, "{error}");
    assert_eq!(
        snapshot(draft),
        before,
        "a failed command changed the draft"
    );
    error
}

#[test]
fn a_new_value_inside_the_limit_keeps_limits_and_translations() {
    let mut draft = draft_with_content();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let old = speed(&draft);
    let change = draft.set_value(key(1), &json("150"), &trees).unwrap();
    let after = change.after().unwrap();
    assert_eq!(after.value(), &json!(150).into());
    assert_eq!(after.limits(), old.limits());
    assert_eq!(after.translations(), old.translations());
    assert_eq!(after.tag(), old.tag());
    assert_eq!(change.before(), Some(&old));
    assert_eq!(speed(&draft), *after);
}

#[test]
fn a_value_above_the_stored_limit_gives_e_rcp_004() {
    let mut draft = draft_with_content();
    let error = set_value_rejected(&mut draft, "250", ErrorCode::RCP_004);
    assert_eq!(
        error.to_string(),
        "Value 250 of SPEED is outside the process limit 0 to 200."
    );
}

#[test]
fn a_json_string_for_an_int_gives_e_rcp_005() {
    let mut draft = draft_with_content();
    let error = set_value_rejected(&mut draft, "\"fast\"", ErrorCode::RCP_005);
    let range = Expected::NumberRange {
        type_name: Primitive::Int,
        low: Decimal::from(-32_768),
        high: Decimal::from(32_767),
    };
    let list = match error {
        DraftError::List(list) => Some(list),
        DraftError::Recipe(_) | DraftError::Plant(_) => None,
    }
    .unwrap();
    assert!(matches!(
        list.first(),
        RecipeError::ValueShapeMismatch { expected, .. } if *expected == range
    ));
}

#[test]
fn a_stored_limit_outside_the_type_range_gives_e_rcp_006() {
    let mut stored = stored_parameter(1, "SPEED", id(INT), None);
    stored.limits = vec![stored_limit_to(40_000)];
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![stored]);
    set_value_rejected(&mut draft, "1", ErrorCode::RCP_006);
}

#[test]
fn a_type_that_the_trees_do_not_offer_gives_e_plt_013_for_a_value() {
    let stored = stored_parameter(1, "SPEED", id(NOT_ASKED), None);
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![stored]);
    set_value_rejected(&mut draft, "1", ErrorCode::PLT_013);
}

#[test]
fn an_unknown_key_gives_e_rcp_051_for_a_value() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let before = snapshot(&draft);
    let error = draft.set_value(key(77), &json("1"), &trees).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    assert_eq!(snapshot(&draft), before);
}

#[test]
fn two_leaves_above_the_limit_give_two_e_rcp_004_entries_in_leaf_order() {
    let mut draft = draft_with_table();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let before = snapshot(&draft);
    let value = texts_input(&[
        ("[1]", "1"),
        ("[2]", "2"),
        ("[3]", "3500"),
        ("[4]", "4"),
        ("[5]", "5"),
        ("[6]", "4000"),
        ("[7]", "7"),
        ("[8]", "8"),
    ]);
    let error = draft.set_value(key(1), &value, &trees).unwrap_err();
    assert_eq!(
        entry_texts(error),
        [
            "Value 3500 of SPEED_TABLE[3] is outside the process limit 0 to 3000.",
            "Value 4000 of SPEED_TABLE[6] is outside the process limit 0 to 3000.",
        ]
    );
    assert_eq!(snapshot(&draft), before);
}

#[test]
fn a_bad_leaf_and_a_leaf_above_the_limit_give_both_faults_in_leaf_order() {
    let mut draft = draft_with_table();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let before = snapshot(&draft);
    let value = texts_input(&[
        ("[1]", "x"),
        ("[2]", "2"),
        ("[3]", "3500"),
        ("[4]", "4"),
        ("[5]", "5"),
        ("[6]", "6"),
        ("[7]", "7"),
        ("[8]", "8"),
    ]);
    let error = draft.set_value(key(1), &value, &trees).unwrap_err();
    assert_eq!(
        entry_texts(error),
        [
            "Value of SPEED_TABLE[1] does not match the type. Expected an integer of type INT.",
            "Value 3500 of SPEED_TABLE[3] is outside the process limit 0 to 3000.",
        ]
    );
    assert_eq!(snapshot(&draft), before);
}

#[test]
fn a_single_fault_of_a_value_stays_a_list() {
    let mut draft = draft_with_content();
    let error = set_value_rejected(&mut draft, "250", ErrorCode::RCP_004);
    assert_eq!(entry_texts(error).len(), 1);
}

#[test]
fn two_bad_limit_rows_give_a_list_in_row_order() {
    let mut draft = draft_with_content();
    let rows = vec![
        limit_row(".speed", None, Some("5")),
        limit_row("", Some("10"), Some("5")),
    ];
    let error = replace_rejected(&mut draft, rows, ErrorCode::RCP_017);
    assert_eq!(
        entry_texts(error),
        [
            "Path .speed does not exist in the type of SPEED.",
            "The minimum 10 of SPEED is greater than the maximum 5.",
        ]
    );
}

#[test]
fn a_value_fault_comes_before_a_limit_row_fault() {
    let mut draft = draft_with_content();
    let rows = vec![
        limit_row(".speed", None, Some("5")),
        limit_row("", Some("0"), Some("100")),
    ];
    let error = replace_rejected(&mut draft, rows, ErrorCode::RCP_004);
    assert_eq!(
        entry_texts(error),
        [
            "Value 120 of SPEED is outside the process limit 0 to 100.",
            "Path .speed does not exist in the type of SPEED.",
        ]
    );
}

#[test]
fn limits_with_a_lower_maximum_than_the_value_give_e_rcp_004() {
    let mut draft = draft_with_content();
    let rows = vec![limit_row("", Some("0"), Some("100"))];
    replace_rejected(&mut draft, rows, ErrorCode::RCP_004);
    let kept = speed(&draft);
    assert_eq!(kept.limits().first().unwrap().max, Some(Decimal::from(200)));
}

#[test]
fn limits_that_fit_the_value_keep_the_value_and_the_translations() {
    let mut draft = draft_with_content();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let old = speed(&draft);
    let rows = vec![limit_row("", Some("0"), Some("150"))];
    let change = draft.replace_limits(key(1), &limits(rows), &trees).unwrap();
    let after = change.after().unwrap();
    assert_eq!(after.limits().len(), 1);
    assert_eq!(
        after.limits().first().unwrap().max,
        Some(Decimal::from(150))
    );
    assert_eq!(after.value(), old.value());
    assert_eq!(after.translations(), old.translations());
    assert_eq!(change.before(), Some(&old));
}

#[test]
fn an_empty_list_removes_every_limit() {
    let mut draft = draft_with_content();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let change = draft
        .replace_limits(key(1), &limits(Vec::new()), &trees)
        .unwrap();
    assert!(change.after().unwrap().limits().is_empty());
}

#[test]
fn limits_for_a_missing_member_give_e_rcp_017() {
    let mut draft = draft_with_content();
    let rows = vec![limit_row(".speed", None, Some("5"))];
    replace_rejected(&mut draft, rows, ErrorCode::RCP_017);
}

#[test]
fn a_stored_value_of_the_wrong_form_gives_e_rcp_005_for_limits() {
    let mut stored = stored_parameter(1, "SPEED", id(INT), None);
    stored.value = json!("text").into();
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![stored]);
    let rows = vec![limit_row("", None, Some("5"))];
    replace_rejected(&mut draft, rows, ErrorCode::RCP_005);
}

#[test]
fn limits_above_the_limit_row_budget_give_e_rcp_021() {
    let mut full = stored_parameter(1, "SPEED", id(INT), None);
    full.limits = (0..32_768).map(|_| stored_limit_to(200)).collect();
    let other = stored_parameter(2, "TEMP", id(INT), None);
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![full, other]);
    let trees = trees_on(PlcPlatform::TwinCat3);
    let before = snapshot(&draft);
    let rows = vec![limit_row("", None, Some("5"))];
    let error = draft
        .replace_limits(key(2), &limits(rows), &trees)
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_021);
    assert_eq!(
        error.to_string(),
        "The version exceeds the limit of 32768 limit rows. Current count: 32769."
    );
    assert_eq!(snapshot(&draft), before);
}

#[test]
fn a_type_that_the_trees_do_not_offer_gives_e_plt_013_for_limits() {
    let stored = stored_parameter(1, "SPEED", id(NOT_ASKED), None);
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![stored]);
    let rows = vec![limit_row("", None, Some("5"))];
    replace_rejected(&mut draft, rows, ErrorCode::PLT_013);
}

#[test]
fn an_unknown_key_gives_e_rcp_051_for_limits() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let before = snapshot(&draft);
    let error = draft
        .replace_limits(key(77), &limits(Vec::new()), &trees)
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    assert_eq!(snapshot(&draft), before);
}
