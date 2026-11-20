//! Tests of the rule "Reserved names on CODESYS and TwinCAT" in `UpdateParameterDefinition`
//! (ADR-0203 (i), (k)).

use recipe_domain::plant::{DerivedTypeReason, PlantError, PlcPlatform, Primitive};
use recipe_domain::recipe::{StoredParameter, StoredValue};
use serde_json::json;

use super::UpdateParameterDefinitionInput;
use super::requests::{run, speed_request, with_retype, with_type};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, parameter_on, text, tid, uid, vid};
use crate::recipe::rebind::fixtures::{primitive_id, typed_struct_row};
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot, stored_parameters};

const IEC_PLATFORMS: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];

/// A draft with the parameter `speed` and the stored `VERSION@1` and `ST_Coil@1`.
fn draft_with_speed(platform: PlcPlatform, speed: StoredParameter) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    let int = primitive_id(Primitive::Int);
    let rows = [
        typed_struct_row(platform, (980, "VERSION", 1), None, &[("a", int)]),
        typed_struct_row(
            platform,
            (981, "ST_Coil", 1),
            None,
            &[("END_IF", int), ("S", int)],
        ),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
    fixture
}

fn draft_with_int_speed(platform: PlcPlatform) -> Fixture {
    draft_with_speed(platform, parameter_on(platform, 1, "SPEED", uid(101), 0))
}

fn assert_reason(error: &AppError, expected: DerivedTypeReason) {
    assert!(
        matches!(
            error,
            AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. }) if *reason == expected
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn adr_0203_a_changed_type_with_a_stored_reserved_struct_gives_plt_013() {
    for platform in IEC_PLATFORMS {
        let fixture = draft_with_int_speed(platform);
        let before = DraftSnapshot::capture(&fixture);
        let request = with_retype(
            with_type(speed_request(), "VERSION@1"),
            "{\"a\":1}",
            Vec::new(),
        );
        let error = run(&fixture, request).await.unwrap_err();
        assert_reason(&error, DerivedTypeReason::ReservedName);
        before.assert_unchanged(&fixture);
        assert!(fixture.store.audit_entries().is_empty());
    }
}

#[tokio::test]
async fn adr_0203_a_changed_type_with_a_stored_keyword_member_gives_plt_013() {
    for platform in IEC_PLATFORMS {
        let fixture = draft_with_int_speed(platform);
        let before = DraftSnapshot::capture(&fixture);
        let request = with_retype(
            with_type(speed_request(), "ST_Coil@1"),
            "{\"END_IF\":1,\"S\":2}",
            Vec::new(),
        );
        let error = run(&fixture, request).await.unwrap_err();
        assert_reason(&error, DerivedTypeReason::ReservedMemberName);
        before.assert_unchanged(&fixture);
        assert!(fixture.store.audit_entries().is_empty());
    }
}

#[tokio::test]
async fn adr_0203_a_changed_type_with_a_stored_reserved_struct_on_s7_succeeds() {
    let fixture = draft_with_int_speed(PlcPlatform::S7_1500);
    let request = with_retype(
        with_type(speed_request(), "VERSION@1"),
        "{\"a\":1}",
        Vec::new(),
    );
    run(&fixture, request).await.unwrap();
    assert_eq!(
        stored_parameters(&fixture).first().unwrap().plc_type,
        tid(980)
    );
}

#[tokio::test]
async fn adr_0203_k_an_unchanged_type_keeps_a_stored_reserved_struct() {
    for (platform, struct_type, value) in [
        (PlcPlatform::TwinCat3, (980, "VERSION@1"), json!({"a": 1})),
        (
            PlcPlatform::CodesysIec,
            (981, "ST_Coil@1"),
            json!({"END_IF": 1, "S": 2}),
        ),
    ] {
        let speed = StoredParameter {
            plc_type: tid(struct_type.0),
            value: StoredValue::from(value),
            limits: Vec::new(),
            ..parameter_on(platform, 1, "SPEED", uid(101), 0)
        };
        let fixture = draft_with_speed(platform, speed);
        let request = UpdateParameterDefinitionInput {
            name_en: text("Renamed"),
            ..with_type(speed_request(), struct_type.1)
        };
        run(&fixture, request).await.unwrap();
        let stored = stored_parameters(&fixture);
        let parameter = stored.first().unwrap();
        assert_eq!(parameter.plc_type, tid(struct_type.0), "{platform:?}");
        assert_eq!(parameter.name_en, text("Renamed"), "{platform:?}");
    }
}
