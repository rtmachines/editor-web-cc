//! Tests of the rule "Reserved names on CODESYS and TwinCAT" in `AddParameter` (ADR-0203 (i)).

use recipe_domain::plant::{DerivedTypeReason, PlantError, PlcPlatform, Primitive};

use super::{AddParameterInput, assert_nothing_written, empty_draft, input, run};
use crate::error::AppError;
use crate::memory::fixtures::Fixture;
use crate::recipe::rebind::fixtures::{primitive_id, typed_struct_row};
use crate::recipe::test_support::stored_parameters;

const IEC_PLATFORMS: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];

/// Store `VERSION@1` with the member `a` on the machine 1.
fn seed_version(fixture: &Fixture, platform: PlcPlatform) {
    let row = typed_struct_row(
        platform,
        (980, "VERSION", 1),
        None,
        &[("a", primitive_id(Primitive::Int))],
    );
    fixture.store.seed_type(&row);
}

/// Store `ST_Coil@1` with the keyword members `END_IF` and `S` on the machine 1.
fn seed_coil(fixture: &Fixture, platform: PlcPlatform) {
    let int = primitive_id(Primitive::Int);
    let row = typed_struct_row(
        platform,
        (981, "ST_Coil", 1),
        None,
        &[("END_IF", int), ("S", int)],
    );
    fixture.store.seed_type(&row);
}

fn reason_of(error: &AppError) -> Option<DerivedTypeReason> {
    match error {
        AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. }) => Some(*reason),
        _ => None,
    }
}

fn version_input(type_text: &str, value: &str) -> AddParameterInput {
    input(type_text, value, Vec::new())
}

#[tokio::test]
async fn adr_0203_a_stored_reserved_struct_on_codesys_and_twincat_gives_plt_013() {
    for platform in IEC_PLATFORMS {
        let fixture = empty_draft(platform);
        seed_version(&fixture, platform);
        let error = run(&fixture, version_input("VERSION@1", "{\"a\":1}"))
            .await
            .unwrap_err();
        assert_eq!(reason_of(&error), Some(DerivedTypeReason::ReservedName));
        assert_nothing_written(&fixture);
        assert!(stored_parameters(&fixture).is_empty());
    }
}

#[tokio::test]
async fn adr_0203_a_stored_reserved_struct_as_array_element_gives_plt_013() {
    let fixture = empty_draft(PlcPlatform::TwinCat3);
    seed_version(&fixture, PlcPlatform::TwinCat3);
    let request = version_input("ARRAY[1..2] OF VERSION@1", "[{\"a\":1},{\"a\":2}]");
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(reason_of(&error), Some(DerivedTypeReason::ReservedName));
    assert_nothing_written(&fixture);
    assert!(stored_parameters(&fixture).is_empty());
}

#[tokio::test]
async fn adr_0203_a_valid_struct_name_with_a_stored_keyword_member_gives_plt_013() {
    for platform in IEC_PLATFORMS {
        let fixture = empty_draft(platform);
        seed_coil(&fixture, platform);
        let request = version_input("ST_Coil@1", "{\"END_IF\":1,\"S\":2}");
        let error = run(&fixture, request).await.unwrap_err();
        assert_eq!(
            reason_of(&error),
            Some(DerivedTypeReason::ReservedMemberName)
        );
        assert_nothing_written(&fixture);
        assert!(stored_parameters(&fixture).is_empty());
    }
}

#[tokio::test]
async fn adr_0203_e_a_stored_reserved_struct_on_s7_adds_the_parameter() {
    for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
        let fixture = empty_draft(platform);
        seed_version(&fixture, platform);
        run(&fixture, version_input("VERSION@1", "{\"a\":1}"))
            .await
            .unwrap();
        assert_eq!(stored_parameters(&fixture).len(), 1, "{platform:?}");
    }
}

#[tokio::test]
async fn adr_0203_e_a_stored_keyword_member_on_s7_adds_the_parameter() {
    let fixture = empty_draft(PlcPlatform::S7_1500);
    seed_coil(&fixture, PlcPlatform::S7_1500);
    run(
        &fixture,
        version_input("ST_Coil@1", "{\"END_IF\":1,\"S\":2}"),
    )
    .await
    .unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}
