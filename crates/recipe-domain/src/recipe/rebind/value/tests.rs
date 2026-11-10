use serde_json::{Value, json};

use super::super::build::visit_elements;
use super::super::test_support::{
    GRID, MODE_1, MODE_2, NAME_1, NAME_2, OLD_1, OLD_2, RAMP_1, RAMP_2, ZONE_2, ZONE_3, ZONES_2,
    draft_with, key, number, param, plan_of, planned_json, request, request_with,
    request_with_texts, trees,
};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::dimensions;
use crate::plant::{LeafSegment, MemberName, Primitive};
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

/// ADR-0137: U+0000 is no part of a `STRING`, so the plan fails and the draft stays as it is.
#[test]
fn string_member_rejects_a_nul_with_e_rcp_005() {
    let draft = draft_with(vec![param(
        1,
        "TITLE",
        NAME_1,
        json!({"setpoint": 3}),
        Vec::new(),
    )]);
    let before = draft.clone();

    let bad = request_with(NAME_1, NAME_2, &[(1, ".label", json!("\u{0}"))]);
    let error = draft.plan_rebind(&bad, &trees()).unwrap_err();

    let list = entries(error);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].code(), ErrorCode::RCP_005);
    assert!(matches!(
        &list[0],
        RecipeError::ValueShapeMismatch { leaf, expected: Expected::String { .. }, .. }
            if leaf.to_string() == ".label"
    ));
    assert_eq!(draft.parameters(), before.parameters());
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

/// A draft with `GRID`, an array with two dimensions of `ST_Zone@2`.
fn grid_draft() -> crate::recipe::Draft {
    draft_with(vec![param(
        1,
        "GRID",
        GRID,
        json!([[{"setpoint": 1}, {"setpoint": 2}], [{"setpoint": 3}, {"setpoint": 4}]]),
        Vec::new(),
    )])
}

// ADR-0018 and domain-model.md section 2 (leaf path grammar): an array with two dimensions
// reports each new leaf with the index tuple `[row,column]` in row-major order.
#[test]
fn array_with_two_dimensions_reports_each_missing_element_in_row_major_order() {
    let error = grid_draft()
        .plan_rebind(&request(ZONE_2, ZONE_3), &trees())
        .unwrap_err();

    let list = entries(error);
    let leaves: Vec<String> = list
        .iter()
        .filter_map(|entry| match entry {
            RecipeError::RebindNeedsValue { tag, leaf } if tag.as_str() == "GRID" => {
                Some(leaf.to_string())
            }
            _other => None,
        })
        .collect();
    assert_eq!(leaves.len(), list.len());
    assert_eq!(
        leaves,
        [
            "[1,1].temp_max",
            "[1,2].temp_max",
            "[2,1].temp_max",
            "[2,2].temp_max"
        ]
    );
    for entry in &list {
        assert_eq!(entry.code(), ErrorCode::RCP_028);
    }
}

// ADR-0018: the value of an array with two dimensions keeps its nested shape, and each new
// member takes the value that the request gives for its index tuple.
#[test]
fn array_with_two_dimensions_keeps_the_nested_shape_and_maps_each_element() {
    let values = [
        (1, "[1,1].temp_max", number(1)),
        (1, "[1,2].temp_max", number(2)),
        (1, "[2,1].temp_max", number(3)),
        (1, "[2,2].temp_max", number(4)),
    ];

    let plan = plan_of(&grid_draft(), &request_with(ZONE_2, ZONE_3, &values));

    assert_eq!(
        planned_json(&plan, 1),
        json!([
            [{"setpoint": 1, "temp_max": 1}, {"setpoint": 2, "temp_max": 2}],
            [{"setpoint": 3, "temp_max": 3}, {"setpoint": 4, "temp_max": 4}]
        ])
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

/// Visit `value` along `bounds` and return the result with the visited elements and prefixes.
fn visited(
    value: &mut Value,
    bounds: &[(i32, i32)],
) -> (Option<()>, Vec<(Value, Vec<LeafSegment>)>) {
    let chain = dimensions(bounds);
    let mut calls = Vec::new();
    let result = visit_elements(value, &[&chain], &mut |element, prefix| {
        calls.push((element.clone(), prefix.to_vec()));
    });
    (result, calls)
}

// build.rs doc: `visit_elements` returns `None` when the value does not have the shape of the
// chain. A value that fails at the outermost array gets no visit. The map of a value checks the
// shape first, so the test injects that fault.
#[test]
fn visit_elements_of_a_value_that_is_no_array_gives_none_and_no_visit() {
    let (result, calls) = visited(&mut json!({"setpoint": 1}), &[(1, 2)]);

    assert_eq!(result, None);
    assert_eq!(calls, Vec::new());
}

#[test]
fn visit_elements_of_an_array_with_the_wrong_element_count_gives_none_and_no_visit() {
    let (short, short_calls) = visited(&mut json!([{"setpoint": 1}]), &[(1, 2)]);
    let (long, long_calls) = visited(&mut json!([1, 2, 3]), &[(1, 2)]);

    assert_eq!(short, None);
    assert_eq!(short_calls, Vec::new());
    assert_eq!(long, None);
    assert_eq!(long_calls, Vec::new());
}

// domain-model.md section 2 (leaf path grammar): an array with two dimensions has the index
// tuple `[row,column]` with the PLC indexes. Elements come in row-major order.
#[test]
fn visit_elements_walks_two_dimensions_in_row_major_order_with_plc_indexes() {
    let mut value = json!([[1, 2], [3, 4]]);

    let (result, calls) = visited(&mut value, &[(5, 6), (1, 2)]);

    assert_eq!(result, Some(()));
    let expected: Vec<(Value, Vec<LeafSegment>)> = [
        (1, vec![5, 1]),
        (2, vec![5, 2]),
        (3, vec![6, 1]),
        (4, vec![6, 2]),
    ]
    .into_iter()
    .map(|(element, indexes)| (json!(element), vec![LeafSegment::Index(indexes)]))
    .collect();
    assert_eq!(calls, expected);
}

#[test]
fn visit_elements_of_a_row_with_the_wrong_element_count_gives_none() {
    let (result, calls) = visited(&mut json!([[1, 2], [3]]), &[(1, 2), (1, 2)]);

    assert_eq!(result, None);
    // The walk is depth first. It visits the elements of the full first row before it finds the
    // short second row.
    assert_eq!(calls.len(), 2);
}
