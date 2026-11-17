//! Tests of `UpdateParameterDefinition` for the rules "array bounds of a parameter with an S7
//! address" (ADR-0178) and "BOOL array with three dimensions and an S7 address" (ADR-0187).

use recipe_domain::plant::{PlcPlatform, S7Address};
use recipe_domain::recipe::S7AccessReason;
use serde_json::Value;

use super::UpdateParameterDefinitionInput;
use super::requests::{run, speed_request, with_retype, with_type};
use crate::memory::fixtures::{Fixture, parameter_on, tid, uid, vid};
use crate::recipe::test_support::{
    DRAFT_VERSION, DraftSnapshot, WIDE_ARRAY, assert_not_addressable, bool_cube,
    byte_elements_limit, s7_draft_with_speed, seed_standard_access_rows, speed_key,
    stored_parameters, wide_array_value, wide_struct_value,
};

/// A draft of an `S7_300` machine with `SPEED` at `DB1.DBW0` and the rows of the two rules.
fn s7_300_draft() -> Fixture {
    let fixture = s7_draft_with_speed();
    seed_standard_access_rows(&fixture, PlcPlatform::S7_300);
    fixture
}

/// A request that changes `SPEED` to the type text and the value, with the address `DB1.DBB0`.
fn retype_request(type_text: &str, value: &Value, address: bool) -> UpdateParameterDefinitionInput {
    let request = UpdateParameterDefinitionInput {
        s7_address: address.then(|| S7Address::parse("DB1.DBB0").unwrap()),
        ..with_type(speed_request(), type_text)
    };
    with_retype(request, &value.to_string(), Vec::new())
}

/// Assert that the command gives `E-RCP-058` for `SPEED` and changes nothing.
async fn assert_refused(
    fixture: &Fixture,
    request: UpdateParameterDefinitionInput,
    reason: S7AccessReason,
) {
    let before = DraftSnapshot::capture(fixture);
    let error = run(fixture, request).await.unwrap_err();
    assert_not_addressable(&error, "SPEED", reason);
    before.assert_unchanged(fixture);
}

#[tokio::test]
async fn adr_0178_a_changed_type_with_a_bound_above_32767_and_an_address_gives_rcp_058() {
    let fixture = s7_300_draft();
    let request = retype_request("ARRAY[32760..32770] OF BYTE", &wide_array_value(), true);
    assert_refused(&fixture, request, S7AccessReason::ArrayBoundOutsideInt).await;
}

#[tokio::test]
async fn adr_0178_a_changed_type_with_a_wide_bound_in_a_struct_member_gives_rcp_058() {
    let fixture = s7_300_draft();
    let request = retype_request("ST_Buf@1", &wide_struct_value(), true);
    assert_refused(&fixture, request, S7AccessReason::ArrayBoundOutsideInt).await;
}

#[tokio::test]
async fn adr_0178_a_new_address_for_a_stored_wide_type_gives_rcp_058() {
    let fixture = s7_300_draft();
    let wide = tid(WIDE_ARRAY);
    let stored = recipe_domain::recipe::StoredParameter {
        plc_type: wide,
        value: recipe_domain::recipe::StoredValue::from(wide_array_value()),
        limits: vec![byte_elements_limit()],
        s7_address: None,
        ..parameter_on(PlcPlatform::S7_300, 6, "SPEED_B", uid(101), 40)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &stored);
    let request = UpdateParameterDefinitionInput {
        key: stored.key,
        tag: stored.tag.clone(),
        type_expr: recipe_domain::plant::TypeExpr::parse("ARRAY[32760..32770] OF BYTE").unwrap(),
        s7_address: Some(S7Address::parse("DB1.DBB0").unwrap()),
        ordinal: stored.ordinal,
        ..speed_request()
    };
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, request).await.unwrap_err();
    assert_not_addressable(&error, "SPEED_B", S7AccessReason::ArrayBoundOutsideInt);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn adr_0178_a_changed_type_with_a_wide_bound_and_no_address_succeeds() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let speed = parameter_on(PlcPlatform::S7_1500, 1, "SPEED", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    seed_standard_access_rows(&fixture, PlcPlatform::S7_1500);
    let request = retype_request("ARRAY[32760..32770] OF BYTE", &wide_array_value(), false);
    run(&fixture, request).await.unwrap();
    let row = stored_parameters(&fixture)
        .into_iter()
        .find(|row| row.key == speed_key())
        .unwrap();
    assert_ne!(row.plc_type, speed.plc_type);
}

#[tokio::test]
async fn adr_0178_a_stored_wide_bound_of_another_parameter_does_not_block_the_command() {
    let fixture = s7_300_draft();
    let stored = recipe_domain::recipe::StoredParameter {
        plc_type: tid(WIDE_ARRAY),
        value: recipe_domain::recipe::StoredValue::from(wide_array_value()),
        limits: vec![byte_elements_limit()],
        s7_address: Some(S7Address::parse("DB1.DBB16").unwrap()),
        ..parameter_on(PlcPlatform::S7_300, 5, "BUF", uid(101), 40)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &stored);
    let request = UpdateParameterDefinitionInput {
        name_en: crate::memory::fixtures::text("Renamed speed"),
        s7_address: Some(S7Address::parse("DB2.DBW0").unwrap()),
        ..speed_request()
    };
    let output = run(&fixture, request).await.unwrap();
    assert_eq!(output.row_version, 2);
}

#[tokio::test]
async fn adr_0187_a_changed_type_with_a_bool_array_with_three_dimensions_gives_rcp_058() {
    let fixture = s7_300_draft();
    let request = retype_request("ARRAY[1..3,1..3,1..3] OF BOOL", &bool_cube(3), true);
    assert_refused(
        &fixture,
        request,
        S7AccessReason::BoolArrayWithThreeDimensions,
    )
    .await;
}

#[tokio::test]
async fn adr_0187_a_changed_type_with_a_struct_member_of_a_bool_cube_gives_rcp_058() {
    let fixture = s7_300_draft();
    let value = serde_json::json!({"bits": bool_cube(2)});
    let request = retype_request("ST_Cube@1", &value, true);
    assert_refused(
        &fixture,
        request,
        S7AccessReason::BoolArrayWithThreeDimensions,
    )
    .await;
}

#[tokio::test]
async fn adr_0187_a_bool_array_with_two_dimensions_and_an_address_succeeds() {
    let fixture = s7_300_draft();
    let row = serde_json::json!([[false, false], [false, false]]);
    let request = retype_request("ARRAY[1..2,1..2] OF BOOL", &row, true);
    run(&fixture, request).await.unwrap();
}
