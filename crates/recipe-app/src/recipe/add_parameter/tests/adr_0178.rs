//! Tests of `AddParameter` for the rules "array bounds of a parameter with an S7 address"
//! (ADR-0178) and "BOOL array with three dimensions and an S7 address" (ADR-0187).

use recipe_domain::plant::{PlcPlatform, S7Address};
use recipe_domain::recipe::{ParameterTag, S7AccessReason, StoredParameter, StoredValue};
use serde_json::Value;

use super::{AddParameterInput, assert_nothing_written, empty_draft, input, int_input, run};
use crate::memory::fixtures::{Fixture, parameter_on, tid, uid, vid};
use crate::recipe::test_support::{
    DRAFT_VERSION, WIDE_ARRAY, assert_not_addressable, bool_cube, byte_elements_limit,
    seed_standard_access_rows, stored_parameters, wide_array_value, wide_struct_value,
};

const WIDE_REASON: S7AccessReason = S7AccessReason::ArrayBoundOutsideInt;
const CUBE_REASON: S7AccessReason = S7AccessReason::BoolArrayWithThreeDimensions;

/// A draft of a machine on `platform` with the rows of the two rules.
fn draft_with_rows(platform: PlcPlatform) -> Fixture {
    let fixture = empty_draft(platform);
    seed_standard_access_rows(&fixture, platform);
    fixture
}

/// A request for the tag `BUF` with the type text, the value, and the address `DB1.DBB0`.
fn buf_request(type_text: &str, value: &Value, address: Option<&str>) -> AddParameterInput {
    AddParameterInput {
        tag: ParameterTag::parse("BUF").unwrap(),
        s7_address: address.map(|text| S7Address::parse(text).unwrap()),
        ..input(type_text, &value.to_string(), Vec::new())
    }
}

#[tokio::test]
async fn adr_0178_a_bound_above_32767_with_an_address_gives_rcp_058_and_adds_nothing() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let fixture = draft_with_rows(platform);
        let request = buf_request(
            "ARRAY[32760..32770] OF BYTE",
            &wide_array_value(),
            Some("DB1.DBB0"),
        );
        let error = run(&fixture, request).await.unwrap_err();
        assert_not_addressable(&error, "BUF", WIDE_REASON);
        assert!(stored_parameters(&fixture).is_empty(), "{platform:?}");
        assert_nothing_written(&fixture);
    }
}

#[tokio::test]
async fn adr_0178_a_bound_above_32767_in_a_struct_member_gives_rcp_058() {
    let fixture = draft_with_rows(PlcPlatform::S7_300);
    let request = buf_request("ST_Buf@1", &wide_struct_value(), Some("DB1.DBB0"));
    let error = run(&fixture, request).await.unwrap_err();
    assert_not_addressable(&error, "BUF", WIDE_REASON);
    assert!(stored_parameters(&fixture).is_empty());
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn adr_0178_a_bound_above_32767_without_an_address_adds_the_parameter() {
    let fixture = draft_with_rows(PlcPlatform::S7_1500);
    let request = buf_request("ARRAY[32760..32770] OF BYTE", &wide_array_value(), None);
    run(&fixture, request).await.unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}

/// `BUF` stored before ADR-0178: the type [`WIDE_ARRAY`] at `DB1.DBB0` on the unit 102.
fn stored_wide_parameter() -> StoredParameter {
    StoredParameter {
        plc_type: tid(WIDE_ARRAY),
        value: StoredValue::from(wide_array_value()),
        limits: vec![byte_elements_limit()],
        s7_address: Some(S7Address::parse("DB1.DBB0").unwrap()),
        ..parameter_on(PlcPlatform::S7_300, 5, "BUF", uid(102), 0)
    }
}

#[tokio::test]
async fn adr_0178_a_stored_bound_above_32767_of_another_parameter_does_not_block_the_command() {
    let fixture = draft_with_rows(PlcPlatform::S7_300);
    fixture
        .store
        .seed_parameter(vid(DRAFT_VERSION), &stored_wide_parameter());
    let request = AddParameterInput {
        tag: ParameterTag::parse("MODE").unwrap(),
        s7_address: Some(S7Address::parse("DB2.DBW0").unwrap()),
        ..int_input()
    };
    run(&fixture, request).await.unwrap();
    let tags: Vec<String> = stored_parameters(&fixture)
        .iter()
        .map(|row| row.tag.as_str().to_owned())
        .collect();
    assert_eq!(tags.len(), 2);
    assert!(tags.contains(&"MODE".to_owned()));
}

#[tokio::test]
async fn adr_0187_a_bool_array_with_three_dimensions_and_an_address_gives_rcp_058() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let fixture = draft_with_rows(platform);
        let request = buf_request(
            "ARRAY[1..3,1..3,1..3] OF BOOL",
            &bool_cube(3),
            Some("DB1.DBB0"),
        );
        let error = run(&fixture, request).await.unwrap_err();
        assert_not_addressable(&error, "BUF", CUBE_REASON);
        assert!(stored_parameters(&fixture).is_empty(), "{platform:?}");
        assert_nothing_written(&fixture);
    }
}

#[tokio::test]
async fn adr_0187_a_struct_member_of_a_bool_array_with_three_dimensions_gives_rcp_058() {
    let fixture = draft_with_rows(PlcPlatform::S7_300);
    let value = serde_json::json!({"bits": bool_cube(2)});
    let request = buf_request("ST_Cube@1", &value, Some("DB1.DBB0"));
    let error = run(&fixture, request).await.unwrap_err();
    assert_not_addressable(&error, "BUF", CUBE_REASON);
    assert!(stored_parameters(&fixture).is_empty());
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn adr_0187_a_bool_array_with_three_dimensions_without_an_address_adds_the_parameter() {
    let fixture = draft_with_rows(PlcPlatform::S7_1500);
    let request = buf_request("ARRAY[1..3,1..3,1..3] OF BOOL", &bool_cube(3), None);
    run(&fixture, request).await.unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}

#[tokio::test]
async fn adr_0187_a_bool_array_with_two_dimensions_and_an_address_adds_the_parameter() {
    let fixture = draft_with_rows(PlcPlatform::S7_300);
    let value = serde_json::json!([
        [false, false, false],
        [false, false, false],
        [false, false, false]
    ]);
    let request = buf_request("ARRAY[1..3,1..3] OF BOOL", &value, Some("DB1.DBB0"));
    run(&fixture, request).await.unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}
