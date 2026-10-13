//! Tests of `Draft::set_definition` (design D6, `UpdateParameterDefinition`).

use rust_decimal::Decimal;
use serde_json::json;

use super::super::add::support::{
    DINT, INT, UNRESOLVED, draft_with, draft_with_speed, limit_row, live_unit, snapshot, trees_on,
};
use super::super::edit_support::draft_with_content;
use super::super::support::{key, stored_parameter, tag};
use super::{DefinitionUpdate, Retype};
use crate::error_codes::ErrorCode;
use crate::plant::PlcPlatform;
use crate::plant::fixtures::id;
use crate::recipe::{Draft, DraftError, LimitInput, LimitList, Ordinal};
use crate::text::{LongText, Text};

fn update(tag_text: &str, type_n: u128) -> DefinitionUpdate {
    DefinitionUpdate {
        unit: live_unit(),
        tag: tag(tag_text),
        name_en: Text::parse("Renamed").unwrap(),
        description_en: LongText::parse("New description").unwrap(),
        plc_type: id(type_n),
        s7_address: None,
        ordinal: Ordinal::parse(9).unwrap(),
    }
}

fn retype(value_json: &str, rows: Vec<LimitInput>) -> Retype {
    Retype {
        value_json: value_json.to_owned(),
        limits: LimitList::parse(rows).unwrap(),
    }
}

fn assert_rejected(
    draft: &mut Draft,
    update: DefinitionUpdate,
    retype: Option<Retype>,
    code: ErrorCode,
) -> DraftError {
    let trees = trees_on(PlcPlatform::TwinCat3);
    let before = snapshot(draft);
    let error = draft
        .set_definition(key(1), update, retype, &trees)
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
fn a_type_change_without_a_value_gives_e_rcp_005() {
    let mut draft = draft_with_content();
    let error = assert_rejected(&mut draft, update("SPEED", DINT), None, ErrorCode::RCP_005);
    assert_eq!(
        error.to_string(),
        "Value of SPEED does not match the type. Expected a new value for the new type."
    );
}

#[test]
fn a_type_change_with_a_value_and_limits_applies_both() {
    let mut draft = draft_with_content();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = retype("70000", vec![limit_row("", Some("0"), Some("100000"))]);
    let change = draft
        .set_definition(key(1), update("SPEED", DINT), Some(new), &trees)
        .unwrap();
    let after = change.after().unwrap();
    assert_eq!(after.plc_type(), id(DINT));
    assert_eq!(after.value(), &json!(70000).into());
    assert_eq!(after.limits().len(), 1);
    assert_eq!(
        after.limits().first().unwrap().max,
        Some(Decimal::from(100_000))
    );
    assert_eq!(change.before().unwrap().plc_type(), id(INT));
    assert_eq!(draft.parameter(key(1)).unwrap(), after);
}

#[test]
fn a_retype_value_outside_its_new_limit_gives_e_rcp_004() {
    let mut draft = draft_with_content();
    let new = retype("500", vec![limit_row("", Some("0"), Some("100"))]);
    assert_rejected(
        &mut draft,
        update("SPEED", DINT),
        Some(new),
        ErrorCode::RCP_004,
    );
}

#[test]
fn without_a_type_change_the_value_limits_and_translations_stay() {
    let mut draft = draft_with_content();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let old = draft.parameter(key(1)).unwrap().clone();
    let change = draft
        .set_definition(key(1), update("SPEED", INT), None, &trees)
        .unwrap();
    let after = change.after().unwrap();
    assert_eq!(after.name_en().as_str(), "Renamed");
    assert_eq!(after.ordinal(), Ordinal::parse(9).unwrap());
    assert_eq!(after.value(), old.value());
    assert_eq!(after.limits(), old.limits());
    assert_eq!(after.translations(), old.translations());
    assert_eq!(after.id(), old.id());
    assert_eq!(after.key(), old.key());
    assert_eq!(change.before(), Some(&old));
}

#[test]
fn the_same_type_with_a_retype_applies_the_new_value_and_limits() {
    let mut draft = draft_with_content();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = retype("40", vec![limit_row("", Some("0"), Some("50"))]);
    let change = draft
        .set_definition(key(1), update("SPEED", INT), Some(new), &trees)
        .unwrap();
    let after = change.after().unwrap();
    assert_eq!(after.value(), &json!(40).into());
    assert_eq!(after.limits().first().unwrap().max, Some(Decimal::from(50)));
    assert_eq!(after.translations().len(), 1);
}

#[test]
fn a_parameter_whose_type_did_not_resolve_gets_a_new_type_with_a_retype() {
    let broken = stored_parameter(1, "SPEED", id(UNRESOLVED), None);
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![broken]);
    let trees = trees_on(PlcPlatform::TwinCat3);
    let new = retype("5", Vec::new());
    let change = draft
        .set_definition(key(1), update("SPEED", INT), Some(new), &trees)
        .unwrap();
    assert_eq!(change.after().unwrap().plc_type(), id(INT));
}

#[test]
fn a_parameter_whose_type_did_not_resolve_needs_a_value_for_the_new_type() {
    let broken = stored_parameter(1, "SPEED", id(UNRESOLVED), None);
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![broken]);
    assert_rejected(&mut draft, update("SPEED", INT), None, ErrorCode::RCP_005);
}

#[test]
fn a_tag_of_another_parameter_gives_e_rcp_014() {
    let mut draft = draft_with_content();
    let error = assert_rejected(&mut draft, update("TEMP", INT), None, ErrorCode::RCP_014);
    assert_eq!(
        error.to_string(),
        "Parameter tag TEMP is already used in version 3."
    );
}

#[test]
fn a_changed_tag_that_no_parameter_uses_passes() {
    let mut draft = draft_with_content();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let change = draft
        .set_definition(key(1), update("RPM", INT), None, &trees)
        .unwrap();
    assert_eq!(change.after().unwrap().tag().as_str(), "RPM");
}

#[test]
fn an_unknown_key_gives_e_rcp_051() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let before = snapshot(&draft);
    let error = draft
        .set_definition(key(77), update("SPEED", INT), None, &trees)
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    assert_eq!(snapshot(&draft), before);
}

#[test]
fn a_new_type_that_the_trees_do_not_offer_gives_e_plt_013() {
    let mut draft = draft_with_content();
    let new = retype("1", Vec::new());
    assert_rejected(
        &mut draft,
        update("SPEED", 98),
        Some(new),
        ErrorCode::PLT_013,
    );
}

#[test]
fn an_address_on_a_platform_without_s7_gives_e_rcp_053() {
    let mut draft = draft_with_content();
    let mut changed = update("SPEED", INT);
    changed.s7_address = Some(super::super::support::address("DB1.DBW0"));
    assert_rejected(&mut draft, changed, None, ErrorCode::RCP_053);
}
