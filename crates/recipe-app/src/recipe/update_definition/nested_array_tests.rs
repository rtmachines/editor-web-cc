//! Tests of the rule "no array of an array on S7-1200 and S7-1500" in `UpdateParameterDefinition`
//! (ADR-0148, ADR-0120).

use std::collections::BTreeSet;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, DerivedTypeReason, PlantError, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, TypeExpr,
};
use recipe_domain::recipe::{ParameterTag, StoredParameter, StoredValue};
use serde_json::json;

use super::UpdateParameterDefinitionInput;
use super::requests::{run, speed_request, with_retype, with_type};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, mid, parameter_on, text, tid, uid, vid};
use crate::memory::seed_type_id;
use crate::recipe::test_support::{
    DRAFT_VERSION, DraftSnapshot, seed_speed_draft, speed_key, stored_parameters, zone_key,
};

const NESTED_INT: &str = "ARRAY[1..2] OF ARRAY[1..2] OF INT";

/// A draft of a machine on `platform` with the parameter `SPEED` of type `INT`.
fn draft_on(platform: PlcPlatform) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let speed = parameter_on(platform, 1, "SPEED", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    fixture
}

/// An array row of the machine 1 with the bounds `1..=upper`.
fn array_row(platform: PlcPlatform, id: u128, element: PlcTypeId, upper: i32) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Array {
            element,
            dimensions: ArrayDimensions::parse(vec![ArrayDimension::parse(1, upper).unwrap()])
                .unwrap(),
        },
    })
    .unwrap()
}

/// The error of a type that the platform does not have: `E-PLT-013`, not on the platform.
fn assert_refuses_nesting(error: &AppError) {
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(matches!(
        error,
        AppError::Plant(PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::NotOnPlatform,
            ..
        })
    ));
}

fn speed_type(fixture: &Fixture) -> PlcTypeId {
    stored_parameters(fixture)
        .into_iter()
        .find(|row| row.key == speed_key())
        .unwrap()
        .plc_type
}

async fn assert_changed_type_is_refused(platform: PlcPlatform) {
    let fixture = draft_on(platform);
    let before = DraftSnapshot::capture(&fixture);
    let types_before = fixture.store.state().types.len();
    let request = with_retype(
        with_type(speed_request(), NESTED_INT),
        "[[1,2],[3,4]]",
        Vec::new(),
    );
    let error = run(&fixture, request).await.unwrap_err();
    assert_refuses_nesting(&error);
    assert_eq!(
        speed_type(&fixture),
        seed_type_id(Primitive::Int, None).unwrap()
    );
    assert_eq!(fixture.store.state().types.len(), types_before);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn a_changed_type_to_an_array_of_an_array_on_s7_1500_gives_plt_013_and_keeps_the_type() {
    assert_changed_type_is_refused(PlcPlatform::S7_1500).await;
}

#[tokio::test]
async fn a_changed_type_to_an_array_of_an_array_on_s7_1200_gives_plt_013_and_keeps_the_type() {
    assert_changed_type_is_refused(PlcPlatform::S7_1200).await;
}

#[tokio::test]
async fn a_changed_type_to_an_array_with_two_dimensions_on_s7_1500_succeeds() {
    let fixture = draft_on(PlcPlatform::S7_1500);
    let request = with_retype(
        with_type(speed_request(), "ARRAY[1..2,1..2] OF INT"),
        "[[1,2],[3,4]]",
        Vec::new(),
    );
    run(&fixture, request).await.unwrap();
    assert_ne!(
        speed_type(&fixture),
        seed_type_id(Primitive::Int, None).unwrap()
    );
}

#[tokio::test]
async fn a_changed_type_to_an_array_of_an_array_on_twincat_3_succeeds() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let request = with_retype(
        with_type(speed_request(), NESTED_INT),
        "[[1,2],[3,4]]",
        Vec::new(),
    );
    run(&fixture, request).await.unwrap();
    assert_ne!(
        speed_type(&fixture),
        seed_type_id(Primitive::Int, None).unwrap()
    );
}

/// The id of the stored type `ARRAY[1..2] OF ARRAY[1..3] OF INT`.
const STORED_NESTED: u128 = 812;

/// A draft on `S7_1500` with the parameter `CELLS` of the stored array of an array. The type rows
/// come from the memory adapter, as if they were stored before ADR-0148.
fn stored_nested_draft() -> Fixture {
    let fixture = draft_on(PlcPlatform::S7_1500);
    let int = seed_type_id(Primitive::Int, None).unwrap();
    fixture
        .store
        .seed_type(&array_row(PlcPlatform::S7_1500, 811, int, 3));
    fixture
        .store
        .seed_type(&array_row(PlcPlatform::S7_1500, STORED_NESTED, tid(811), 2));
    let cells = StoredParameter {
        plc_type: tid(STORED_NESTED),
        value: StoredValue::from(json!([[1, 2, 3], [4, 5, 6]])),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::S7_1500, 2, "CELLS", uid(101), 1)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &cells);
    fixture
}

fn cells_request() -> UpdateParameterDefinitionInput {
    UpdateParameterDefinitionInput {
        key: zone_key(),
        tag: ParameterTag::parse("CELLS").unwrap(),
        name_en: text("Renamed cells"),
        type_expr: TypeExpr::parse("ARRAY[1..2] OF ARRAY[1..3] OF INT").unwrap(),
        ordinal: recipe_domain::recipe::Ordinal::parse(1).unwrap(),
        ..speed_request()
    }
}

#[tokio::test]
async fn a_stored_array_of_an_array_stays_when_only_the_name_changes() {
    let fixture = stored_nested_draft();
    let types_before = fixture.store.state().types.len();
    let output = run(&fixture, cells_request()).await.unwrap();
    assert_eq!(output.row_version, 2);
    let row = stored_parameters(&fixture)
        .into_iter()
        .find(|row| row.key == zone_key())
        .unwrap();
    assert_eq!(row.name_en.as_str(), "Renamed cells");
    assert_eq!(row.plc_type, tid(STORED_NESTED));
    assert_eq!(fixture.store.state().types.len(), types_before);
}
