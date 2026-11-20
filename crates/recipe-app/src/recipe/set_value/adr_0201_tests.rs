//! Tests of `SetParameterValue` for the rule "WSTRING characters on CODESYS and TwinCAT"
//! (ADR-0201 (a), (c), (d), (f)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::ValueInput;
use serde_json::{Value, json};

use super::{SetParameterValue, SetParameterValueInput};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, meta, reference};
use crate::recipe::test_support::{
    DRAFT_VERSION, DraftSnapshot, expected_forms, iec_wstring, note_key, seed_note_draft,
    speed_key, stored_parameters,
};

/// A request that sets `value` (JSON text) on the parameter with `key` of the draft with version 1.
fn request(key: recipe_domain::recipe::ParameterKey, value: &str) -> SetParameterValueInput {
    SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key,
        value: ValueInput::Json(value.to_owned()),
    }
}

async fn run(fixture: &Fixture, input: SetParameterValueInput) -> Result<(), AppError> {
    SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
        .map(|_| ())
}

/// The stored value of `NOTE`.
fn note_value(fixture: &Fixture) -> Value {
    stored_parameters(fixture)
        .into_iter()
        .find(|row| row.key == note_key())
        .unwrap()
        .value
        .as_json()
        .clone()
}

#[tokio::test]
async fn adr_0201_a_character_outside_the_basic_plane_gives_rcp_005_on_twincat() {
    let fixture = seed_note_draft(PlcPlatform::TwinCat3, 10, json!("old"));
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, request(note_key(), r#""A😀""#))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(expected_forms(&error), vec![iec_wstring(10)]);
    assert_eq!(note_value(&fixture), json!("old"));
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn adr_0201_a_character_outside_the_basic_plane_gives_rcp_005_on_codesys() {
    let fixture = seed_note_draft(PlcPlatform::CodesysIec, 10, json!("old"));
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, request(note_key(), r#""Z𝄞""#))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(expected_forms(&error), vec![iec_wstring(10)]);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn adr_0201_the_ends_of_the_two_ranges_are_stored() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let fixture = seed_note_draft(platform, 4, json!("old"));
        run(&fixture, request(note_key(), r#""\u0001퟿￿""#))
            .await
            .unwrap();
        assert_eq!(
            note_value(&fixture),
            json!("\u{1}\u{D7FF}\u{E000}\u{FFFF}"),
            "{platform:?}"
        );
    }
}

#[tokio::test]
async fn adr_0201_a_stored_value_of_the_earlier_set_does_not_stop_another_parameter() {
    // ADR-0201 (e), (f): the command checks only the value that it writes.
    let fixture = seed_note_draft(PlcPlatform::TwinCat3, 10, json!("A😀"));
    run(&fixture, request(speed_key(), "130")).await.unwrap();
    assert_eq!(note_value(&fixture), json!("A😀"));
    let speed = stored_parameters(&fixture)
        .into_iter()
        .find(|row| row.key == speed_key())
        .unwrap();
    assert_eq!(speed.value.as_json(), &json!(130));
    assert_eq!(
        fixture
            .store
            .state()
            .version_row_version(crate::memory::fixtures::vid(DRAFT_VERSION)),
        Some(2)
    );
}
