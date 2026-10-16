//! Tests of `RebindTypeRevision` with the form text of a new member (ADR-0039 (f)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{PlcPlatform, Primitive};
use recipe_domain::recipe::{Expected, RecipeError};
use serde_json::json;

use super::fixtures::{
    empty_draft, error_lines, primitive_id, request, run, seed_parameter, stored, typed_struct_row,
    with_text,
};
use crate::error::AppError;
use crate::memory::fixtures::Fixture;
use crate::recipe::test_support::DraftSnapshot;

/// `ST_Hold@1` with the member `setpoint` (`INT`).
const HOLD_1: u128 = 940;
/// `ST_Hold@2` with the members `setpoint` (`INT`) and `hold` (`TIME`).
const HOLD_2: u128 = 941;
/// The key number of the parameter `HOLD1` of the type `ST_Hold@1`.
const HOLD_PARAMETER: u128 = 16;

/// A draft with the rows of `ST_Hold` and the parameter `HOLD1` of the type `ST_Hold@1`.
fn hold_draft() -> Fixture {
    let fixture = empty_draft();
    let platform = PlcPlatform::TwinCat3;
    let int = primitive_id(Primitive::Int);
    let time = primitive_id(Primitive::Time);
    let rows = [
        typed_struct_row(platform, (HOLD_1, "ST_Hold", 1), None, &[("setpoint", int)]),
        typed_struct_row(
            platform,
            (HOLD_2, "ST_Hold", 2),
            Some(HOLD_1),
            &[("setpoint", int), ("hold", time)],
        ),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
    seed_parameter(
        &fixture,
        platform,
        (HOLD_PARAMETER, "HOLD1"),
        (HOLD_1, json!({"setpoint": 1})),
        Vec::new(),
    );
    fixture
}

#[tokio::test]
async fn a_text_of_a_new_time_member_converts_by_the_class_of_the_leaf() {
    let fixture = hold_draft();
    let input = with_text(
        request(HOLD_1, HOLD_2),
        (HOLD_PARAMETER, ".hold"),
        "T#1m30s",
    );
    run(&fixture, input).await.unwrap();
    let after = stored(&fixture, HOLD_PARAMETER);
    assert_eq!(
        after.value.as_json(),
        &json!({"setpoint": 1, "hold": 90_000})
    );
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn a_bad_text_of_a_new_member_gives_the_line_of_the_domain_and_writes_nothing() {
    let fixture = hold_draft();
    let before = DraftSnapshot::capture(&fixture);
    let input = with_text(request(HOLD_1, HOLD_2), (HOLD_PARAMETER, ".hold"), "90000");
    let error = run(&fixture, input).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(error_lines(&error), ["E-RCP-005 '.hold'"]);
    assert!(
        matches!(&error, AppError::Recipes(list) if matches!(
            list.first(),
            RecipeError::ValueShapeMismatch {
                expected: Expected::Literal { type_name: Primitive::Time },
                ..
            }
        )),
        "{error:?}"
    );
    before.assert_unchanged(&fixture);
}
