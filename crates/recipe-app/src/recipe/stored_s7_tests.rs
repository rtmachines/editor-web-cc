//! Tests that a stored S7 address that breaks the earlier rule set gives `E-DB-008` (ADR-0195).
//!
//! The seeded draft holds a `DINT` parameter `SPEED` at `DB1.DBW0`. The width rule refuses that
//! address. A command on the parameter `FEED` must not show it as a rule error of the user.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, Primitive, S7Address, TypeExpr, UnitPath};
use recipe_domain::recipe::{
    LimitList, Ordinal, ParameterTag, StoredParameter, StoredValue, ValueInput,
};
use recipe_domain::text::LongText;
use serde_json::json;

use super::test_support::{DRAFT_VERSION, DraftSnapshot, s7_draft};
use super::{
    AddParameter, AddParameterInput, UpdateParameterDefinition, UpdateParameterDefinitionInput,
};
use crate::memory::fixtures::{
    Fixture, actor, meta, parameter_on, pkey, reference, text, uid, vid,
};
use crate::memory::seed_type_id;

/// An `S7_300` draft with `SPEED` (`DINT` at `DB1.DBW0`) and `FEED` (`INT` at `DB1.DBW8`).
fn draft_with_a_stored_width_defect() -> Fixture {
    let fixture = s7_draft();
    let speed = StoredParameter {
        plc_type: seed_type_id(Primitive::Dint, None).unwrap(),
        value: StoredValue::from(json!(5)),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::S7_300, 1, "SPEED", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    let feed = StoredParameter {
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::S7_300, 2, "FEED", uid(101), 8)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &feed);
    fixture
}

#[tokio::test]
async fn add_parameter_beside_a_stored_width_defect_gives_e_db_008_and_commits_nothing() {
    let fixture = draft_with_a_stored_width_defect();
    let before = DraftSnapshot::capture(&fixture);
    let request = AddParameterInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("NEW").unwrap(),
        name_en: text("New"),
        description_en: LongText::parse("New parameter").unwrap(),
        type_expr: TypeExpr::parse("INT").unwrap(),
        s7_address: Some(S7Address::parse("DB1.DBW16").unwrap()),
        ordinal: Ordinal::parse(2).unwrap(),
        value: ValueInput::Json("1".to_owned()),
        limits: LimitList::parse(Vec::new()).unwrap(),
    };

    let error = AddParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008, "{error:?}");
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn update_definition_beside_a_stored_width_defect_gives_e_db_008_and_commits_nothing() {
    let fixture = draft_with_a_stored_width_defect();
    let before = DraftSnapshot::capture(&fixture);
    let request = UpdateParameterDefinitionInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(2),
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("FEED").unwrap(),
        name_en: text("Renamed"),
        description_en: LongText::parse("Description").unwrap(),
        type_expr: TypeExpr::parse("INT").unwrap(),
        s7_address: Some(S7Address::parse("DB1.DBW8").unwrap()),
        ordinal: Ordinal::parse(0).unwrap(),
        retype: None,
    };

    let error = UpdateParameterDefinition::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008, "{error:?}");
    before.assert_unchanged(&fixture);
}
