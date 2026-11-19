use serde_json::json;

use super::super::map::MapFault;
use super::super::test_support::{
    INT, MODE_1, MODE_2, OLD_1, OLD_2, OLDS_1, RAMP_1, RAMP_2, ZONE_2, ZONE_3, ZONES_2, draft_with,
    key, limit, number, param, plan_of, request, request_with, trees,
};
use super::{LimitMapping, map_limits};
use crate::error_codes::ErrorCode;
use crate::plant::LeafPath;
use crate::plant::fixtures::id;
use crate::recipe::draft::support::assert_stored;
use crate::recipe::{DraftError, ParameterTag, RecipeError};

/// The entries of a rejected plan.
fn entries(error: DraftError) -> Vec<RecipeError> {
    match error {
        DraftError::List(list) => Some(list.into_vec()),
        DraftError::Recipe(_) | DraftError::Plant(_) | DraftError::Stored(_) => None,
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

// `Draft::load` trusts stored rows, so a stored row with a minimum above its maximum reaches the
// rebind. The old tree rejects it first. A kept member has the same shape in both trees, so the
// plan cannot reach the check against the planned tree. The later checks reject this row too, so
// the test does not isolate the old-tree check. The next test calls `map_limits` directly.
#[test]
fn stored_limit_that_the_old_tree_rejects_gives_e_rcp_018() {
    let draft = draft_with(vec![param(
        1,
        "SPEED",
        ZONE_2,
        json!({"setpoint": 5}),
        vec![limit(".setpoint", -5)],
    )]);
    let request = request_with(ZONE_2, ZONE_3, &[(1, ".temp_max", number(70))]);

    let error = draft.plan_rebind(&request, &trees()).unwrap_err();

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_018);
}

// A stored defect stops the plan before the entries of the dropped limits (ADR-0195 (n)).
#[test]
fn stored_defect_of_the_old_tree_comes_before_the_entry_of_a_dropped_limit() {
    let draft = draft_with(vec![param(
        1,
        "OLD",
        OLD_1,
        json!({"setpoint": 1, "old_max": 9}),
        vec![limit(".old_max", 10), limit(".setpoint", -5)],
    )]);

    let error = draft
        .plan_rebind(&request(OLD_1, OLD_2), &trees())
        .unwrap_err();

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_018);
}

// The bad row is on a member that the rebind removes. Only the old-tree check of `map_limits`
// gives the stored defect. Without it the row becomes the entry `E-RCP-029` (ADR-0195 (n)).
#[test]
fn stored_limit_with_bad_bounds_on_a_removed_member_gives_a_stored_defect() {
    let draft = draft_with(vec![param(
        1,
        "OLD",
        OLD_1,
        json!({"setpoint": 1, "old_max": 9}),
        vec![limit(".old_max", -5)],
    )]);

    let error = draft
        .plan_rebind(&request(OLD_1, OLD_2), &trees())
        .unwrap_err();

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_018);
}

// The old tree accepts the row. The planned tree is a leaf that has no member `setpoint`, so
// every row stays and only the check against the planned tree rejects it.
#[test]
fn kept_limit_that_only_the_planned_tree_rejects_gives_e_rcp_017() {
    let trees = trees();
    let tag = ParameterTag::parse("SPEED").unwrap();
    let rows = [limit(".setpoint", 100)];
    let accepted = LimitMapping {
        tag: &tag,
        old_tree: trees.tree(id(ZONE_2)).unwrap(),
        new_tree: trees.tree(id(ZONE_2)).unwrap(),
        rows: &rows,
    };
    assert!(matches!(map_limits(&accepted), Ok(kept) if kept == rows));
    let rejected = LimitMapping {
        new_tree: trees.tree(id(INT)).unwrap(),
        ..accepted
    };

    assert!(matches!(
        map_limits(&rejected),
        Err(MapFault::Defect(error)) if error.code() == ErrorCode::RCP_017
    ));
}

#[test]
fn stored_limit_on_a_path_that_the_old_struct_lacks_gives_a_stored_defect() {
    let draft = draft_with(vec![param(
        1,
        "SPEED",
        ZONE_2,
        json!({"setpoint": 5}),
        vec![limit(".ghost", 10)],
    )]);
    let request = request_with(ZONE_2, ZONE_3, &[(1, ".temp_max", number(70))]);

    let error = draft.plan_rebind(&request, &trees()).unwrap_err();

    assert!(matches!(error, DraftError::Stored(_)), "{error:?}");
    assert_eq!(
        match &error {
            DraftError::Stored(defect) => defect.key().copied(),
            _ => None,
        },
        Some(key(1))
    );
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
