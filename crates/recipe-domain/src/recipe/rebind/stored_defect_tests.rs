//! Tests of the stored defect in the rebind plan (ADR-0195 (l), (n), design D3).

use serde_json::json;

use super::test_support::{ZONE_2, ZONE_3, draft_with, key, limit, number, param, request, trees};
use crate::error_codes::ErrorCode;
use crate::recipe::DraftError;
use crate::recipe::draft::support::assert_stored;

#[test]
fn stored_value_of_the_wrong_shape_stops_the_plan_before_the_request_entries() {
    let draft = draft_with(vec![
        param(1, "A", ZONE_2, json!({"setpoint": 5}), Vec::new()),
        param(2, "B", ZONE_2, json!(5), Vec::new()),
    ]);

    let error = draft
        .plan_rebind(&request(ZONE_2, ZONE_3), &trees())
        .unwrap_err();

    assert_stored(&error, Some(key(2)), ErrorCode::RCP_005);
}

#[test]
fn stored_value_above_its_stored_limit_stops_the_plan_before_the_request_entries() {
    let draft = draft_with(vec![
        param(1, "A", ZONE_2, json!({"setpoint": 5}), Vec::new()),
        param(
            2,
            "B",
            ZONE_2,
            json!({"setpoint": 500}),
            vec![limit(".setpoint", 100)],
        ),
    ]);

    let error = draft
        .plan_rebind(&request(ZONE_2, ZONE_3), &trees())
        .unwrap_err();

    assert_stored(&error, Some(key(2)), ErrorCode::RCP_004);
}

#[test]
fn i7_defect_of_a_parameter_with_request_entries_gives_stored_and_no_list() {
    let draft = draft_with(vec![param(
        1,
        "SPEED",
        ZONE_2,
        json!({"setpoint": 500}),
        vec![limit(".setpoint", 100)],
    )]);

    let error = draft
        .plan_rebind(&request(ZONE_2, ZONE_3), &trees())
        .unwrap_err();

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_004);
}

#[test]
fn first_defect_in_tag_order_wins() {
    // Tag `A` has the higher key, so a walk by key reaches `B` first and gives another answer.
    let draft = draft_with(vec![
        param(
            1,
            "B",
            ZONE_2,
            json!({"setpoint": 500}),
            vec![limit(".setpoint", 100)],
        ),
        param(2, "A", ZONE_2, json!(5), Vec::new()),
    ]);

    let error = draft
        .plan_rebind(&request(ZONE_2, ZONE_3), &trees())
        .unwrap_err();

    assert_stored(&error, Some(key(2)), ErrorCode::RCP_005);
}

#[test]
fn list_with_only_request_entries_stays_a_list() {
    let draft = draft_with(vec![
        param(1, "A", ZONE_2, json!({"setpoint": 5}), Vec::new()),
        param(
            2,
            "B",
            ZONE_2,
            json!({"setpoint": 50}),
            vec![limit(".setpoint", 100)],
        ),
    ]);
    let request = super::test_support::request_with(ZONE_2, ZONE_3, &[(2, ".temp_max", number(7))]);

    let error = draft.plan_rebind(&request, &trees()).unwrap_err();

    assert!(matches!(error, DraftError::List(_)), "{error:?}");
    assert_eq!(error.code(), ErrorCode::RCP_028);
}
