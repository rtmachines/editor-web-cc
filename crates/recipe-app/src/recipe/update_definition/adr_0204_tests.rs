//! Tests of the rule "Two members of a struct on CODESYS" in `UpdateParameterDefinition`
//! (ADR-0204 (c), (e)).

use recipe_domain::plant::{DerivedTypeReason, PlantError, PlcPlatform, Primitive};
use recipe_domain::recipe::{StoredParameter, StoredValue};
use serde_json::json;

use super::UpdateParameterDefinitionInput;
use super::requests::{run, speed_request, with_retype, with_type};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, parameter_on, text, tid, uid, vid};
use crate::recipe::rebind::fixtures::{primitive_id, typed_struct_row};
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot, stored_parameters};

const ONE: u128 = 982;
const PAIR: u128 = 983;

/// A draft with the parameter `speed` and the stored `ST_One@1` and `ST_Pair@1`.
fn draft_with_speed(platform: PlcPlatform, speed: StoredParameter) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    let int = primitive_id(Primitive::Int);
    let rows = [
        typed_struct_row(platform, (ONE, "ST_One", 1), None, &[("a", int)]),
        typed_struct_row(
            platform,
            (PAIR, "ST_Pair", 1),
            None,
            &[("a", int), ("b", int)],
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

fn assert_too_few(error: &AppError) {
    assert!(
        matches!(
            error,
            AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. })
                if *reason == DerivedTypeReason::TooFewMembers
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn adr_0204_c2_a_changed_type_with_a_stored_one_member_struct_on_codesys_gives_plt_013() {
    for (type_text, value) in [
        ("ST_One@1", "{\"a\":1}"),
        ("ARRAY[1..2] OF ST_One@1", "[{\"a\":1},{\"a\":2}]"),
    ] {
        let fixture = draft_with_int_speed(PlcPlatform::CodesysIec);
        let before = DraftSnapshot::capture(&fixture);
        let request = with_retype(with_type(speed_request(), type_text), value, Vec::new());
        let error = run(&fixture, request).await.unwrap_err();
        assert_too_few(&error);
        before.assert_unchanged(&fixture);
        assert!(fixture.store.audit_entries().is_empty());
    }
}

#[tokio::test]
async fn adr_0204_c2_a_changed_type_with_a_stored_one_member_struct_on_twincat_succeeds() {
    let fixture = draft_with_int_speed(PlcPlatform::TwinCat3);
    let request = with_retype(
        with_type(speed_request(), "ST_One@1"),
        "{\"a\":1}",
        Vec::new(),
    );
    run(&fixture, request).await.unwrap();
    assert_eq!(
        stored_parameters(&fixture).first().unwrap().plc_type,
        tid(ONE)
    );
}

#[tokio::test]
async fn adr_0204_c2_a_changed_type_with_a_stored_two_member_struct_on_codesys_succeeds() {
    let fixture = draft_with_int_speed(PlcPlatform::CodesysIec);
    let request = with_retype(
        with_type(speed_request(), "ST_Pair@1"),
        "{\"a\":1,\"b\":2}",
        Vec::new(),
    );
    run(&fixture, request).await.unwrap();
    assert_eq!(
        stored_parameters(&fixture).first().unwrap().plc_type,
        tid(PAIR)
    );
}

#[tokio::test]
async fn adr_0204_e_an_unchanged_type_keeps_a_stored_one_member_struct_on_codesys() {
    let speed = StoredParameter {
        plc_type: tid(ONE),
        value: StoredValue::from(json!({"a": 1})),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::CodesysIec, 1, "SPEED", uid(101), 0)
    };
    let fixture = draft_with_speed(PlcPlatform::CodesysIec, speed);
    let request = UpdateParameterDefinitionInput {
        name_en: text("Renamed"),
        ..with_type(speed_request(), "ST_One@1")
    };
    run(&fixture, request).await.unwrap();
    let stored = stored_parameters(&fixture);
    let parameter = stored.first().unwrap();
    assert_eq!(parameter.plc_type, tid(ONE));
    assert_eq!(parameter.name_en, text("Renamed"));
}
