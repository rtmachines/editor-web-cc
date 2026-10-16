//! Tests of `SetParameterValue` with the texts of a form (ADR-0039 (c), ADR-0041 (b)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, Primitive};
use recipe_domain::recipe::{StoredParameter, StoredValue, ValueInput};
use serde_json::json;

use super::{SetParameterValue, SetParameterValueInput};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, meta, parameter_on, pkey, reference, tid, uid, vid};
use crate::memory::seed_type_id;
use crate::recipe::DraftOutput;
use crate::recipe::rebind::fixtures::{char_row, error_lines, leaf_texts, typed_struct_row};
use crate::recipe::test_support::{
    DRAFT_VERSION, DraftSnapshot, speed_key, stored_parameters, twincat_draft,
};

/// The identifier of the global `CHAR` row.
const CHAR: u128 = 950;
/// The identifier of `ST_Text@1` with the members `label` (`STRING(80)`) and `sign` (`CHAR`).
const TEXT_1: u128 = 951;
/// The key number of the parameter `TEXT` of the type `ST_Text@1`.
const TEXT_PARAMETER: u128 = 21;

/// A request that sets the parameter `key` to `value` with the row version `expected`.
fn request(key: u128, value: ValueInput, expected: i64) -> SetParameterValueInput {
    SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: expected,
        key: pkey(key),
        value,
    }
}

/// Run the use case as a process engineer.
async fn run(fixture: &Fixture, input: SetParameterValueInput) -> Result<DraftOutput, AppError> {
    SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
}

/// The key number of `SPEED`.
fn speed() -> u128 {
    let number = 1;
    assert_eq!(pkey(number), speed_key());
    number
}

/// A draft with `SPEED` and the parameter `TEXT` of the type `ST_Text@1`.
fn text_draft() -> Fixture {
    let fixture = twincat_draft();
    fixture.store.seed_type(&char_row(CHAR));
    let label = seed_type_id(Primitive::String, Some(80)).unwrap();
    let members = [("label", label), ("sign", tid(CHAR))];
    let row = typed_struct_row(
        PlcPlatform::TwinCat3,
        (TEXT_1, "ST_Text", 1),
        None,
        &members,
    );
    fixture.store.seed_type(&row);
    let parameter = StoredParameter {
        plc_type: tid(TEXT_1),
        value: StoredValue::from(json!({"label": "a", "sign": "b"})),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::TwinCat3, TEXT_PARAMETER, "TEXT", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &parameter);
    fixture
}

#[tokio::test]
async fn a_text_sets_the_value_and_returns_the_next_row_version() {
    let fixture = twincat_draft();
    let output = run(&fixture, request(speed(), leaf_texts(&[("", "150")]), 1))
        .await
        .unwrap();
    assert_eq!(output.row_version, 2);
    let parameters = stored_parameters(&fixture);
    let row = parameters.first().unwrap();
    assert_eq!(row.value.as_json(), &json!(150));
    assert_eq!(row.limits.len(), 1);
}

#[tokio::test]
async fn every_bad_text_gives_one_line_of_one_list_and_writes_nothing() {
    let fixture = twincat_draft();
    let before = DraftSnapshot::capture(&fixture);
    let value = leaf_texts(&[("", "abc"), (".extra", "1")]);
    let error = run(&fixture, request(speed(), value, 1)).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(error_lines(&error), ["E-RCP-005 ''", "E-RCP-005 '.extra'"]);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn string_and_char_texts_are_stored_without_a_trim() {
    let fixture = text_draft();
    let value = leaf_texts(&[(".label", " Zé "), (".sign", " ")]);
    run(&fixture, request(TEXT_PARAMETER, value, 1))
        .await
        .unwrap();
    let row = stored_parameters(&fixture)
        .into_iter()
        .find(|row| row.key == pkey(TEXT_PARAMETER))
        .unwrap();
    assert_eq!(row.value.as_json(), &json!({"label": " Zé ", "sign": " "}));
}

#[tokio::test]
async fn an_old_row_version_with_texts_gives_web_001_and_writes_nothing() {
    let fixture = twincat_draft();
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, request(speed(), leaf_texts(&[("", "150")]), 7))
        .await
        .unwrap_err();
    assert!(matches!(
        &error,
        AppError::ConcurrentEdit {
            expected: 7,
            actual: 1,
            ..
        }
    ));
    before.assert_unchanged(&fixture);
}
