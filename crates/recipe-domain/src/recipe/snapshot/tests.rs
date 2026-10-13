//! Tests of the snapshot check (design D12, invariants I6 and I19, ADR-0028).

use serde_json::json;
use uuid::Uuid;

use super::fixtures::{
    ARCHIVED_UNIT, INT, UNKNOWN, WSTRING_10, ZONE_2, limited, on_unit, param, parameter_tag, range,
    recipe, trees, trees_on, units,
};
use crate::error_codes::{ErrorCode, REGISTRY};
use crate::plant::{PlantError, PlcPlatform, UnitId};
use crate::recipe::{
    ParameterTag, RecipeError, SnapshotError, SnapshotItem, StoredParameter, VersionSnapshot,
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
