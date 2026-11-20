//! Tests of the check of the type text in `GetParameterForm` (ADR-0206 (d3), ADR-0203 (j1),
//! ADR-0204 (d1)).
//!
//! The form checks a new type text, and a changed type text of the edit form, with the current rule
//! set before it resolves the text. An unchanged type text of an edit form keeps its stored struct.
//! The tests build each type text with the earlier rule set, as the request parser of `recipe-web`
//! does after a refusal.

use std::collections::BTreeSet;

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, DerivedTypeReason, MemberName, PlantError, PlcPlatform,
    PlcType, PlcTypeRecord, RecordShape, RuleSet, TypeMember, TypeName,
};
use recipe_domain::recipe::{Ordinal, RecipeVersionId, StoredParameter, StoredValue};
use serde_json::{Value, json};

use super::parameter_form::ParameterForm;
use super::parameter_form_tests::{form_as, input_as};
use super::test_world::seed_users;
use crate::auth::ParameterFormPurpose::{self, AddParameter, UpdateParameterDefinition};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, mid, parameter_on, tid, uid};
use crate::memory::seed_type_id;

/// The key of the parameter `START`, of type `INT`.
const START: u128 = 22;
/// The key of the parameter that holds the stored struct of the case.
const STRUCT_PARAMETER: u128 = 21;
/// The key of the parameter `STAMPS`, an array of the stored struct.
const ARRAY_PARAMETER: u128 = 23;
/// The row number of the stored struct of a case.
const STRUCT_ROW: u128 = 984;
/// The row number of the array of the stored struct of a case.
const ARRAY_ROW: u128 = 985;

/// A draft of the machine `code` on `platform` with the parameter `START` of type `INT`.
fn world_on(platform: PlcPlatform, code: &str) -> (Fixture, RecipeVersionId) {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = fixture.seed_machine_on(platform, 1, code);
    let (_recipe, version) = fixture.seed_draft(&machine, 1, "R-1");
    let start = StoredParameter {
        ordinal: Ordinal::parse(1).unwrap(),
        ..parameter_on(platform, START, "START", uid(101), 0)
    };
    fixture.store.seed_parameter(version, &start);
    (fixture, version)
}

/// The stored struct `name@1` of the machine with the `members`, which have the type `INT`.
///
/// The row is from before the current rule set, so the name and the row use the earlier one.
fn struct_row(platform: PlcPlatform, name: &str, members: &[&str]) -> PlcType {
    let int = seed_type_id(recipe_domain::plant::Primitive::Int, None).unwrap();
    let record = PlcTypeRecord {
        id: tid(STRUCT_ROW),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Struct {
            name: TypeName::from_stored(name, RuleSet::Earlier).unwrap(),
            revision: 1,
            supersedes: None,
            members: members
                .iter()
                .map(|member| TypeMember {
                    name: MemberName::parse(member).unwrap(),
                    type_id: int,
                })
                .collect(),
        },
    };
    PlcType::from_stored(record, RuleSet::Earlier).unwrap()
}

/// The array `ARRAY[0..7] OF <the stored struct>`.
fn array_row(platform: PlcPlatform) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(ARRAY_ROW),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Array {
            element: tid(STRUCT_ROW),
            dimensions: ArrayDimensions::parse(vec![ArrayDimension::parse(0, 7).unwrap()]).unwrap(),
        },
    })
    .unwrap()
}

/// Store the parameter `tag` with the key `key`, the type row `row`, and the `value`.
fn seed_parameter(
    (fixture, version): (&Fixture, RecipeVersionId),
    platform: PlcPlatform,
    (key, tag, row): (u128, &str, u128),
    value: Value,
) {
    let parameter = StoredParameter {
        plc_type: tid(row),
        limits: Vec::new(),
        value: StoredValue::from(value),
        ordinal: Ordinal::parse(u32::try_from(key).unwrap()).unwrap(),
        ..parameter_on(platform, key, tag, uid(101), 0)
    };
    fixture.store.seed_parameter(version, &parameter);
}

/// The reason of the `E-PLT-013` that the form gives, or `None` when the form opens.
async fn reason_of(
    fixture: &Fixture,
    purpose: ParameterFormPurpose,
    (key, type_text): (Option<u128>, &str),
) -> Option<DerivedTypeReason> {
    match form_as(
        fixture,
        Role::ProcessEngineer,
        input_as(purpose, key, type_text),
    )
    .await
    {
        Err(AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. })) => Some(reason),
        // An open form gives `None`. Any other error fails the test with its `Debug` text.
        other => other.map(|_form| None).unwrap(),
    }
}

/// The form that opens for the type text, with its preview tree.
async fn grid_of(fixture: &Fixture, key: u128, type_text: &str) -> ParameterForm {
    let input = input_as(UpdateParameterDefinition, Some(key), type_text);
    form_as(fixture, Role::ProcessEngineer, input)
        .await
        .unwrap()
}

/// The number of leaves in the preview tree of `form`.
fn leaf_count(form: &ParameterForm) -> usize {
    form.preview.as_ref().unwrap().leaves().len()
}

#[tokio::test]
async fn adr_0206_preview_of_a_missing_reserved_struct_gives_reserved_name() {
    let (fixture, _version) = world_on(PlcPlatform::CodesysIec, "M9");
    let reserved = Some(DerivedTypeReason::ReservedName);

    assert_eq!(
        reason_of(&fixture, AddParameter, (None, "DT@1")).await,
        reserved
    );
    assert_eq!(
        reason_of(&fixture, AddParameter, (None, "Int@9")).await,
        reserved
    );
    assert_eq!(
        reason_of(&fixture, AddParameter, (None, "ST_Zone@9")).await,
        Some(DerivedTypeReason::UnknownStruct)
    );
}

#[tokio::test]
async fn adr_0206_preview_of_a_stored_reserved_struct_needs_an_unchanged_text() {
    let (fixture, version) = world_on(PlcPlatform::CodesysIec, "M9");
    fixture
        .store
        .seed_type(&struct_row(PlcPlatform::CodesysIec, "DT", &["a", "b"]));
    fixture.store.seed_type(&array_row(PlcPlatform::CodesysIec));
    let at = (&fixture, version);
    let stamp = json!({"a": 1, "b": 2});
    seed_parameter(
        at,
        PlcPlatform::CodesysIec,
        (STRUCT_PARAMETER, "STAMP", STRUCT_ROW),
        stamp.clone(),
    );
    seed_parameter(
        at,
        PlcPlatform::CodesysIec,
        (ARRAY_PARAMETER, "STAMPS", ARRAY_ROW),
        Value::Array(vec![stamp; 8]),
    );
    let reserved = Some(DerivedTypeReason::ReservedName);

    assert_eq!(
        reason_of(&fixture, AddParameter, (None, "DT@1")).await,
        reserved
    );
    assert_eq!(
        reason_of(&fixture, UpdateParameterDefinition, (Some(START), "DT@1")).await,
        reserved
    );
    assert_eq!(
        leaf_count(&grid_of(&fixture, STRUCT_PARAMETER, "DT@1").await),
        2
    );
    // `expr_names_row` compares the canonical text, so the sent spelling does not matter.
    let spelled = grid_of(&fixture, ARRAY_PARAMETER, "ARRAY[-0..007] OF DT@1").await;
    assert_eq!(leaf_count(&spelled), 16);
}

#[tokio::test]
async fn adr_0206_preview_of_a_siemens_reserved_struct_on_s7_only() {
    let (s7, _version) = world_on(PlcPlatform::S7_1500, "M5");
    let (twincat, _version) = world_on(PlcPlatform::TwinCat3, "M8");

    assert_eq!(
        reason_of(&s7, AddParameter, (None, "BEGIN@9")).await,
        Some(DerivedTypeReason::ReservedName)
    );
    assert_eq!(
        reason_of(&twincat, AddParameter, (None, "BEGIN@9")).await,
        Some(DerivedTypeReason::UnknownStruct)
    );
}

#[tokio::test]
async fn adr_0203_preview_of_a_stored_reserved_revision_needs_an_unchanged_text() {
    let (fixture, version) = world_on(PlcPlatform::TwinCat3, "M8");
    fixture
        .store
        .seed_type(&struct_row(PlcPlatform::TwinCat3, "VERSION", &["a", "b"]));
    seed_parameter(
        (&fixture, version),
        PlcPlatform::TwinCat3,
        (STRUCT_PARAMETER, "VER", STRUCT_ROW),
        json!({"a": 1, "b": 2}),
    );
    let reserved = Some(DerivedTypeReason::ReservedName);

    assert_eq!(
        reason_of(&fixture, AddParameter, (None, "VERSION@1")).await,
        reserved
    );
    assert_eq!(
        reason_of(
            &fixture,
            UpdateParameterDefinition,
            (Some(START), "VERSION@1")
        )
        .await,
        reserved
    );
    assert_eq!(
        leaf_count(&grid_of(&fixture, STRUCT_PARAMETER, "VERSION@1").await),
        2
    );
}

#[tokio::test]
async fn adr_0203_preview_of_a_stored_keyword_member_needs_an_unchanged_text() {
    let (fixture, version) = world_on(PlcPlatform::CodesysIec, "M9");
    fixture.store.seed_type(&struct_row(
        PlcPlatform::CodesysIec,
        "ST_Coil",
        &["END_IF", "S"],
    ));
    seed_parameter(
        (&fixture, version),
        PlcPlatform::CodesysIec,
        (STRUCT_PARAMETER, "COIL", STRUCT_ROW),
        json!({"END_IF": 1, "S": 2}),
    );
    let reserved = Some(DerivedTypeReason::ReservedMemberName);

    assert_eq!(
        reason_of(&fixture, AddParameter, (None, "ST_Coil@1")).await,
        reserved
    );
    assert_eq!(
        reason_of(
            &fixture,
            UpdateParameterDefinition,
            (Some(START), "ST_Coil@1")
        )
        .await,
        reserved
    );
    assert_eq!(
        leaf_count(&grid_of(&fixture, STRUCT_PARAMETER, "ST_Coil@1").await),
        2
    );
}

#[tokio::test]
async fn adr_0204_preview_of_a_stored_one_member_struct_needs_an_unchanged_text() {
    let (fixture, version) = world_on(PlcPlatform::CodesysIec, "M9");
    fixture
        .store
        .seed_type(&struct_row(PlcPlatform::CodesysIec, "ST_One", &["a"]));
    seed_parameter(
        (&fixture, version),
        PlcPlatform::CodesysIec,
        (STRUCT_PARAMETER, "ONE", STRUCT_ROW),
        json!({"a": 1}),
    );
    let too_few = Some(DerivedTypeReason::TooFewMembers);

    assert_eq!(
        reason_of(&fixture, AddParameter, (None, "ST_One@1")).await,
        too_few
    );
    assert_eq!(
        reason_of(
            &fixture,
            UpdateParameterDefinition,
            (Some(START), "ST_One@1")
        )
        .await,
        too_few
    );
    assert_eq!(
        leaf_count(&grid_of(&fixture, STRUCT_PARAMETER, "ST_One@1").await),
        1
    );
}
