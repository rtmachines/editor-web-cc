//! Tests of the rule "Siemens reserved struct names on S7" in `UpdateParameterDefinition` (ADR-0186).

use recipe_domain::plant::{DerivedTypeReason, PlantError, PlcPlatform, Primitive, S7Address};
use recipe_domain::recipe::{StoredParameter, StoredValue};
use serde_json::json;

use super::UpdateParameterDefinitionInput;
use super::requests::{run, speed_request, with_retype, with_type};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, parameter_on, text, tid, uid, vid};
use crate::recipe::rebind::fixtures::{primitive_id, typed_struct_row};
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot, stored_parameters};

/// A draft of a machine on `platform` with the INT parameter `SPEED` and the stored `PID_Compact@1`.
fn draft_with_pid(platform: PlcPlatform) -> Fixture {
    draft_with_speed(platform, parameter_on(platform, 1, "SPEED", uid(101), 0))
}

/// A draft of a machine on `platform` with the parameter `speed` and the stored `PID_Compact@1`.
fn draft_with_speed(platform: PlcPlatform, speed: StoredParameter) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    let row = typed_struct_row(
        platform,
        (970, "PID_Compact", 1),
        None,
        &[("a", primitive_id(Primitive::Int))],
    );
    fixture.store.seed_type(&row);
    fixture
}

fn pid_request() -> UpdateParameterDefinitionInput {
    with_retype(
        with_type(speed_request(), "PID_Compact@1"),
        "{\"a\":1}",
        Vec::new(),
    )
}

#[tokio::test]
async fn adr_0186_update_definition_with_a_siemens_struct_on_s7_gives_plt_013() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let fixture = draft_with_pid(platform);
        let before = DraftSnapshot::capture(&fixture);
        let error = run(&fixture, pid_request()).await.unwrap_err();
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
        before.assert_unchanged(&fixture);
        assert!(fixture.store.audit_entries().is_empty());
    }
}

#[tokio::test]
async fn adr_0186_update_definition_with_a_siemens_struct_on_codesys_iec_succeeds() {
    let fixture = draft_with_pid(PlcPlatform::CodesysIec);
    run(&fixture, pid_request()).await.unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}

#[tokio::test]
async fn adr_0186_update_definition_renames_a_parameter_that_keeps_a_stored_siemens_struct_on_s7() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let speed = StoredParameter {
            s7_address: (platform == PlcPlatform::S7_300)
                .then(|| S7Address::parse("DB1.DBB0").unwrap()),
            plc_type: tid(970),
            value: StoredValue::from(json!({"a": 1})),
            limits: Vec::new(),
            ..parameter_on(platform, 1, "SPEED", uid(101), 0)
        };
        let fixture = draft_with_speed(platform, speed.clone());
        let request = UpdateParameterDefinitionInput {
            name_en: text("Renamed"),
            s7_address: speed.s7_address,
            ..with_type(speed_request(), "PID_Compact@1")
        };
        run(&fixture, request).await.unwrap();
        let stored = stored_parameters(&fixture);
        assert_eq!(stored.len(), 1);
        let parameter = stored.first().unwrap();
        assert_eq!(parameter.plc_type, tid(970));
        assert_eq!(parameter.name_en, text("Renamed"));
    }
}
