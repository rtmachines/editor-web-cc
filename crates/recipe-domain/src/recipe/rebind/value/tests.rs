use serde_json::{Value, json};

use super::super::test_support::{
    MODE_1, MODE_2, NAME_1, NAME_2, OLD_1, OLD_2, RAMP_1, RAMP_2, ZONE_2, ZONE_3, ZONES_2,
    draft_with, key, number, param, plan_of, planned_json, request, request_with,
    request_with_texts, trees,
};
use crate::error_codes::ErrorCode;
use crate::plant::{MemberName, Primitive};
use crate::recipe::{DraftError, Expected, ParameterTag, RecipeError};

/// The entries of a rejected plan, as text of code and message.
fn entries(error: DraftError) -> Vec<RecipeError> {
    match error {
        DraftError::List(list) => Some(list.into_vec()),
        DraftError::Recipe(_) | DraftError::Plant(_) => None,
    }
    .unwrap()
}

fn zone_draft() -> crate::recipe::Draft {
    draft_with(vec![param(
        1,
        "SPEED",
        ZONE_2,
        json!({"setpoint": 5}),
        Vec::new(),
    )])
}

#[test]
fn new_member_takes_its_value_from_the_map_and_kept_member_stays() {
    let draft = zone_draft();
    let request = request_with(ZONE_2, ZONE_3, &[(1, ".temp_max", number(70))]);

    let plan = plan_of(&draft, &request);

    assert_eq!(
        planned_json(&plan, 1),
        json!({"setpoint": 5, "temp_max": 70})
    );
}

#[test]
fn text_of_a_new_int_member_converts_by_the_class_of_the_leaf() {
    let draft = zone_draft();
    let request = request_with_texts(ZONE_2, ZONE_3, &[(1, ".temp_max", "70")]);

    let plan = plan_of(&draft, &request);

    assert_eq!(
        planned_json(&plan, 1),
        json!({"setpoint": 5, "temp_max": 70})
    );
}

#[test]
fn text_with_a_comma_for_a_new_int_member_expects_an_integer() {
    let draft = zone_draft();
    let request = request_with_texts(ZONE_2, ZONE_3, &[(1, ".temp_max", "1,5")]);

    let error = draft.plan_rebind(&request, &trees()).unwrap_err();

    let list = entries(error);
    assert_eq!(list.len(), 1);
    assert!(matches!(
        &list[0],
        RecipeError::ValueShapeMismatch {
            leaf,
            expected: Expected::Integer {
                type_name: Primitive::Int,
            },
            ..
        } if leaf.to_string() == ".temp_max"
    ));
}

#[test]
fn text_of_a_new_string_member_is_kept_as_sent() {
    let draft = draft_with(vec![param(
        1,
        "TITLE",
        NAME_1,
        json!({"setpoint": 3}),
        Vec::new(),
    )]);
    let request = request_with_texts(NAME_1, NAME_2, &[(1, ".label", " ab")]);

    let plan = plan_of(&draft, &request);

    assert_eq!(
        planned_json(&plan, 1),
        json!({"setpoint": 3, "label": " ab"})
    );
}

#[test]
fn missing_value_gives_one_entry_with_e_rcp_028() {
    let draft = zone_draft();

    let error = draft
        .plan_rebind(&request(ZONE_2, ZONE_3), &trees())
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_028);
    let list = entries(error);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].to_string(),
        "SPEED.temp_max is a new member. Give it a value."
    );
}

#[test]
fn two_parameters_without_values_give_two_entries_ordered_by_tag() {
    let draft = draft_with(vec![
        param(1, "TEMP", ZONE_2, json!({"setpoint": 1}), Vec::new()),
        param(2, "SPEED", ZONE_2, json!({"setpoint": 2}), Vec::new()),
    ]);

    let error = draft
        .plan_rebind(&request(ZONE_2, ZONE_3), &trees())
        .unwrap_err();

    let texts: Vec<String> = entries(error).iter().map(ToString::to_string).collect();
    assert_eq!(
        texts,
        vec![
            "SPEED.temp_max is a new member. Give it a value.",
            "TEMP.temp_max is a new member. Give it a value.",
        ]
    );
}

#[test]
fn json_string_for_an_int_member_gives_e_rcp_005() {
    let draft = zone_draft();
    let request = request_with(ZONE_2, ZONE_3, &[(1, ".temp_max", json!("hot"))]);

    let error = draft.plan_rebind(&request, &trees()).unwrap_err();

    let list = entries(error);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].code(), ErrorCode::RCP_005);
    let range = Expected::NumberRange {
        type_name: Primitive::Int,
        low: rust_decimal::Decimal::from(-32_768),
        high: rust_decimal::Decimal::from(32_767),
    };
    assert!(matches!(
        &list[0],
        RecipeError::ValueShapeMismatch { leaf, expected, .. }
            if leaf.to_string() == ".temp_max" && *expected == range
    ));
}

/// K7 (a): a sent value of a path that is no new leaf of the parameter is an entry, not dropped.
#[test]
fn value_of_an_unknown_member_path_gives_e_rcp_005() {
    let draft = zone_draft();
    let request = request_with(
        ZONE_2,
        ZONE_3,
        &[(1, ".temp_max", number(70)), (1, ".bogus", number(1))],
    );

    let error = draft.plan_rebind(&request, &trees()).unwrap_err();

    let list = entries(error);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].code(), ErrorCode::RCP_005);
    assert!(matches!(
        &list[0],
        RecipeError::ValueShapeMismatch {
            leaf,
            expected: Expected::KnownLeaf,
            ..
        } if leaf.to_string() == ".bogus"
    ));
}

#[test]
fn out_of_range_value_for_an_int_member_gives_e_rcp_005() {
    let draft = zone_draft();
    let request = request_with(ZONE_2, ZONE_3, &[(1, ".temp_max", number(70_000))]);

    let error = draft.plan_rebind(&request, &trees()).unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005);
}

#[test]
fn string_member_takes_a_string_and_rejects_a_long_one() {
    let draft = draft_with(vec![param(
        1,
        "TITLE",
        NAME_1,
        json!({"setpoint": 3}),
        Vec::new(),
    )]);

    let good = request_with(NAME_1, NAME_2, &[(1, ".label", json!("abc"))]);
    let plan = plan_of(&draft, &good);
    assert_eq!(
        planned_json(&plan, 1),
        json!({"setpoint": 3, "label": "abc"})
    );

    let bad = request_with(NAME_1, NAME_2, &[(1, ".label", json!("far too long text"))]);
    let error = draft.plan_rebind(&bad, &trees()).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
}

#[test]
fn kept_array_member_keeps_its_three_values() {
    let draft = draft_with(vec![param(
        1,
        "RAMP",
        RAMP_1,
        json!({"setpoint": 1, "ramps": [10, 20, 30]}),
        Vec::new(),
    )]);
    let request = request_with(RAMP_1, RAMP_2, &[(1, ".extra", number(4))]);

    let plan = plan_of(&draft, &request);

    assert_eq!(
        planned_json(&plan, 1),
        json!({"setpoint": 1, "ramps": [10, 20, 30], "extra": 4})
    );
}

#[test]
fn member_with_a_changed_type_is_new_and_needs_a_value() {
    let draft = draft_with(vec![param(
        1,
        "MODE",
        MODE_1,
        json!({"setpoint": 1, "mode": 2}),
        Vec::new(),
    )]);

    let error = draft
        .plan_rebind(&request(MODE_1, MODE_2), &trees())
        .unwrap_err();
    let list = entries(error);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].code(), ErrorCode::RCP_028);
    assert!(matches!(
        &list[0],
        RecipeError::RebindNeedsValue { leaf, .. } if leaf.to_string() == ".mode"
    ));

    let plan = plan_of(
        &draft,
        &request_with(MODE_1, MODE_2, &[(1, ".mode", number(70_000))]),
    );
    assert_eq!(
        planned_json(&plan, 1),
        json!({"setpoint": 1, "mode": 70_000})
    );
}

#[test]
fn removed_member_disappears_from_the_value() {
    let draft = draft_with(vec![param(
        1,
        "OLD",
        OLD_1,
        json!({"setpoint": 1, "old_max": 9}),
        Vec::new(),
    )]);

    let plan = plan_of(&draft, &request(OLD_1, OLD_2));

    assert_eq!(planned_json(&plan, 1), json!({"setpoint": 1}));
}

#[test]
fn array_of_the_struct_maps_each_element_with_indexed_keys() {
    let draft = draft_with(vec![param(
        1,
        "ZONES",
        ZONES_2,
        json!([{"setpoint": 1}, {"setpoint": 2}]),
        Vec::new(),
    )]);
    let values = [
        (1, "[1].temp_max", number(11)),
        (1, "[2].temp_max", number(22)),
    ];

    let plan = plan_of(&draft, &request_with(ZONE_2, ZONE_3, &values));

    assert_eq!(
        planned_json(&plan, 1),
        json!([{"setpoint": 1, "temp_max": 11}, {"setpoint": 2, "temp_max": 22}])
    );
}

#[test]
fn array_of_the_struct_reports_each_missing_element_in_order() {
    let draft = draft_with(vec![param(
        1,
        "ZONES",
        ZONES_2,
        json!([{"setpoint": 1}, {"setpoint": 2}]),
        Vec::new(),
    )]);
    let values = [(1, "[2].temp_max", number(22))];

    let error = draft
        .plan_rebind(&request_with(ZONE_2, ZONE_3, &values), &trees())
        .unwrap_err();

    let texts: Vec<String> = entries(error).iter().map(ToString::to_string).collect();
    assert_eq!(
        texts,
        vec!["ZONES[1].temp_max is a new member. Give it a value."]
    );
}

#[test]
fn stored_value_of_the_wrong_shape_gives_e_rcp_005() {
    let draft = draft_with(vec![param(1, "SPEED", ZONE_2, json!(5), Vec::new())]);

    let error = draft
        .plan_rebind(&request(ZONE_2, ZONE_3), &trees())
        .unwrap_err();

    let list = entries(error);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].code(), ErrorCode::RCP_005);
    let object = Expected::Object {
        members: vec![MemberName::parse("setpoint").unwrap()],
    };
    assert!(matches!(
        &list[0],
        RecipeError::ValueShapeMismatch { expected, .. } if *expected == object
    ));
}

#[test]
fn failing_plan_leaves_the_draft_unchanged() {
    let draft = zone_draft();
    let before = draft.clone();

    draft
        .plan_rebind(&request(ZONE_2, ZONE_3), &trees())
        .unwrap_err();

    assert_eq!(draft.parameters(), before.parameters());
    let stored: Value = draft.parameter(key(1)).unwrap().value().as_json().clone();
    assert_eq!(stored, json!({"setpoint": 5}));
}

#[test]
fn needs_value_has_its_code_registry_row_and_message() {
    let error = RecipeError::RebindNeedsValue {
        tag: ParameterTag::parse("SPEED").unwrap(),
        leaf: crate::plant::LeafPath::parse(".zones[2].temp_max").unwrap(),
    };

    assert_eq!(error.code(), ErrorCode::RCP_028);
    assert_eq!(error.code().as_str(), "E-RCP-028");
    assert!(crate::error_codes::REGISTRY.contains(&ErrorCode::RCP_028));
    assert_eq!(
        error.to_string(),
        "SPEED.zones[2].temp_max is a new member. Give it a value."
    );
}
