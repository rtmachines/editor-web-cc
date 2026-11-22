//! Tests of `RebindTypeRevision` for the rule "no LF and no CR in a request text"
//! (ADR-0210 (a)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{PlcPlatform, Primitive};
use serde_json::json;

use super::fixtures::{
    empty_draft, primitive_id, request, run, seed_parameter, stored, typed_struct_row, with_text,
    with_value,
};
use crate::memory::fixtures::{Fixture, tid};
use crate::memory::seed_type_id;
use crate::recipe::test_support::DraftSnapshot;

/// `ST_Label@1` with the member `setpoint` (`INT`).
const LABEL_1: u128 = 960;
/// `ST_Label@2` with the members `setpoint` (`INT`) and `label` (`STRING(80)`).
const LABEL_2: u128 = 961;
/// The key number of the parameter `LABEL1` of the type `ST_Label@1`.
const LABEL_PARAMETER: u128 = 17;

/// A draft with the rows of `ST_Label` and the parameter `LABEL1` of the type `ST_Label@1`.
fn label_draft() -> Fixture {
    let fixture = empty_draft();
    let platform = PlcPlatform::TwinCat3;
    let int = primitive_id(Primitive::Int);
    let label = seed_type_id(Primitive::String, Some(80)).unwrap();
    let rows = [
        typed_struct_row(
            platform,
            (LABEL_1, "ST_Label", 1),
            None,
            &[("setpoint", int)],
        ),
        typed_struct_row(
            platform,
            (LABEL_2, "ST_Label", 2),
            Some(LABEL_1),
            &[("setpoint", int), ("label", label)],
        ),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
    seed_parameter(
        &fixture,
        platform,
        (LABEL_PARAMETER, "LABEL1"),
        (LABEL_1, json!({"setpoint": 1})),
        Vec::new(),
    );
    fixture
}

#[tokio::test]
async fn rebind_with_cr_in_a_new_member_gives_e_rcp_005() {
    let fixture = label_draft();
    let before = DraftSnapshot::capture(&fixture);
    let input = with_value(
        request(LABEL_1, LABEL_2),
        (LABEL_PARAMETER, ".label"),
        &json!("A\rB"),
    );
    let error = run(&fixture, input).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(stored(&fixture, LABEL_PARAMETER).plc_type, tid(LABEL_1));
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn rebind_with_lf_in_the_text_of_a_new_member_gives_e_rcp_005() {
    let fixture = label_draft();
    let before = DraftSnapshot::capture(&fixture);
    let input = with_text(
        request(LABEL_1, LABEL_2),
        (LABEL_PARAMETER, ".label"),
        "A\nB",
    );
    let error = run(&fixture, input).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    before.assert_unchanged(&fixture);
}
