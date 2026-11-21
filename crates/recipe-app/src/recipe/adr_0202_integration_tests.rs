//! Use case tests of the `STRING` rules of a `Utf8` machine (ADR-0202 (f), (g), (j1)).
//!
//! Each test drives `SetParameterValue` or `AddParameter` on the memory adapter, as a
//! ProcessEngineer does, and asserts the error code, its expected form, or the stored value. The
//! scenario of the Polish letters in a `STRING(10)` has its test in `queries/draft_read_paths_tests.rs`.

use std::collections::BTreeSet;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    LeafPath, Machine, PlcType, PlcTypeRecord, Primitive, RecordShape, StringKind, StringLength,
    UnitPath,
};
use recipe_domain::recipe::{
    Expected, LimitList, Ordinal, ParameterTag, StoredLimit, StoredParameter, StoredValue,
    ValueInput,
};
use recipe_domain::text::LongText;
use serde_json::{Value, json};

use super::test_support::{DRAFT_VERSION, expected_forms, stored_parameters};
use super::{AddParameter, AddParameterInput, SetParameterValue, SetParameterValueInput};
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, machine, meta, parameter_on, pkey, reference, text, tid, uid, utf8_machine, vid,
};
use crate::recipe::rebind::fixtures::leaf_texts;

/// The identifier of the `STRING` row of the machine.
const STRING_TYPE: u128 = 951;

/// The key of the parameter `LABEL`.
const KEY: u128 = 1;

/// A `Utf8` machine `M9` on `CodesysIec`.
fn utf8_m9() -> Machine {
    utf8_machine(1, "M9")
}

/// A `Latin1` machine `M8` on `TwinCat3`.
fn latin1_m8() -> Machine {
    machine(1, "M8")
}

/// The machine row `STRING(length)` of `machine`.
fn string_row(machine: &Machine, length: u32) -> PlcType {
    let platform = machine.platform();
    let length = StringLength::parse(StringKind::String, platform, length).unwrap();
    PlcType::try_from(PlcTypeRecord {
        id: tid(STRING_TYPE),
        machine: Some(machine.id()),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes: length.size_bytes(StringKind::String).unwrap(),
            string_length: Some(length.value()),
        },
    })
    .unwrap()
}

/// A machine with the live draft `R-1` and no parameter.
fn empty_draft(machine: &Machine) -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_actors();
    fixture.store.seed_machine(machine);
    fixture.seed_draft(machine, 1, "R-1");
    fixture
}

/// A machine with the live draft `R-1` and the parameter `LABEL` of type `STRING(length)`.
///
/// `LABEL` holds the value `"old"`. It has the limit `max_length` when `max_length` is `Some`.
fn label_draft(machine: &Machine, length: u32, max_length: Option<u16>) -> Fixture {
    let fixture = empty_draft(machine);
    fixture.store.seed_type(&string_row(machine, length));
    let limits = max_length
        .map(|max_length| StoredLimit {
            leaf_path: LeafPath::parse("").unwrap(),
            min: None,
            max: None,
            max_length: Some(max_length),
        })
        .into_iter()
        .collect();
    let label = StoredParameter {
        plc_type: tid(STRING_TYPE),
        value: StoredValue::from(json!("old")),
        limits,
        ordinal: Ordinal::parse(0).unwrap(),
        ..parameter_on(machine.platform(), KEY, "LABEL", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &label);
    fixture
}

/// Run `SetParameterValue` on `LABEL` with the JSON text `json_text`.
async fn set_label(fixture: &Fixture, json_text: &str) -> Result<(), AppError> {
    SetParameterValue::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            SetParameterValueInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                key: pkey(KEY),
                value: ValueInput::Json(json_text.to_owned()),
            },
        )
        .await
        .map(|_| ())
}

/// The stored value of `LABEL`.
fn label_value(fixture: &Fixture) -> Value {
    stored_parameters(fixture)
        .into_iter()
        .find(|row| row.key == pkey(KEY))
        .unwrap()
        .value
        .as_json()
        .clone()
}

/// A request that adds `HOST` of type `STRING(2)` with the form text `form_text` on the unit `U1`.
fn add_host(form_text: &str) -> AddParameterInput {
    AddParameterInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("HOST").unwrap(),
        name_en: text("Host"),
        description_en: LongText::parse("Host name").unwrap(),
        type_expr: recipe_domain::plant::TypeExpr::parse("STRING(2)").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(1).unwrap(),
        value: leaf_texts(&[("", form_text)]),
        limits: LimitList::parse(Vec::new()).unwrap(),
    }
}

#[tokio::test]
async fn adr_0202_utf8_stores_a_character_outside_the_basic_plane_in_four_bytes() {
    let fixture = label_draft(&utf8_m9(), 4, None);

    set_label(&fixture, r#""😀""#).await.unwrap();

    assert_eq!(label_value(&fixture), json!("😀"));
}

#[tokio::test]
async fn adr_0202_utf8_form_text_with_too_many_bytes_gives_rcp_005_with_the_length_in_bytes() {
    let fixture = empty_draft(&utf8_m9());

    let error = AddParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), add_host("ąę"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(
        expected_forms(&error),
        vec![Expected::StringUtf8 { length: 2 }]
    );
    assert!(stored_parameters(&fixture).is_empty());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn adr_0202_utf8_form_text_with_two_bytes_a_character_adds_the_parameter() {
    let fixture = empty_draft(&utf8_m9());

    AddParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), add_host("ą"))
        .await
        .unwrap();

    let rows = stored_parameters(&fixture);
    assert_eq!(rows.first().unwrap().value.as_json(), &json!("ą"));
}

#[tokio::test]
async fn adr_0202_utf8_refuses_u0000_in_a_value() {
    let fixture = label_draft(&utf8_m9(), 10, None);

    let error = set_label(&fixture, r#""A\u0000""#).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(
        expected_forms(&error),
        vec![Expected::StringUtf8 { length: 10 }]
    );
    assert_eq!(label_value(&fixture), json!("old"));
}

#[tokio::test]
async fn adr_0202_latin1_machine_on_twincat_keeps_the_latin_1_rule() {
    let fixture = label_draft(&latin1_m8(), 10, None);

    let error = set_label(&fixture, r#""Łódź""#).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(
        expected_forms(&error),
        vec![Expected::String { length: 10 }]
    );
    assert_eq!(label_value(&fixture), json!("old"));
}

#[tokio::test]
async fn adr_0202_utf8_json_value_with_too_many_bytes_gives_rcp_005() {
    let fixture = label_draft(&utf8_m9(), 6, None);

    let error = set_label(&fixture, r#""Łódź""#).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(
        expected_forms(&error),
        vec![Expected::StringUtf8 { length: 6 }]
    );
    assert_eq!(label_value(&fixture), json!("old"));
}

#[tokio::test]
async fn adr_0202_utf8_stores_the_first_character_after_latin_1() {
    let fixture = label_draft(&utf8_m9(), 4, None);

    set_label(&fixture, r#""Ā""#).await.unwrap();

    assert_eq!(label_value(&fixture), json!("Ā"));
}

#[tokio::test]
async fn adr_0202_utf8_max_length_of_a_string_limit_counts_bytes() {
    let fixture = label_draft(&utf8_m9(), 10, Some(3));

    let error = set_label(&fixture, r#""éé""#).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_004);
    assert_eq!(label_value(&fixture), json!("old"));
}

#[tokio::test]
async fn adr_0202_utf8_max_length_of_a_string_limit_accepts_the_exact_byte_count() {
    let fixture = label_draft(&utf8_m9(), 10, Some(4));

    set_label(&fixture, r#""éé""#).await.unwrap();

    assert_eq!(label_value(&fixture), json!("éé"));
}
