//! Tests of the snapshot check (design D12, invariants I6 and I19, ADR-0028).

use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

use super::fixtures::{
    ARCHIVED_UNIT, DATE, DINT, INT, REAL, UNKNOWN, WSTRING_10, ZONE_2, limited, on_unit, param,
    parameter_tag, range, recipe, trees, trees_on, units,
};
use crate::error_codes::{ErrorCode, REGISTRY};
use crate::plant::fixtures::{TIME_TREES_ROOT, time_trees_on};
use crate::plant::{LeafPath, PlantError, PlcPlatform, Primitive, S7Address, TypeTrees, UnitId};
use crate::recipe::draft::support::stored_draft_with;
use crate::recipe::{
    Draft, DraftError, ParameterTag, RecipeError, SnapshotError, SnapshotItem, StoredLimit,
    StoredParameter, VersionSnapshot, VersionState,
};

fn check(
    parameters: Vec<StoredParameter>,
    is_deleted: bool,
    is_machine_archived: bool,
) -> Result<Vec<ParameterTag>, RecipeError> {
    VersionSnapshot::from(parameters).check(
        &recipe(is_deleted),
        &units(is_machine_archived),
        &trees(),
    )
}

fn items_of(result: Result<Vec<ParameterTag>, RecipeError>) -> Vec<SnapshotItem> {
    match result.expect_err("the check must fail") {
        RecipeError::ValidationFailed { items } => items,
        other => {
            assert_eq!(other.code(), ErrorCode::RCP_020, "{other}");
            Vec::new()
        }
    }
}

fn codes(items: &[SnapshotItem]) -> Vec<ErrorCode> {
    items.iter().map(SnapshotItem::code).collect()
}

#[test]
fn a_retired_version_with_a_parameter_on_an_archived_unit_gives_one_e_plt_009_item() {
    let parameters = vec![on_unit(param(1, "SPEED", INT, json!(5)), ARCHIVED_UNIT)];
    let error = check(parameters, false, false).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_020);
    assert_eq!(
        error.to_string(),
        "1 parameters are not valid. See the list."
    );
    let items = items_of(Err(error));
    assert_eq!(codes(&items), vec![ErrorCode::PLT_009]);
    assert_eq!(
        items.first().unwrap().to_string(),
        "Unit U1.2 is archived. Select a live unit."
    );
    assert!(matches!(
        items.first(),
        Some(SnapshotItem::Parameter {
            error: SnapshotError::Plant(PlantError::UnitArchived { .. }),
            ..
        })
    ));
}

#[test]
fn a_type_that_left_the_platform_gives_an_e_rcp_013_item_with_the_type_label() {
    let parameters = vec![
        param(1, "SPEED", INT, json!(5)),
        param(2, "NAME", WSTRING_10, json!("x")),
    ];
    let result = VersionSnapshot::from(parameters).check(
        &recipe(false),
        &units(false),
        &trees_on(PlcPlatform::S7_300),
    );
    let items = items_of(result);
    assert_eq!(codes(&items), vec![ErrorCode::RCP_013]);
    assert_eq!(
        items.first().unwrap().to_string(),
        "Parameter NAME uses type WSTRING(10). This type does not exist on S7_300."
    );
}

#[test]
fn a_type_that_the_trees_do_not_know_gives_an_e_rcp_013_item() {
    let parameters = vec![param(1, "SPEED", UNKNOWN, json!(5))];
    let items = items_of(check(parameters, false, false));
    assert_eq!(codes(&items), vec![ErrorCode::RCP_013]);
}

#[test]
fn a_stored_value_outside_its_type_gives_an_e_rcp_005_item() {
    let items = items_of(check(
        vec![param(1, "SPEED", INT, json!(40_000))],
        false,
        false,
    ));
    assert_eq!(codes(&items), vec![ErrorCode::RCP_005]);
}

#[test]
fn a_stored_value_outside_its_limit_gives_an_e_rcp_004_item() {
    let parameters = vec![limited(
        param(1, "SPEED", INT, json!(150)),
        vec![range("", 0, 100)],
    )];
    let items = items_of(check(parameters, false, false));
    assert_eq!(codes(&items), vec![ErrorCode::RCP_004]);
    assert_eq!(
        items.first().unwrap().to_string(),
        "Value 150 of SPEED is outside the process limit 0 to 100."
    );
}

#[test]
fn a_stored_limit_that_the_tree_rejects_gives_the_mapped_limit_item() {
    let parameters = vec![limited(
        param(1, "SPEED", INT, json!(5)),
        vec![range(".missing", 0, 1)],
    )];
    let items = items_of(check(parameters, false, false));
    assert_eq!(codes(&items), vec![ErrorCode::RCP_017]);
}

#[test]
fn an_archived_machine_gives_an_e_plt_010_version_item() {
    let items = items_of(check(vec![param(1, "SPEED", INT, json!(5))], false, true));
    assert_eq!(codes(&items), vec![ErrorCode::PLT_010]);
    assert!(matches!(
        items.first(),
        Some(SnapshotItem::Version {
            error: SnapshotError::Plant(PlantError::MachineArchived { .. })
        })
    ));
}

#[test]
fn a_deleted_recipe_gives_an_e_rcp_012_version_item() {
    let items = items_of(check(vec![param(1, "SPEED", INT, json!(5))], true, false));
    assert_eq!(codes(&items), vec![ErrorCode::RCP_012]);
    assert_eq!(
        items.first().unwrap().to_string(),
        "Recipe R-1 is deleted. An administrator can restore it."
    );
}

#[test]
fn version_items_follow_the_parameter_items() {
    let parameters = vec![param(1, "SPEED", INT, json!(40_000))];
    let items = items_of(check(parameters, true, true));
    assert_eq!(
        codes(&items),
        vec![ErrorCode::RCP_005, ErrorCode::PLT_010, ErrorCode::RCP_012]
    );
}

#[test]
fn two_problems_give_two_items_in_tag_order() {
    let parameters = vec![
        on_unit(param(1, "B_SPEED", INT, json!(5)), ARCHIVED_UNIT),
        param(2, "A_SPEED", INT, json!(40_000)),
    ];
    let error = check(parameters, false, false).unwrap_err();
    assert_eq!(
        error.to_string(),
        "2 parameters are not valid. See the list."
    );
    let items = items_of(Err(error));
    assert_eq!(codes(&items), vec![ErrorCode::RCP_005, ErrorCode::PLT_009]);
}

#[test]
fn one_parameter_can_give_an_item_for_its_unit_and_one_for_its_value() {
    let parameters = vec![on_unit(
        param(1, "SPEED", INT, json!(40_000)),
        ARCHIVED_UNIT,
    )];
    let items = items_of(check(parameters, false, false));
    assert_eq!(codes(&items), vec![ErrorCode::PLT_009, ErrorCode::RCP_005]);
}

#[test]
fn an_unknown_unit_gives_an_item_with_the_code_e_plt_020() {
    let parameters = vec![on_unit(param(1, "SPEED", INT, json!(5)), 999)];
    let items = items_of(check(parameters, false, false));
    assert_eq!(codes(&items), vec![ErrorCode::PLT_020]);
    let unit = UnitId::from(Uuid::from_u128(999));
    assert_eq!(
        items.first().unwrap().to_string(),
        format!("Parameter SPEED uses the unit {unit}, which does not exist on this machine.")
    );
    assert!(matches!(
        items.first(),
        Some(SnapshotItem::UnknownUnit { unit: found, .. }) if *found == unit
    ));
}

#[test]
fn a_valid_snapshot_lists_only_the_outdated_parameters() {
    let parameters = vec![
        param(1, "SPEED", INT, json!(5)),
        param(2, "ZONE", ZONE_2, json!({"temp": 1})),
    ];
    let outdated = check(parameters, false, false).unwrap();
    assert_eq!(outdated, vec![parameter_tag("ZONE")]);
}

#[test]
fn a_valid_snapshot_without_parameters_lists_nothing() {
    assert!(check(Vec::new(), false, false).unwrap().is_empty());
}

#[test]
fn a_snapshot_keeps_the_outdated_tags_in_tag_order() {
    let parameters = vec![
        param(1, "Z_ZONE", ZONE_2, json!({"temp": 1})),
        param(2, "A_ZONE", ZONE_2, json!({"temp": 2})),
    ];
    let outdated = check(parameters, false, false).unwrap();
    assert_eq!(
        outdated,
        vec![parameter_tag("A_ZONE"), parameter_tag("Z_ZONE")]
    );
}

#[test]
fn the_new_variants_have_their_codes_registry_rows_and_messages() {
    let error = RecipeError::TypeNotOnPlatform {
        tag: parameter_tag("SPEED"),
        type_name: "WSTRING(10)".to_owned(),
        platform: PlcPlatform::S7_300,
    };
    assert_eq!(error.code(), ErrorCode::RCP_013);
    assert_eq!(
        error.to_string(),
        "Parameter SPEED uses type WSTRING(10). This type does not exist on S7_300."
    );
    let list = RecipeError::ValidationFailed { items: Vec::new() };
    assert_eq!(list.code(), ErrorCode::RCP_020);
    assert_eq!(
        list.to_string(),
        "0 parameters are not valid. See the list."
    );
    assert!(REGISTRY.contains(&ErrorCode::RCP_013));
    assert!(REGISTRY.contains(&ErrorCode::RCP_020));
}

#[test]
fn trees_of_another_machine_give_one_e_plt_013_item() {
    use super::fixtures::trees_of;
    let wrong = trees_of(
        crate::plant::fixtures::other_machine(),
        PlcPlatform::TwinCat3,
    );
    let result = VersionSnapshot::from(vec![param(1, "SPEED", INT, json!(5))]).check(
        &recipe(false),
        &units(false),
        &wrong,
    );
    assert_eq!(codes(&items_of(result)), vec![ErrorCode::PLT_013]);
}

#[test]
fn units_of_another_machine_give_e_plt_020_items() {
    let result = VersionSnapshot::from(vec![param(1, "SPEED", INT, json!(5))]).check(
        &recipe(false),
        &super::fixtures::foreign_units(),
        &trees(),
    );
    assert_eq!(codes(&items_of(result)), vec![ErrorCode::PLT_020]);
}

#[test]
fn the_count_of_e_rcp_020_counts_parameters_and_not_items() {
    let parameters = vec![limited(
        on_unit(param(1, "SPEED", INT, json!(40_000)), ARCHIVED_UNIT),
        vec![range("", 0, 99_999)],
    )];
    let error = check(parameters, false, false).unwrap_err();
    assert_eq!(
        error.to_string(),
        "1 parameters are not valid. See the list."
    );
    assert_eq!(items_of(Err(error)).len(), 3);
}

#[test]
fn the_count_of_e_rcp_020_skips_version_items_and_counts_an_unknown_unit() {
    let only_version = check(Vec::new(), true, false).unwrap_err();
    assert_eq!(
        only_version.to_string(),
        "0 parameters are not valid. See the list."
    );
    let parameters = vec![
        on_unit(param(1, "SPEED", INT, json!(5)), 999),
        on_unit(param(2, "TEMP", INT, json!(5)), ARCHIVED_UNIT),
    ];
    let two = check(parameters, true, false).unwrap_err();
    assert_eq!(two.to_string(), "2 parameters are not valid. See the list.");
}

/// The `REAL` limit `max` of ADR-0117: 8 significant digits.
fn real_max_limit() -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some(Decimal::ZERO),
        max: Some("100.00001".parse().unwrap()),
        max_length: None,
    }
}

/// The base version of ADR-0120: each part is valid only under the earlier rule set.
fn earlier_rules_base(date: i64) -> Vec<StoredParameter> {
    let mut addressed = param(4, "ADDRESSED", DINT, json!(0));
    addressed.s7_address = Some(S7Address::parse("DB1.DBD65534").unwrap());
    vec![
        limited(
            param(1, "DUE", DATE, json!(date)),
            vec![range("", 7_305, 72_700)],
        ),
        param(2, "PRECISE", REAL, json!(16_777_217)),
        limited(param(3, "LIMITED", REAL, json!(50)), vec![real_max_limit()]),
        addressed,
    ]
}

fn snapshot_check_on_s7_1500(
    parameters: Vec<StoredParameter>,
) -> Result<Vec<ParameterTag>, RecipeError> {
    snapshot_check_with_trees(parameters, &trees_on(PlcPlatform::S7_1500))
}

fn snapshot_check_with_trees(
    parameters: Vec<StoredParameter>,
    trees: &TypeTrees,
) -> Result<Vec<ParameterTag>, RecipeError> {
    VersionSnapshot::from(parameters).check(&recipe(false), &units(false), trees)
}

#[test]
fn adr_0120_the_snapshot_check_passes_a_base_that_is_valid_under_the_earlier_rules() {
    let result = snapshot_check_on_s7_1500(earlier_rules_base(72_700));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn adr_0120_the_snapshot_check_refuses_a_date_above_the_earlier_range() {
    let items = items_of(snapshot_check_on_s7_1500(earlier_rules_base(72_900)));
    assert_eq!(codes(&items), vec![ErrorCode::RCP_005]);
}

/// A numeric limit row with a `min` and no `max`.
fn min_only_limit(min: i64) -> StoredLimit {
    StoredLimit {
        max: None,
        ..range("", min, 0)
    }
}

/// A `DATE` parameter with the count 72700 and a limit row that has a `min` and no `max`.
fn date_above_current_end_with_min_only_limit() -> Vec<StoredParameter> {
    vec![limited(
        param(1, "DUE", DATE, json!(72_700)),
        vec![min_only_limit(70_000)],
    )]
}

#[test]
fn adr_0120_the_snapshot_check_bounds_a_limit_without_max_by_the_earlier_range() {
    let result = snapshot_check_on_s7_1500(date_above_current_end_with_min_only_limit());
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn adr_0120_the_approval_still_refuses_the_date_72700_when_the_limit_has_no_max() {
    let items = approval_items_on_s7_1500(date_above_current_end_with_min_only_limit());
    assert_eq!(codes(&items), vec![ErrorCode::RCP_005]);
}

fn approval_items_on_s7_1500(parameters: Vec<StoredParameter>) -> Vec<SnapshotItem> {
    approval_items_with_trees(parameters, &trees_on(PlcPlatform::S7_1500))
}

fn approval_items_with_trees(
    parameters: Vec<StoredParameter>,
    trees: &TypeTrees,
) -> Vec<SnapshotItem> {
    let draft = Draft::load(stored_draft_with(
        VersionState::LiveDraft,
        PlcPlatform::S7_1500,
        parameters,
    ))
    .unwrap();
    let error = draft
        .check_for_approval(&units(false), trees, false)
        .expect_err("the approval must fail");
    match error {
        DraftError::Recipe(RecipeError::ValidationFailed { items }) => items,
        other => {
            assert_eq!(other.code(), ErrorCode::RCP_020, "{other}");
            Vec::new()
        }
    }
}

#[test]
fn adr_0120_the_approval_gives_e_rcp_005_for_the_date_value_72700() {
    let items = approval_items_on_s7_1500(vec![limited(
        param(1, "DUE", DATE, json!(72_700)),
        vec![range("", 7_305, 72_683)],
    )]);
    assert_eq!(codes(&items), vec![ErrorCode::RCP_005]);
}

#[test]
fn adr_0120_the_approval_gives_e_rcp_006_for_the_date_limit_max_72700() {
    let items = approval_items_on_s7_1500(vec![limited(
        param(1, "DUE", DATE, json!(8_000)),
        vec![range("", 7_305, 72_700)],
    )]);
    assert_eq!(codes(&items), vec![ErrorCode::RCP_006]);
}

/// The type identifier of the `S5TIME` row of the tests below.
const S5TIME: u128 = TIME_TREES_ROOT;

/// The trees of one `S5TIME` type on `S7_1500`.
fn s5time_trees() -> TypeTrees {
    time_trees_on(PlcPlatform::S7_1500, Primitive::S5Time)
}

/// An `S5TIME` parameter `DELAY` with the value 0 and the limit `max` 15 ms. 15 is no S5TIME step.
fn s5time_delay_with_max_15() -> Vec<StoredParameter> {
    let max_15 = StoredLimit {
        max: Some(Decimal::from(15)),
        ..min_only_limit(0)
    };
    vec![limited(param(1, "DELAY", S5TIME, json!(0)), vec![max_15])]
}

/// Assert that the only item of the list belongs to the parameter `DELAY`.
fn assert_is_item_of_delay(items: &[SnapshotItem]) {
    assert!(
        matches!(items, [SnapshotItem::Parameter { tag, .. }] if *tag == parameter_tag("DELAY")),
        "{items:?}"
    );
}

#[test]
fn a_stored_s5time_limit_max_of_15_ms_gives_an_e_rcp_006_item_in_the_snapshot_check() {
    let result = snapshot_check_with_trees(s5time_delay_with_max_15(), &s5time_trees());
    let items = items_of(result);
    assert_eq!(codes(&items), vec![ErrorCode::RCP_006]);
    assert_is_item_of_delay(&items);
}

#[test]
fn a_stored_s5time_limit_max_of_15_ms_gives_an_e_rcp_006_item_in_the_approval() {
    let items = approval_items_with_trees(s5time_delay_with_max_15(), &s5time_trees());
    assert_eq!(codes(&items), vec![ErrorCode::RCP_006]);
    assert_is_item_of_delay(&items);
}
