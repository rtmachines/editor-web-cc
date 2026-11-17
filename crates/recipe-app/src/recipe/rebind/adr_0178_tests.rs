//! Tests of `RebindTypeRevision` for the rules "array bounds of a parameter with an S7 address"
//! (ADR-0178) and "BOOL array with three dimensions and an S7 address" (ADR-0187).

use recipe_domain::plant::{PlcPlatform, Primitive, S7Address};
use recipe_domain::recipe::{S7AccessReason, StoredParameter, StoredValue};
use serde_json::json;

use super::RebindTypeRevisionInput;
use super::fixtures::{request, run, typed_struct_row, with_value};
use crate::memory::fixtures::{Fixture, parameter_on, struct_chain_row, tid, uid, vid};
use crate::memory::seed_type_id;
use crate::recipe::test_support::{
    CUBE_ARRAY, DRAFT_VERSION, DraftSnapshot, WIDE_MEMBER, assert_not_addressable,
    seed_standard_access_rows, stored_parameters,
};

// The rows of the four structs come from the range of `seed_struct_chain`. These tests seed no chain.
/// `ST_Wide@1` with the member `a`.
const WIDE_1: u128 = struct_chain_row(3);
/// `ST_Wide@2` with the members `a` and `data` of type `ARRAY[40000..40001] OF BYTE`.
const WIDE_2: u128 = struct_chain_row(4);
/// `ST_Cubed@1` with the member `a`.
const CUBED_1: u128 = struct_chain_row(5);
/// `ST_Cubed@2` with the members `a` and `bits` of type `ARRAY[1..2,1..2,1..2] OF BOOL`.
const CUBED_2: u128 = struct_chain_row(6);
/// The key number of the parameter that the tests rebind.
const PARAMETER: u128 = 11;

/// A draft of a machine on `platform` with the parameter `MIX` of `plc_type` at `address`.
fn draft_with_mix(platform: PlcPlatform, plc_type: u128, address: Option<&str>) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    seed_standard_access_rows(&fixture, platform);
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let rows = [
        typed_struct_row(platform, (WIDE_1, "ST_Wide", 1), None, &[("a", int)]),
        typed_struct_row(
            platform,
            (WIDE_2, "ST_Wide", 2),
            Some(WIDE_1),
            &[("a", int), ("data", tid(WIDE_MEMBER))],
        ),
        typed_struct_row(platform, (CUBED_1, "ST_Cubed", 1), None, &[("a", int)]),
        typed_struct_row(
            platform,
            (CUBED_2, "ST_Cubed", 2),
            Some(CUBED_1),
            &[("a", int), ("bits", tid(CUBE_ARRAY))],
        ),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
    let mix = StoredParameter {
        plc_type: tid(plc_type),
        value: StoredValue::from(json!({"a": 1})),
        limits: Vec::new(),
        s7_address: address.map(|text| S7Address::parse(text).unwrap()),
        ..parameter_on(platform, PARAMETER, "MIX", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &mix);
    fixture
}

/// The rebind of `ST_Wide@1` to `ST_Wide@2` with a value for each new byte.
fn wide_request() -> RebindTypeRevisionInput {
    let input = request(WIDE_1, WIDE_2);
    let input = with_value(input, (PARAMETER, ".data[40000]"), &json!(0));
    with_value(input, (PARAMETER, ".data[40001]"), &json!(0))
}

/// The rebind of `ST_Cubed@1` to `ST_Cubed@2` with a value for each new boolean.
fn cubed_request() -> RebindTypeRevisionInput {
    let mut input = request(CUBED_1, CUBED_2);
    for first in 1..=2 {
        for second in 1..=2 {
            for third in 1..=2 {
                let path = format!(".bits[{first},{second},{third}]");
                input = with_value(input, (PARAMETER, &path), &json!(false));
            }
        }
    }
    input
}

#[tokio::test]
async fn adr_0178_a_rebind_to_a_struct_with_a_wide_bound_gives_rcp_058_and_writes_nothing() {
    let fixture = draft_with_mix(PlcPlatform::S7_300, WIDE_1, Some("DB1.DBB0"));
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, wide_request()).await.unwrap_err();
    assert_not_addressable(&error, "MIX", S7AccessReason::ArrayBoundOutsideInt);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn adr_0178_a_rebind_to_a_struct_with_a_wide_bound_without_an_address_succeeds() {
    let fixture = draft_with_mix(PlcPlatform::S7_1500, WIDE_1, None);
    run(&fixture, wide_request()).await.unwrap();
    let row = stored_parameters(&fixture)
        .into_iter()
        .find(|row| row.tag.as_str() == "MIX")
        .unwrap();
    assert_eq!(row.plc_type, tid(WIDE_2));
}

#[tokio::test]
async fn adr_0187_a_rebind_to_a_struct_with_a_bool_cube_gives_rcp_058_and_writes_nothing() {
    let fixture = draft_with_mix(PlcPlatform::S7_300, CUBED_1, Some("DB1.DBB0"));
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, cubed_request()).await.unwrap_err();
    assert_not_addressable(&error, "MIX", S7AccessReason::BoolArrayWithThreeDimensions);
    before.assert_unchanged(&fixture);
}
