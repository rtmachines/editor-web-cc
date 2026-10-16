use serde_json::json;

use super::super::test_support::{
    MODE_1, MODE_2, OLD_1, OLD_2, OLDS_1, RAMP_1, RAMP_2, ZONE_2, ZONE_3, ZONES_2, draft_with, key,
    limit, number, param, plan_of, request, request_with, trees,
};
use crate::error_codes::ErrorCode;
use crate::plant::LeafPath;
use crate::recipe::{DraftError, ParameterTag, RecipeError};

/// The entries of a rejected plan.
fn entries(error: DraftError) -> Vec<RecipeError> {
    match error {
        DraftError::List(list) => Some(list.into_vec()),
        DraftError::Recipe(_) | DraftError::Plant(_) => None,
    }
    .unwrap()
}

fn texts(error: DraftError) -> Vec<String> {
    entries(error).iter().map(ToString::to_string).collect()
}

#[test]
fn limit_on_a_kept_member_stays_unchanged() {
    let kept = limit(".setpoint", 100);
    let draft = draft_with(vec![param(
        1,
        "SPEED",
        ZONE_2,
        json!({"setpoint": 5}),
        vec![kept.clone()],
    )]);
    let request = request_with(ZONE_2, ZONE_3, &[(1, ".temp_max", number(70))]);

    let plan = plan_of(&draft, &request);

    assert_eq!(plan.limits(key(1)), Some(&[kept][..]));
}

#[test]
fn parameter_without_limits_keeps_an_empty_list() {
    let draft = draft_with(vec![param(
        1,
        "SPEED",
        ZONE_2,
        json!({"setpoint": 5}),
        Vec::new(),
    )]);
    let request = request_with(ZONE_2, ZONE_3, &[(1, ".temp_max", number(70))]);

    let plan = plan_of(&draft, &request);

    assert_eq!(plan.limits(key(1)), Some(&[][..]));
    assert_eq!(plan.limits(key(2)), None);
}

#[test]
fn star_limit_on_a_kept_array_member_stays() {
    let kept = limit(".ramps[*]", 50);
    let draft = draft_with(vec![param(
        1,
        "RAMP",
        RAMP_1,
        json!({"setpoint": 1, "ramps": [1, 2, 3]}),
        vec![kept.clone()],
    )]);
    let request = request_with(RAMP_1, RAMP_2, &[(1, ".extra", number(4))]);

    let plan = plan_of(&draft, &request);

    assert_eq!(plan.limits(key(1)), Some(&[kept][..]));
}

#[test]
fn limit_of_an_array_root_on_a_kept_member_stays() {
    let star = limit("[*].setpoint", 100);
    let second = limit("[2].setpoint", 10);
    let draft = draft_with(vec![param(
        1,
        "ZONES",
        ZONES_2,
        json!([{"setpoint": 1}, {"setpoint": 2}]),
        vec![star.clone()],
    )]);
    let values = [
        (1, "[1].temp_max", number(1)),
        (1, "[2].temp_max", number(2)),
    ];
    let request = request_with(ZONE_2, ZONE_3, &values);

    let plan = plan_of(&draft, &request);
    assert_eq!(plan.limits(key(1)), Some(&[star][..]));

    let draft = draft_with(vec![param(
        1,
        "ZONES",
        ZONES_2,
        json!([{"setpoint": 1}, {"setpoint": 2}]),
        vec![second.clone()],
    )]);
    let plan = plan_of(&draft, &request);
    assert_eq!(plan.limits(key(1)), Some(&[second][..]));
}

#[test]
fn limit_on_a_removed_member_gives_e_rcp_029() {
    let draft = draft_with(vec![param(
        1,
        "OLD",
        OLD_1,
        json!({"setpoint": 1, "old_max": 9}),
        vec![limit(".old_max", 10)],
    )]);

    let error = draft
        .plan_rebind(&request(OLD_1, OLD_2), &trees())
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_029);
    assert_eq!(
        texts(error),
        vec![
            "OLD.old_max has a limit, but the new revision has no such member or gives the member another type. Delete the limit first. After the rebind, set the limit again."
        ]
    );
}

#[test]
fn star_limit_on_a_removed_member_of_an_array_root_gives_e_rcp_029() {
    let draft = draft_with(vec![param(
        1,
        "OLDS",
        OLDS_1,
        json!([{"setpoint": 1, "old_max": 9}, {"setpoint": 2, "old_max": 8}]),
        vec![limit("[*].old_max", 10)],
    )]);

    let error = draft
        .plan_rebind(&request(OLD_1, OLD_2), &trees())
        .unwrap_err();

    assert_eq!(
        texts(error),
        vec![
            "OLDS[*].old_max has a limit, but the new revision has no such member or gives the member another type. Delete the limit first. After the rebind, set the limit again."
        ]
    );
}

#[test]
fn limit_on_a_member_with_a_changed_type_gives_e_rcp_029() {
    let draft = draft_with(vec![param(
        1,
        "MODE",
        MODE_1,
        json!({"setpoint": 1, "mode": 2}),
        vec![limit(".mode", 5)],
    )]);
    let request = request_with(MODE_1, MODE_2, &[(1, ".mode", number(3))]);

    let error = draft.plan_rebind(&request, &trees()).unwrap_err();

    let list = entries(error);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].code(), ErrorCode::RCP_029);
    assert!(matches!(
        &list[0],
        RecipeError::RebindDropsLimit { leaf, .. } if *leaf == LeafPath::parse(".mode").unwrap()
    ));
}

#[test]
fn value_entry_and_limit_entry_of_one_parameter_both_appear() {
    let draft = draft_with(vec![param(
        1,
        "MODE",
        MODE_1,
        json!({"setpoint": 1, "mode": 2}),
        vec![limit(".mode", 5)],
    )]);

    let error = draft
        .plan_rebind(&request(MODE_1, MODE_2), &trees())
        .unwrap_err();

    let codes: Vec<ErrorCode> = entries(error).iter().map(RecipeError::code).collect();
    assert_eq!(codes, vec![ErrorCode::RCP_028, ErrorCode::RCP_029]);
}

#[test]
fn two_parameters_of_one_source_with_dropped_limits_give_two_entries() {
    let draft = draft_with(vec![
        param(
            1,
            "ZZ",
            OLD_1,
            json!({"setpoint": 1, "old_max": 9}),
            vec![limit(".old_max", 10)],
        ),
        param(
            2,
            "AA",
            OLD_1,
            json!({"setpoint": 1, "old_max": 9}),
            vec![limit(".old_max", 10)],
        ),
    ]);

    let error = draft
        .plan_rebind(&request(OLD_1, OLD_2), &trees())
        .unwrap_err();

    let texts = texts(error);
    assert_eq!(texts.len(), 2);
    assert!(texts[0].starts_with("AA.old_max"));
    assert!(texts[1].starts_with("ZZ.old_max"));
}

#[test]
fn failing_plan_leaves_the_draft_unchanged() {
    let draft = draft_with(vec![param(
        1,
        "OLD",
        OLD_1,
        json!({"setpoint": 1, "old_max": 9}),
        vec![limit(".old_max", 10)],
    )]);
    let before = draft.clone();

    draft
        .plan_rebind(&request(OLD_1, OLD_2), &trees())
        .unwrap_err();

    assert_eq!(draft.parameters(), before.parameters());
}

#[test]
fn drops_limit_has_its_code_registry_row_and_message() {
    let error = RecipeError::RebindDropsLimit {
        tag: ParameterTag::parse("SPEED").unwrap(),
        leaf: LeafPath::parse(".zones[2].temp_max").unwrap(),
    };

    assert_eq!(error.code(), ErrorCode::RCP_029);
    assert_eq!(error.code().as_str(), "E-RCP-029");
    assert!(crate::error_codes::REGISTRY.contains(&ErrorCode::RCP_029));
    assert_eq!(
        error.to_string(),
        "SPEED.zones[2].temp_max has a limit, but the new revision has no such member or gives the member another type. Delete the limit first. After the rebind, set the limit again."
    );
}
