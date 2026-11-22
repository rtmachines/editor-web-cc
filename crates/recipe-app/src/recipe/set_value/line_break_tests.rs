//! Tests of `SetParameterValue` for the rule "no LF and no CR in a request text" (ADR-0210 (a)).

use std::collections::BTreeSet;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::LeafPath;
use recipe_domain::plant::{
    PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape, StringKind, StringLength,
};
use recipe_domain::recipe::{StoredLimit, StoredParameter, StoredValue, ValueInput};
use serde_json::json;

use super::{SetParameterValue, SetParameterValueInput};
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, pkey, reference, tid, uid, vid,
};
use crate::recipe::rebind::fixtures::leaf_texts;
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot, stored_parameters};

/// The identifier of the row `STRING(10)`.
const NAME_TYPE: u128 = 985;
/// The key number of the parameter `NAME`.
const NAME_KEY: u128 = 8;

/// The row `STRING(10)` of machine 1 on `platform`.
fn string_row(platform: PlcPlatform) -> PlcType {
    let size_bytes = StringLength::parse(StringKind::String, platform, 10)
        .unwrap()
        .size_bytes(StringKind::String)
        .unwrap();
    PlcType::try_from(PlcTypeRecord {
        id: tid(NAME_TYPE),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes,
            string_length: Some(10),
        },
    })
    .unwrap()
}

/// A draft with the parameter `NAME` of type `STRING(10)` that holds `"old"`.
fn name_draft() -> Fixture {
    let platform = PlcPlatform::TwinCat3;
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture.store.seed_type(&string_row(platform));
    let limit = StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: None,
        max_length: Some(10),
    };
    let parameter = StoredParameter {
        plc_type: tid(NAME_TYPE),
        value: StoredValue::from(json!("old")),
        limits: vec![limit],
        ..parameter_on(platform, NAME_KEY, "NAME", uid(102), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &parameter);
    fixture
}

/// Run the use case on `NAME` with `value`.
async fn set_name(fixture: &Fixture, value: ValueInput) -> crate::error::AppError {
    let input = SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(NAME_KEY),
        value,
    };
    SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
        .unwrap_err()
}

/// The stored value of `NAME`.
fn name_value(fixture: &Fixture) -> serde_json::Value {
    stored_parameters(fixture)
        .into_iter()
        .find(|row| row.key == pkey(NAME_KEY))
        .unwrap()
        .value
        .as_json()
        .clone()
}

#[tokio::test]
async fn set_parameter_value_with_lf_gives_e_rcp_005() {
    let fixture = name_draft();
    let before = DraftSnapshot::capture(&fixture);
    let error = set_name(&fixture, ValueInput::Json(r#""A\nB""#.to_owned())).await;
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(name_value(&fixture), json!("old"));
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn set_parameter_value_with_a_form_text_with_cr_gives_e_rcp_005() {
    let fixture = name_draft();
    let before = DraftSnapshot::capture(&fixture);
    let error = set_name(&fixture, leaf_texts(&[("", "A\rB")])).await;
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(name_value(&fixture), json!("old"));
    before.assert_unchanged(&fixture);
}
