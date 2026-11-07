//! Tests of `Draft::set_value` and `Draft::replace_limits` on a stored part of the earlier rule set.
//!
//! The earlier set (ADR-0120) allows a `DATE` count up to 72 840 on `S7_1500`.
//! The current set (ADR-0114) allows a count up to 72 683.
//! A stored count of 72 700 is `D#2169-01-17`. It is legal in the earlier set and illegal in the
//! current set.

use rust_decimal::Decimal;
use serde_json::json;

use super::add::support::{REAL, draft_with, limit_row, snapshot, trees_on};
use super::support::{key, stored_parameter};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{id, machine, primitive_row, types_for};
use crate::plant::{LeafPath, PlcPlatform, Primitive, TypeTrees};
use crate::recipe::{Draft, LimitList, StoredLimit, ValueInput};

/// The type identifier of the `DATE` row of these tests.
const DATE: u128 = 7;

/// The stored `DATE` count of `D#2169-01-17`. The earlier rule set allows it.
const EARLIER_ONLY_DAY: i64 = 72_700;

/// The stored `DATE` count of `D#2168-12-31`. The current rule set allows it.
const LAST_DAY: i64 = 72_683;

/// The trees of one `DATE` type on `S7_1500`.
fn date_trees() -> TypeTrees {
    let rows = vec![primitive_row(DATE, Primitive::Date)];
    TypeTrees::resolve(
        [id(DATE)],
        &types_for(machine(), PlcPlatform::S7_1500, rows),
    )
}

/// A live draft on `S7_1500` with `DUE` of type `DATE`, stored value 72 700, and stored `max` 72 700.
///
/// The key of `DUE` derives from 1.
fn due_draft() -> Draft {
    due_draft_with_max(EARLIER_ONLY_DAY)
}

/// The draft of [`due_draft`] with the stored `max` of `DUE` set to `max`.
fn due_draft_with_max(max: i64) -> Draft {
    let mut stored = stored_parameter(1, "DUE", id(DATE), None);
    stored.value = json!(EARLIER_ONLY_DAY).into();
    stored.limits = vec![StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some(Decimal::from(max)),
        max_length: None,
    }];
    draft_with(PlcPlatform::S7_1500, vec![stored])
}

fn json_text(text: &str) -> ValueInput {
    ValueInput::Json(format!("\"{text}\""))
}

#[test]
fn a_stored_limit_of_the_earlier_set_does_not_block_set_value_with_the_last_day() {
    let mut draft = due_draft();
    let change = draft
        .set_value(key(1), &json_text("D#2168-12-31"), &date_trees())
        .unwrap();
    assert_eq!(change.after().unwrap().value(), &json!(LAST_DAY).into());
}

#[test]
fn replace_limits_with_the_last_day_passes_after_set_value_fixed_the_value() {
    let mut draft = due_draft();
    let trees = date_trees();
    draft
        .set_value(key(1), &json_text("D#2168-12-31"), &trees)
        .unwrap();
    let before = snapshot(&draft);
    let rows = LimitList::parse(vec![limit_row("", None, Some("D#2168-12-31"))]).unwrap();
    let change = draft.replace_limits(key(1), &rows, &trees).unwrap();
    let limits = change.after().unwrap().limits();
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].max, Some(Decimal::from(LAST_DAY)));
    assert_ne!(snapshot(&draft), before, "the command changed nothing");
}

#[test]
fn set_value_with_the_last_day_passes_after_replace_limits_dropped_the_max() {
    let mut draft = due_draft();
    let trees = date_trees();
    let rows = LimitList::parse(vec![limit_row("", Some("D#2000-01-01"), None)]).unwrap();
    draft.replace_limits(key(1), &rows, &trees).unwrap();
    let change = draft
        .set_value(key(1), &json_text("D#2168-12-31"), &trees)
        .unwrap();
    assert_eq!(change.after().unwrap().value(), &json!(LAST_DAY).into());
}

#[test]
fn set_value_with_a_day_of_the_earlier_set_only_gives_e_rcp_005() {
    let mut draft = due_draft();
    let trees = date_trees();
    let before = snapshot(&draft);
    let error = draft
        .set_value(key(1), &json_text("D#2169-01-17"), &trees)
        .expect_err("the current set refuses the day");
    assert_eq!(error.code(), ErrorCode::RCP_005, "{error}");
    assert_eq!(
        snapshot(&draft),
        before,
        "a failed command changed the draft"
    );
}

#[test]
fn set_value_above_a_stored_max_inside_the_current_range_gives_e_rcp_004() {
    let mut draft = due_draft_with_max(72_000);
    let trees = date_trees();
    let before = snapshot(&draft);
    let error = draft
        .set_value(key(1), &json_text("D#2168-12-31"), &trees)
        .expect_err("the value is above the stored max");
    assert_eq!(error.code(), ErrorCode::RCP_004, "{error}");
    assert_eq!(
        snapshot(&draft),
        before,
        "a failed command changed the draft"
    );
}

/// A live draft on `TwinCat3` with `SPEED` of type `REAL`, stored value 0, and stored `max` `100.00001`.
///
/// The stored `max` has 8 significant digits. The current rule set refuses it with `E-RCP-055`.
/// The earlier rule set accepts it. The key of `SPEED` derives from 1.
fn real_draft_with_long_max() -> Draft {
    let mut stored = stored_parameter(1, "SPEED", id(REAL), None);
    stored.limits = vec![StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some("100.00001".parse().unwrap()),
        max_length: None,
    }];
    draft_with(PlcPlatform::TwinCat3, vec![stored])
}

#[test]
fn a_stored_real_limit_with_8_digits_does_not_block_set_value() {
    let mut draft = real_draft_with_long_max();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let change = draft
        .set_value(key(1), &ValueInput::Json("50".to_owned()), &trees)
        .unwrap();
    assert_eq!(change.after().unwrap().value(), &json!(50).into());
    let limits = change.after().unwrap().limits();
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].max, Some("100.00001".parse().unwrap()));
}

#[test]
fn remove_parameter_passes_for_a_stored_day_of_the_earlier_set_only() {
    let mut draft = due_draft();
    let change = draft.remove_parameter(key(1)).unwrap();
    assert_eq!(
        change.before().unwrap().value(),
        &json!(EARLIER_ONLY_DAY).into()
    );
    assert!(change.after().is_none());
    assert!(
        draft.parameter(key(1)).is_err(),
        "the draft still holds DUE"
    );
    assert!(draft.parameters().is_empty());
}

#[test]
fn replace_limits_below_a_stored_day_of_the_earlier_set_writes_literals_in_the_message() {
    let mut draft = due_draft();
    let rows = LimitList::parse(vec![limit_row("", None, Some("D#2168-12-31"))]).unwrap();
    let error = draft
        .replace_limits(key(1), &rows, &date_trees())
        .expect_err("the stored value is above the new max");
    assert_eq!(error.code(), ErrorCode::RCP_004, "{error}");
    let message = error.to_string();
    for literal in ["D#2169-01-17", "D#1990-01-01", "D#2168-12-31"] {
        assert!(message.contains(literal), "{literal} missing in {message}");
    }
}

#[test]
fn set_value_against_a_stored_limit_of_the_earlier_set_writes_literals_in_the_message() {
    let mut stored = stored_parameter(1, "DUE", id(DATE), None);
    stored.value = json!(EARLIER_ONLY_DAY).into();
    stored.limits = vec![StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some(Decimal::from(EARLIER_ONLY_DAY)),
        max: Some(Decimal::from(72_800)),
        max_length: None,
    }];
    let mut draft = draft_with(PlcPlatform::S7_1500, vec![stored]);
    let error = draft
        .set_value(key(1), &json_text("D#2168-12-31"), &date_trees())
        .expect_err("the value is below the stored min");
    assert_eq!(error.code(), ErrorCode::RCP_004, "{error}");
    let message = error.to_string();
    for literal in ["D#2168-12-31", "D#2169-01-17", "D#2169-04-27"] {
        assert!(message.contains(literal), "{literal} missing in {message}");
    }
}
