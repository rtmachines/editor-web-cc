use std::collections::BTreeMap;

use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{
    array_row, id, machine, other_machine, primitive_row, struct_row, struct_row_revision,
    types_for,
};
use crate::plant::{LeafPath, PlcPlatform, Primitive, TypeTrees};
use crate::recipe::draft::support::{stored_draft_with, stored_parameter};
use crate::recipe::{
    Draft, DraftError, ParameterKey, RebindRequest, RebindTargetReason, RecipeError, VersionState,
};

const INT: u128 = 1;
const BOOL: u128 = 3;
const ZONE_2: u128 = 20;
const ZONE_3: u128 = 21;
const PUMP_4: u128 = 22;
const ARRAY_ZONE: u128 = 30;
const ARRAY_INT: u128 = 31;
const LINE: u128 = 40;
const BIG_1: u128 = 50;
const BIG_2: u128 = 51;
const BIG_ARRAY: u128 = 52;

const ALL_TYPES: [u128; 12] = [
    INT, BOOL, ZONE_2, ZONE_3, PUMP_4, ARRAY_ZONE, ARRAY_INT, LINE, BIG_1, BIG_2, BIG_ARRAY, 99,
];

fn trees_for(owner: crate::plant::MachineId) -> TypeTrees {
    let rows = vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(BOOL, Primitive::Bool),
        struct_row_revision(ZONE_2, "ST_Zone", 2, &[("temp", INT)]),
        struct_row_revision(ZONE_3, "ST_Zone", 3, &[("temp", INT), ("active", BOOL)]),
        struct_row_revision(PUMP_4, "ST_Pump", 4, &[("rpm", INT)]),
        array_row(ARRAY_ZONE, ZONE_2, &[(1, 4)]),
        array_row(ARRAY_INT, INT, &[(1, 4)]),
        struct_row(LINE, "ST_Line", &[("zone", ZONE_2)]),
        struct_row_revision(BIG_1, "ST_Big", 1, &[("a", INT)]),
        struct_row_revision(BIG_2, "ST_Big", 2, &[("a", INT), ("b", INT)]),
        array_row(BIG_ARRAY, BIG_1, &[(1, 16_384)]),
    ];
    TypeTrees::resolve(
        ALL_TYPES.map(id),
        &types_for(owner, PlcPlatform::TwinCat3, rows),
    )
}

fn trees() -> TypeTrees {
    trees_for(machine())
}

fn draft_of(parameters: &[(u128, &str, u128)]) -> Draft {
    let stored = parameters
        .iter()
        .map(|(n, tag, type_n)| stored_parameter(*n, tag, id(*type_n), None))
        .collect();
    Draft::load(stored_draft_with(
        VersionState::LiveDraft,
        PlcPlatform::TwinCat3,
        stored,
    ))
    .unwrap()
}

fn mixed_draft() -> Draft {
    let zone = |n: u128, tag: &str, type_n: u128, value: serde_json::Value| {
        let mut stored = stored_parameter(n, tag, id(type_n), None);
        stored.value = value.into();
        stored
    };
    let zones = serde_json::json!([{"temp": 1}, {"temp": 2}, {"temp": 3}, {"temp": 4}]);
    Draft::load(stored_draft_with(
        VersionState::LiveDraft,
        PlcPlatform::TwinCat3,
        vec![
            zone(1, "ZONE", ZONE_2, serde_json::json!({"temp": 1})),
            zone(2, "ZONES", ARRAY_ZONE, zones),
            zone(3, "LINE", LINE, serde_json::json!({"zone": {"temp": 1}})),
            zone(4, "COUNT", INT, serde_json::json!(0)),
            zone(5, "COUNTS", ARRAY_INT, serde_json::json!([0, 0, 0, 0])),
        ],
    ))
    .unwrap()
}

/// The request for the mixed draft: `active` is the only new member.
fn valued_request() -> RebindRequest {
    let mut built = request(ZONE_2, ZONE_3);
    let no = || crate::recipe::MemberValue::Json(serde_json::json!(false));
    built
        .new_member_values
        .insert((key(1), LeafPath::parse(".active").unwrap()), no());
    for index in 1..=4 {
        let path = LeafPath::parse(&format!("[{index}].active")).unwrap();
        built.new_member_values.insert((key(2), path), no());
    }
    built
}

fn request(from: u128, to: u128) -> RebindRequest {
    RebindRequest {
        from: id(from),
        to: id(to),
        new_member_values: BTreeMap::new(),
    }
}

fn key(n: u128) -> ParameterKey {
    ParameterKey::from(uuid::Uuid::from_u128(n))
}

/// Return the type name and the reason of a rejected target.
fn rejected(draft: &Draft, from: u128, to: u128) -> (String, RebindTargetReason) {
    let error = draft.plan_rebind(&request(from, to), &trees()).unwrap_err();
    match error {
        DraftError::Recipe(RecipeError::RebindTargetInvalid { type_name, reason }) => {
            Some((type_name, reason))
        }
        _ => None,
    }
    .unwrap()
}

#[test]
fn valid_rebind_lists_the_struct_and_the_array_parameter_only() {
    let draft = mixed_draft();

    let plan = draft.plan_rebind(&valued_request(), &trees()).unwrap();

    let keys: Vec<ParameterKey> = plan.parameters().keys().copied().collect();
    assert_eq!(keys, vec![key(1), key(2)]);
    assert_eq!(plan.from(), id(ZONE_2));
    assert_eq!(plan.to(), id(ZONE_3));
}

#[test]
fn planned_trees_come_from_type_tree_rebind() {
    let draft = mixed_draft();
    let all = trees();

    let plan = draft.plan_rebind(&valued_request(), &all).unwrap();

    let target = all.tree(id(ZONE_3)).unwrap();
    let for_struct = plan.parameters().get(&key(1)).unwrap();
    assert_eq!(for_struct, target);
    let source = all.tree(id(ARRAY_ZONE)).unwrap();
    let expected = source.rebind(id(ZONE_2), target).unwrap().unwrap();
    assert_eq!(plan.parameters().get(&key(2)), Some(&expected));
}

#[test]
fn plan_leaves_the_draft_unchanged() {
    let draft = mixed_draft();
    let before = draft.clone();

    draft.plan_rebind(&valued_request(), &trees()).unwrap();

    assert_eq!(draft.parameters(), before.parameters());
    assert_eq!(draft.row_version(), before.row_version());
}

#[test]
fn rebind_to_an_older_revision_is_rejected_as_not_newer() {
    let draft = draft_of(&[(1, "ZONE", ZONE_3)]);

    let (type_name, reason) = rejected(&draft, ZONE_3, ZONE_2);

    assert_eq!(type_name, "ST_Zone@3");
    assert_eq!(reason, RebindTargetReason::NotNewer);
}

#[test]
fn rebind_to_the_same_revision_is_rejected_as_not_newer() {
    let draft = draft_of(&[(1, "ZONE", ZONE_2)]);

    let (_, reason) = rejected(&draft, ZONE_2, ZONE_2);

    assert_eq!(reason, RebindTargetReason::NotNewer);
}

#[test]
fn rebind_to_another_struct_name_is_rejected() {
    let draft = mixed_draft();

    let (type_name, reason) = rejected(&draft, ZONE_2, PUMP_4);

    assert_eq!(type_name, "ST_Zone@2");
    assert_eq!(reason, RebindTargetReason::OtherStruct);
}

#[test]
fn primitive_source_is_rejected() {
    let draft = mixed_draft();

    let (type_name, reason) = rejected(&draft, INT, ZONE_3);

    assert_eq!(type_name, "INT");
    assert_eq!(reason, RebindTargetReason::SourceNotStruct);
}

#[test]
fn array_source_is_rejected() {
    let draft = mixed_draft();

    let (_, reason) = rejected(&draft, ARRAY_ZONE, ZONE_3);

    assert_eq!(reason, RebindTargetReason::SourceNotStruct);
}

#[test]
fn unresolved_source_is_rejected() {
    let draft = mixed_draft();

    let (_, reason) = rejected(&draft, 99, ZONE_3);

    assert_eq!(reason, RebindTargetReason::SourceNotStruct);
}

#[test]
fn primitive_target_is_rejected() {
    let draft = mixed_draft();

    let (_, reason) = rejected(&draft, ZONE_2, INT);

    assert_eq!(reason, RebindTargetReason::TargetNotStruct);
}

#[test]
fn source_that_only_a_struct_member_uses_is_not_used() {
    let draft = draft_of(&[(3, "LINE", LINE), (4, "COUNT", INT)]);

    let (type_name, reason) = rejected(&draft, ZONE_2, ZONE_3);

    assert_eq!(type_name, "ST_Zone@2");
    assert_eq!(reason, RebindTargetReason::NotUsed);
}

#[test]
fn trees_of_another_machine_give_e_plt_013() {
    let draft = mixed_draft();
    let foreign = trees_for(other_machine());

    let error = draft
        .plan_rebind(&request(ZONE_2, ZONE_3), &foreign)
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(matches!(error, DraftError::Plant(_)));
}

#[test]
fn target_above_the_leaf_limit_gives_e_plt_018() {
    let draft = draft_of(&[(1, "BIGS", BIG_ARRAY)]);

    let error = draft
        .plan_rebind(&request(BIG_1, BIG_2), &trees())
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_018);
    assert!(matches!(error, DraftError::Plant(_)));
}

#[test]
fn every_reason_has_its_phrase() {
    let cases = [
        (
            RebindTargetReason::SourceNotStruct,
            "the source type is not a struct",
        ),
        (
            RebindTargetReason::TargetNotStruct,
            "the target type is not a struct",
        ),
        (
            RebindTargetReason::OtherStruct,
            "the target is a revision of another struct",
        ),
        (
            RebindTargetReason::NotNewer,
            "the target revision is not newer than the source revision",
        ),
        (
            RebindTargetReason::NotUsed,
            "no parameter of the draft uses the source revision",
        ),
    ];
    for (reason, phrase) in cases {
        assert_eq!(reason.to_string(), phrase);
    }
}

#[test]
fn error_message_names_the_source_and_the_reason() {
    let error = RecipeError::RebindTargetInvalid {
        type_name: "ST_Zone@2".to_owned(),
        reason: RebindTargetReason::NotNewer,
    };

    assert_eq!(
        error.to_string(),
        "The rebind of ST_Zone@2 is not possible: the target revision is not newer than the source revision."
    );
    assert_eq!(error.code(), ErrorCode::RCP_052);
    assert_eq!(error.code().as_str(), "E-RCP-052");
    assert!(crate::error_codes::REGISTRY.contains(&ErrorCode::RCP_052));
}

#[test]
fn target_that_the_trees_do_not_hold_is_not_found() {
    let draft = mixed_draft();

    let (_, reason) = rejected(&draft, ZONE_2, 77);

    assert_eq!(reason, RebindTargetReason::TargetNotFound);
}

#[test]
fn target_not_found_has_its_phrase() {
    assert_eq!(
        RebindTargetReason::TargetNotFound.to_string(),
        "the target revision does not exist on the machine of the draft"
    );
}
