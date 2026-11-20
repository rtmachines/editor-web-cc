//! Tests of the rule "Siemens reserved struct names on S7" in `AddParameter` (ADR-0186).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{DerivedTypeReason, PlantError, PlcPlatform, Primitive};

use super::{AddParameterInput, assert_nothing_written, empty_draft, input, run};
use crate::error::AppError;
use crate::memory::fixtures::Fixture;
use crate::recipe::rebind::fixtures::{primitive_id, typed_struct_row};
use crate::recipe::test_support::stored_parameters;

/// Store `PID_Compact@1` with the members `a` and `b` on the machine 1.
fn seed_pid(fixture: &Fixture, platform: PlcPlatform) {
    let row = typed_struct_row(
        platform,
        (970, "PID_Compact", 1),
        None,
        &[
            ("a", primitive_id(Primitive::Int)),
            ("b", primitive_id(Primitive::Int)),
        ],
    );
    fixture.store.seed_type(&row);
}

fn pid_input(type_text: &str, value: &str) -> AddParameterInput {
    input(type_text, value, Vec::new())
}

fn assert_reserved_name(error: &AppError) {
    assert!(
        matches!(
            error,
            AppError::Plant(PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::ReservedName,
                ..
            })
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn adr_0186_a_stored_struct_with_a_siemens_name_on_s7_gives_plt_013() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let fixture = empty_draft(platform);
        seed_pid(&fixture, platform);
        let error = run(&fixture, pid_input("PID_Compact@1", "{\"a\":1,\"b\":2}"))
            .await
            .unwrap_err();
        assert_reserved_name(&error);
        assert_nothing_written(&fixture);
        assert!(stored_parameters(&fixture).is_empty());
    }
}

#[tokio::test]
async fn adr_0186_an_array_of_a_stored_struct_with_a_siemens_name_on_s7_gives_plt_013() {
    let fixture = empty_draft(PlcPlatform::S7_1500);
    seed_pid(&fixture, PlcPlatform::S7_1500);
    let error = run(
        &fixture,
        pid_input(
            "ARRAY[1..2] OF PID_Compact@1",
            "[{\"a\":1,\"b\":2},{\"a\":2}]",
        ),
    )
    .await
    .unwrap_err();
    assert_reserved_name(&error);
    assert_nothing_written(&fixture);
    assert!(stored_parameters(&fixture).is_empty());
}

#[tokio::test]
async fn adr_0186_a_stored_struct_with_a_siemens_name_on_codesys_iec_adds_the_parameter() {
    let fixture = empty_draft(PlcPlatform::CodesysIec);
    seed_pid(&fixture, PlcPlatform::CodesysIec);
    run(&fixture, pid_input("PID_Compact@1", "{\"a\":1,\"b\":2}"))
        .await
        .unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}

#[tokio::test]
async fn adr_0186_a_stale_row_version_comes_before_the_siemens_name_fault() {
    let fixture = empty_draft(PlcPlatform::S7_1500);
    seed_pid(&fixture, PlcPlatform::S7_1500);
    let request = AddParameterInput {
        expected_row_version: 7,
        ..pid_input("PID_Compact@1", "{\"a\":1,\"b\":2}")
    };
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    assert_nothing_written(&fixture);
    assert!(stored_parameters(&fixture).is_empty());
}
