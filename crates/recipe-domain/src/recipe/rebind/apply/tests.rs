use std::collections::BTreeMap;

use serde_json::json;

use super::super::test_support::{
    BIG_1, BIG_2, BIG_ARRAY, BIG_ARRAY_NEW, DINT, FILL, INT, ZONE_2, ZONE_3, ZONES_2, ZONES_2_NEW,
    ZONES_4, ZONES_4_NEW, draft_with, key, limit, number, param, plan_of, request_with, trees,
    trees_with_zone_3,
};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{dimensions, id};
use crate::plant::{PlcTypeId, TypeTrees};
use crate::recipe::draft::support::stored_parameter;
use crate::recipe::{
    ArrayShape, Budget, Draft, DraftError, LangCode, ParameterKey, RebindPlan, RebindTargetReason,
    RecipeError, Translation,
};
use crate::text::Text;

fn translation() -> Translation {
    Translation {
        lang: LangCode::parse("pl").unwrap(),
        name: Text::parse("Predkosc").unwrap(),
        description: None,
    }
}

/// A draft with `SPEED` (struct, a limit, a translation) and `ZONES` (array of two).
fn zone_draft() -> Draft {
    let mut speed = param(
        1,
        "SPEED",
        ZONE_2,
        json!({"setpoint": 5}),
        vec![limit(".setpoint", 100)],
    );
    speed.translations = vec![translation()];
    let zones = param(
        2,
        "ZONES",
        ZONES_2,
        json!([{"setpoint": 1}, {"setpoint": 2}]),
        vec![limit("[*].setpoint", 50)],
    );
    draft_with(vec![speed, zones])
}

fn zone_plan(draft: &Draft) -> RebindPlan {
    let values = [
        (1, ".temp_max", number(70)),
        (2, "[1].temp_max", number(71)),
        (2, "[2].temp_max", number(72)),
    ];
    plan_of(draft, &request_with(ZONE_2, ZONE_3, &values))
}

fn new_types() -> BTreeMap<ParameterKey, PlcTypeId> {
    BTreeMap::from([(key(2), id(ZONES_2_NEW))])
}

fn mismatch(error: DraftError) -> bool {
    matches!(
        error,
        DraftError::Recipe(RecipeError::RebindTargetInvalid {
            reason: RebindTargetReason::PlanMismatch,
            ..
        })
    )
}

#[test]
fn complete_plan_changes_types_values_and_limits_and_keeps_translations() {
    let mut draft = zone_draft();
    let plan = zone_plan(&draft);

    let changes = draft.apply_rebind(plan, &new_types(), &trees()).unwrap();

    assert_eq!(changes.len(), 2);
    let speed = draft.parameter(key(1)).unwrap();
    assert_eq!(speed.plc_type(), id(ZONE_3));
    assert_eq!(
        speed.value().as_json(),
        &json!({"setpoint": 5, "temp_max": 70})
    );
    assert_eq!(speed.limits(), &[limit(".setpoint", 100)][..]);
    assert_eq!(speed.translations().len(), 1);
    let zones = draft.parameter(key(2)).unwrap();
    assert_eq!(zones.plc_type(), id(ZONES_2_NEW));
    assert_eq!(
        zones.value().as_json(),
        &json!([{"setpoint": 1, "temp_max": 71}, {"setpoint": 2, "temp_max": 72}])
    );
    assert_eq!(zones.limits(), &[limit("[*].setpoint", 50)][..]);
}

#[test]
fn changes_hold_the_old_and_the_new_parameter_in_tag_order() {
    let mut draft = zone_draft();
    let before = draft.clone();
    let plan = zone_plan(&draft);

    let changes = draft.apply_rebind(plan, &new_types(), &trees()).unwrap();

    let tags: Vec<String> = changes
        .iter()
        .map(|c| c.after().unwrap().tag().to_string())
        .collect();
    assert_eq!(tags, vec!["SPEED", "ZONES"]);
    assert_eq!(changes[0].before(), before.parameter(key(1)).ok());
    assert_eq!(changes[0].after(), draft.parameter(key(1)).ok());
}

#[test]
fn new_tree_that_differs_from_the_planned_tree_gives_plan_mismatch_and_no_change() {
    let mut draft = zone_draft();
    let before = draft.clone();
    let plan = zone_plan(&draft);
    let other = trees_with_zone_3(&[("setpoint", INT), ("temp_max", DINT)]);

    let error = draft.apply_rebind(plan, &new_types(), &other).unwrap_err();

    assert!(mismatch(error));
    assert_eq!(draft.parameters(), before.parameters());
}

#[test]
fn new_tree_with_another_member_name_gives_plan_mismatch() {
    let mut draft = zone_draft();
    let plan = zone_plan(&draft);
    let other = trees_with_zone_3(&[("setpoint", INT), ("temp_min", INT)]);

    let error = draft.apply_rebind(plan, &new_types(), &other).unwrap_err();

    assert!(mismatch(error));
}

#[test]
fn missing_new_array_type_gives_plan_mismatch_and_no_change() {
    let mut draft = zone_draft();
    let before = draft.clone();
    let plan = zone_plan(&draft);

    let error = draft
        .apply_rebind(plan, &BTreeMap::new(), &trees())
        .unwrap_err();

    assert!(mismatch(error));
    assert_eq!(draft.parameters(), before.parameters());
}

#[test]
fn new_array_type_with_other_dimensions_gives_plan_mismatch() {
    let mut draft = zone_draft();
    let plan = zone_plan(&draft);
    let wrong = BTreeMap::from([(key(2), id(ZONES_4_NEW))]);

    let error = draft.apply_rebind(plan, &wrong, &trees()).unwrap_err();

    assert!(mismatch(error));
}

#[test]
fn new_array_type_that_is_not_resolved_gives_plan_mismatch() {
    let mut draft = zone_draft();
    let plan = zone_plan(&draft);
    let unknown = BTreeMap::from([(key(2), id(99))]);

    let error = draft.apply_rebind(plan, &unknown, &trees()).unwrap_err();

    assert!(mismatch(error));
}

#[test]
fn parameter_that_is_gone_gives_plan_mismatch() {
    let plan = zone_plan(&zone_draft());
    let mut other = draft_with(vec![param(
        1,
        "SPEED",
        ZONE_2,
        json!({"setpoint": 5}),
        vec![limit(".setpoint", 100)],
    )]);
    let before = other.clone();

    let error = other
        .apply_rebind(plan, &new_types(), &trees())
        .unwrap_err();

    assert!(mismatch(error));
    assert_eq!(other.parameters(), before.parameters());
}

#[test]
fn parameter_with_another_type_than_the_plan_gives_plan_mismatch() {
    let plan = zone_plan(&zone_draft());
    let mut other = draft_with(vec![
        param(1, "SPEED", INT, json!(5), Vec::new()),
        param(
            2,
            "ZONES",
            ZONES_2,
            json!([{"setpoint": 1}, {"setpoint": 2}]),
            vec![limit("[*].setpoint", 50)],
        ),
    ]);

    let error = other
        .apply_rebind(plan, &new_types(), &trees())
        .unwrap_err();

    assert!(mismatch(error));
}

#[test]
fn trees_of_another_machine_give_e_plt_013() {
    let mut draft = zone_draft();
    let plan = zone_plan(&draft);
    let foreign = TypeTrees::resolve(
        [id(ZONE_3)],
        &crate::plant::fixtures::types_for(
            crate::plant::fixtures::other_machine(),
            crate::plant::PlcPlatform::TwinCat3,
            Vec::new(),
        ),
    );

    let error = draft
        .apply_rebind(plan, &new_types(), &foreign)
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_013);
}

/// A draft near the leaf budget: 15 fillers, the big array, and one small parameter.
fn near_budget_draft() -> Draft {
    let mut parameters: Vec<_> = (100..115)
        .map(|n| stored_parameter(n, &format!("FILL_{n}"), id(FILL), None))
        .collect();
    let elements = vec![json!({"a": 0}); 8_192];
    parameters.push(param(1, "BIG", BIG_ARRAY, json!(elements), Vec::new()));
    parameters.push(stored_parameter(2, "SMALL", id(INT), None));
    draft_with(parameters)
}

fn big_plan(draft: &Draft) -> RebindPlan {
    let values: Vec<_> = (1..=8_192)
        .map(|index| (1, format!("[{index}].b"), number(0)))
        .collect();
    let borrowed: Vec<_> = values
        .iter()
        .map(|(n, path, value)| (*n, path.as_str(), value.clone()))
        .collect();
    plan_of(draft, &request_with(BIG_1, BIG_2, &borrowed))
}

#[test]
fn rebind_above_the_leaf_budget_gives_e_rcp_021_and_no_change() {
    let mut draft = near_budget_draft();
    let before = draft.clone();
    let plan = big_plan(&draft);
    let types = BTreeMap::from([(key(1), id(BIG_ARRAY_NEW))]);

    let error = draft.apply_rebind(plan, &types, &trees()).unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_021);
    assert!(matches!(
        error,
        DraftError::Recipe(RecipeError::SizeBudgetExceeded {
            budget: Budget::Leaves,
            limit: 262_144,
            actual: 262_145,
        })
    ));
    assert_eq!(draft.parameters(), before.parameters());
}

#[test]
fn rebind_that_fills_the_leaf_budget_exactly_applies() {
    let mut parameters: Vec<_> = (100..115)
        .map(|n| stored_parameter(n, &format!("FILL_{n}"), id(FILL), None))
        .collect();
    let elements = vec![json!({"a": 0}); 8_192];
    parameters.push(param(1, "BIG", BIG_ARRAY, json!(elements), Vec::new()));
    let mut draft = draft_with(parameters);
    let plan = big_plan(&draft);
    let types = BTreeMap::from([(key(1), id(BIG_ARRAY_NEW))]);

    let applied = draft.apply_rebind(plan, &types, &trees());

    assert!(applied.is_ok());
}

#[test]
fn array_shapes_lists_each_shape_once() {
    let draft = draft_with(vec![
        param(
            1,
            "ZONES_A",
            ZONES_4,
            json!([{"setpoint": 1}, {"setpoint": 1}, {"setpoint": 1}, {"setpoint": 1}]),
            Vec::new(),
        ),
        param(
            2,
            "ZONES_B",
            ZONES_4,
            json!([{"setpoint": 2}, {"setpoint": 2}, {"setpoint": 2}, {"setpoint": 2}]),
            Vec::new(),
        ),
        param(3, "ZONE", ZONE_2, json!({"setpoint": 3}), Vec::new()),
    ]);
    let mut values = vec![(3, ".temp_max".to_owned(), number(0))];
    for n in [1, 2] {
        for index in 1..=4 {
            values.push((n, format!("[{index}].temp_max"), number(0)));
        }
    }
    let borrowed: Vec<_> = values
        .iter()
        .map(|(n, path, value)| (*n, path.as_str(), value.clone()))
        .collect();

    let plan = plan_of(&draft, &request_with(ZONE_2, ZONE_3, &borrowed));

    let shapes = plan.array_shapes();
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].dimensions(), &[dimensions(&[(1, 4)])][..]);
    assert_eq!(shapes[0].element(), id(ZONE_3));
}

#[test]
fn array_shapes_of_a_plan_without_arrays_is_empty() {
    let draft = draft_with(vec![param(
        1,
        "ZONE",
        ZONE_2,
        json!({"setpoint": 3}),
        Vec::new(),
    )]);
    let plan = plan_of(
        &draft,
        &request_with(ZONE_2, ZONE_3, &[(1, ".temp_max", number(0))]),
    );

    assert_eq!(plan.array_shapes(), Vec::<ArrayShape>::new());
}

#[test]
fn array_shapes_are_ordered_and_differ_by_dimensions() {
    let draft = draft_with(vec![
        param(
            1,
            "FOUR",
            ZONES_4,
            json!([{"setpoint": 1}, {"setpoint": 1}, {"setpoint": 1}, {"setpoint": 1}]),
            Vec::new(),
        ),
        param(
            2,
            "TWO",
            ZONES_2,
            json!([{"setpoint": 2}, {"setpoint": 2}]),
            Vec::new(),
        ),
    ]);
    let mut values = Vec::new();
    for index in 1..=4 {
        values.push((1, format!("[{index}].temp_max"), number(0)));
    }
    for index in 1..=2 {
        values.push((2, format!("[{index}].temp_max"), number(0)));
    }
    let borrowed: Vec<_> = values
        .iter()
        .map(|(n, path, value)| (*n, path.as_str(), value.clone()))
        .collect();

    let plan = plan_of(&draft, &request_with(ZONE_2, ZONE_3, &borrowed));

    let shapes = plan.array_shapes();
    assert_eq!(shapes.len(), 2);
    assert!(shapes[0] < shapes[1]);
}

#[test]
fn plan_mismatch_has_its_phrase() {
    assert_eq!(
        RebindTargetReason::PlanMismatch.to_string(),
        "the plan does not match the draft or the new types"
    );
}
