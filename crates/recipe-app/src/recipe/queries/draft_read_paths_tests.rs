//! Tests of the read paths of a draft that take the STRING encoding of its machine (ADR-0202 (m)).
//!
//! Every test runs on a `Utf8` machine with a stored `STRING(10)` leaf. The text `"Łódź"` has
//! 4 characters and 7 UTF-8 bytes. The `Latin1` rule refuses `Ł`, so a path that reads the wrong
//! encoding refuses a value that the `Utf8` rule accepts.

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, StringKind,
    StringLength, TypeExpr, UnitPath,
};
use recipe_domain::recipe::{
    LimitList, Ordinal, ParameterTag, StoredParameter, StoredValue, ValueInput,
};
use recipe_domain::text::LongText;
use serde_json::{Value, json};

use super::parameter_form_tests::{form_as, input};
use super::test_world::seed_users;
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, pkey, reference, text, tid, uid, utf8_machine, vid,
};
use crate::recipe::rebind::fixtures::typed_struct_row;
use crate::recipe::test_support::{DRAFT_VERSION, stored_parameters};
use crate::recipe::{
    RebindTypeRevision, RebindTypeRevisionInput, RetypeInput, SetParameterValue,
    SetParameterValueInput, UpdateParameterDefinition, UpdateParameterDefinitionInput,
};

/// The platform of `utf8_machine`.
const PLATFORM: PlcPlatform = PlcPlatform::CodesysIec;

/// The stored text. It has 4 characters and 7 UTF-8 bytes.
const VALUE: &str = "Łódź";

/// The identifier of the `STRING(10)` row of the machine.
const STRING_TYPE: u128 = 951;

/// `ST_Text@1` and `ST_Text@2`, two revisions with the member `label` of type `STRING(10)`.
const TEXT_1: u128 = 940;
const TEXT_2: u128 = 941;

/// The key of the parameter that each test changes.
const KEY: u128 = 1;

/// The machine row `STRING(10)` of the machine 1.
fn string_row() -> PlcType {
    let length = StringLength::parse(StringKind::String, PLATFORM, 10).unwrap();
    PlcType::try_from(PlcTypeRecord {
        id: tid(STRING_TYPE),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes: length.size_bytes(StringKind::String).unwrap(),
            string_length: Some(length.value()),
        },
    })
    .unwrap()
}

/// A `Utf8` machine `M1` with the live draft `R-1` and the row `STRING(10)`.
///
/// The draft has the parameter `LABEL` of type `plc_type` with the stored `value` and no limit.
fn utf8_draft(plc_type: PlcTypeId, value: Value) -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = utf8_machine(1, "M1");
    fixture.store.seed_machine(&machine);
    fixture.seed_draft(&machine, 1, "R-1");
    fixture.store.seed_type(&string_row());
    let label = StoredParameter {
        plc_type,
        value: StoredValue::from(value),
        limits: Vec::new(),
        ordinal: Ordinal::parse(0).unwrap(),
        ..parameter_on(PLATFORM, KEY, "LABEL", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &label);
    fixture
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

#[tokio::test]
async fn adr_0202_draft_read_paths_carry_utf8_set_value() {
    let fixture = utf8_draft(tid(STRING_TYPE), json!("abc"));

    SetParameterValue::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            SetParameterValueInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                key: pkey(KEY),
                value: ValueInput::Json(json!(VALUE).to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(label_value(&fixture), json!(VALUE));
}

#[tokio::test]
async fn adr_0202_draft_read_paths_carry_utf8_rebind() {
    let fixture = utf8_draft(tid(TEXT_1), json!({"label": VALUE}));
    let member = tid(STRING_TYPE);
    fixture.store.seed_type(&typed_struct_row(
        PLATFORM,
        (TEXT_1, "ST_Text", 1),
        None,
        &[("label", member)],
    ));
    fixture.store.seed_type(&typed_struct_row(
        PLATFORM,
        (TEXT_2, "ST_Text", 2),
        Some(TEXT_1),
        &[("label", member)],
    ));

    RebindTypeRevision::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            RebindTypeRevisionInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                from: tid(TEXT_1),
                to: tid(TEXT_2),
                new_member_values: BTreeMap::new(),
            },
        )
        .await
        .unwrap();

    assert_eq!(label_value(&fixture), json!({"label": VALUE}));
}

#[tokio::test]
async fn adr_0202_draft_read_paths_carry_utf8_retype() {
    let fixture = utf8_draft(tid(STRING_TYPE), json!(VALUE));

    UpdateParameterDefinition::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            UpdateParameterDefinitionInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                key: pkey(KEY),
                unit_path: UnitPath::parse("U1").unwrap(),
                tag: ParameterTag::parse("LABEL").unwrap(),
                name_en: text("Name"),
                description_en: LongText::parse("Description").unwrap(),
                type_expr: TypeExpr::parse("STRING(20)").unwrap(),
                s7_address: None,
                ordinal: Ordinal::parse(0).unwrap(),
                retype: Some(RetypeInput {
                    value: ValueInput::Json(json!(VALUE).to_string()),
                    limits: LimitList::parse(Vec::new()).unwrap(),
                }),
            },
        )
        .await
        .unwrap();

    assert_eq!(label_value(&fixture), json!(VALUE));
}

#[tokio::test]
async fn adr_0202_draft_read_paths_carry_utf8_edit_form() {
    let fixture = utf8_draft(tid(STRING_TYPE), json!(VALUE));

    let form = form_as(
        &fixture,
        Role::ProcessEngineer,
        input(Some(KEY), None, None),
    )
    .await
    .unwrap();

    assert!(form.rule_faults.is_empty(), "{:?}", form.rule_faults);
}
