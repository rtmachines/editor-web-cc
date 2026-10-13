//! Tests of the first part of `Draft::add_parameter` (design D6, step 1).

use super::support::{
    BOOL, INT, NOT_ASKED, UNRESOLVED, assert_rejected, draft_with, draft_with_speed, foreign_unit,
    new_parameter, new_parameter_at, trees_of, trees_on,
};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{id, machine, other_machine};
use crate::plant::{DerivedTypeReason, PlantError, PlcPlatform};
use crate::recipe::{DraftError, NewParameter, RecipeError};

#[test]
fn a_tag_that_the_draft_uses_gives_e_rcp_014_with_the_version_number() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let error = assert_rejected(
        &mut draft,
        &trees,
        new_parameter(2, "SPEED", INT, "0"),
        ErrorCode::RCP_014,
    );
    assert_eq!(
        error.to_string(),
        "Parameter tag SPEED is already used in version 3."
    );
}

#[test]
fn a_key_that_the_draft_uses_gives_e_rcp_014_with_the_new_tag() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let error = assert_rejected(
        &mut draft,
        &trees,
        new_parameter(1, "TEMP", INT, "0"),
        ErrorCode::RCP_014,
    );
    assert!(matches!(
        error,
        DraftError::Recipe(RecipeError::DuplicateTag { tag, number: 3 }) if tag.as_str() == "TEMP"
    ));
}

#[test]
fn a_unit_of_another_machine_gives_e_plt_020_with_the_path_of_the_unit() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let mut new = new_parameter(2, "TEMP", INT, "0");
    new.unit = foreign_unit();
    let error = assert_rejected(&mut draft, &trees, new, ErrorCode::PLT_020);
    assert!(matches!(
        error,
        DraftError::Plant(PlantError::UnitNotFound { path }) if path.to_string() == "U1"
    ));
}

#[test]
fn trees_of_another_machine_give_e_plt_013_not_on_platform() {
    let mut draft = draft_with_speed();
    let trees = trees_of(other_machine(), PlcPlatform::TwinCat3);
    let error = assert_rejected(
        &mut draft,
        &trees,
        new_parameter(2, "TEMP", INT, "0"),
        ErrorCode::PLT_013,
    );
    assert!(matches!(
        error,
        DraftError::Plant(PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::NotOnPlatform,
            ..
        })
    ));
}

#[test]
fn trees_of_another_platform_give_e_plt_013() {
    let mut draft = draft_with_speed();
    let trees = trees_of(machine(), PlcPlatform::S7_1500);
    assert_rejected(
        &mut draft,
        &trees,
        new_parameter(2, "TEMP", INT, "0"),
        ErrorCode::PLT_013,
    );
}

#[test]
fn a_type_that_the_trees_never_resolved_gives_e_plt_013_with_the_type_label() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let error = assert_rejected(
        &mut draft,
        &trees,
        new_parameter(2, "TEMP", NOT_ASKED, "0"),
        ErrorCode::PLT_013,
    );
    let expected = trees.label(id(NOT_ASKED));
    assert!(matches!(
        error,
        DraftError::Plant(PlantError::DerivedTypeInvalid {
            type_expr,
            reason: DerivedTypeReason::NotOnPlatform,
        }) if type_expr == expected
    ));
}

#[test]
fn a_type_whose_resolution_failed_gives_the_plant_error_of_the_resolution() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let expected = trees
        .get(id(UNRESOLVED))
        .and_then(|result| result.as_ref().err())
        .map(|error| (error.code(), error.to_string()))
        .expect("the type does not resolve");
    let error = assert_rejected(
        &mut draft,
        &trees,
        new_parameter(2, "TEMP", UNRESOLVED, "0"),
        expected.0,
    );
    assert_eq!(error.to_string(), expected.1);
}

#[test]
fn an_int_at_a_double_word_address_on_s7_300_gives_e_rcp_054() {
    let mut draft = draft_with(PlcPlatform::S7_300, Vec::new());
    let trees = trees_on(PlcPlatform::S7_300);
    let error = assert_rejected(
        &mut draft,
        &trees,
        new_parameter_at(2, "SPEED", INT, "DB1.DBD0"),
        ErrorCode::RCP_054,
    );
    assert_eq!(
        error.to_string(),
        "Address DB1.DBD0 of SPEED does not match the type of SPEED. Use the width W."
    );
}

#[test]
fn a_parameter_without_an_address_on_s7_300_gives_e_rcp_003() {
    let mut draft = draft_with(PlcPlatform::S7_300, Vec::new());
    let trees = trees_on(PlcPlatform::S7_300);
    assert_rejected(
        &mut draft,
        &trees,
        new_parameter(2, "SPEED", INT, "0"),
        ErrorCode::RCP_003,
    );
}

#[test]
fn an_address_on_twincat3_gives_e_rcp_053() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    assert_rejected(
        &mut draft,
        &trees,
        new_parameter_at(2, "TEMP", INT, "DB1.DBW0"),
        ErrorCode::RCP_053,
    );
}

#[test]
fn an_int_at_an_odd_offset_gives_e_rcp_023() {
    let mut draft = draft_with(PlcPlatform::S7_300, Vec::new());
    let trees = trees_on(PlcPlatform::S7_300);
    assert_rejected(
        &mut draft,
        &trees,
        new_parameter_at(2, "SPEED", INT, "DB1.DBW3"),
        ErrorCode::RCP_023,
    );
}

#[test]
fn the_tag_check_runs_before_the_unit_check() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let mut new = new_parameter(2, "SPEED", INT, "0");
    new.unit = foreign_unit();
    assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_014);
}

#[test]
fn the_unit_check_runs_before_the_trees_check() {
    let mut draft = draft_with_speed();
    let trees = trees_of(other_machine(), PlcPlatform::TwinCat3);
    let mut new = new_parameter(2, "TEMP", INT, "0");
    new.unit = foreign_unit();
    assert_rejected(&mut draft, &trees, new, ErrorCode::PLT_020);
}

#[test]
fn the_trees_check_runs_before_the_s7_rules() {
    let mut draft = draft_with(PlcPlatform::S7_300, Vec::new());
    let trees = trees_of(other_machine(), PlcPlatform::S7_300);
    assert_rejected(
        &mut draft,
        &trees,
        new_parameter_at(2, "SPEED", INT, "DB1.DBD0"),
        ErrorCode::PLT_013,
    );
}

#[test]
fn a_valid_definition_passes_step_one() {
    let mut draft = draft_with(PlcPlatform::S7_300, Vec::new());
    let trees = trees_on(PlcPlatform::S7_300);
    let new = NewParameter {
        value_json: "true".to_owned(),
        ..new_parameter_at(2, "RUN", BOOL, "DB1.DBX0.0")
    };
    assert!(super::support::try_add(&mut draft, &trees, new).is_ok());
}

#[test]
fn trees_that_do_not_hold_a_type_of_the_draft_give_e_plt_013() {
    let mut draft = draft_with_speed();
    let trees = super::support::trees_asking(&[crate::recipe::draft::add::support::REAL]);
    let new = new_parameter(2, "TEMP", crate::recipe::draft::add::support::REAL, "0");
    assert_rejected(&mut draft, &trees, new, ErrorCode::PLT_013);
}

#[test]
fn an_identifier_that_the_draft_uses_gives_e_rcp_014_with_the_new_tag() {
    let mut draft = draft_with_speed();
    let trees = trees_on(PlcPlatform::TwinCat3);
    let mut new = new_parameter(2, "TEMP", INT, "0");
    new.id = crate::recipe::ParameterId::from(uuid::Uuid::from_u128(1));
    let error = assert_rejected(&mut draft, &trees, new, ErrorCode::RCP_014);
    assert!(matches!(
        error,
        DraftError::Recipe(RecipeError::DuplicateTag { tag, number: 3 }) if tag.as_str() == "TEMP"
    ));
}

#[test]
fn two_parameters_with_one_sort_order_on_one_unit_are_allowed() {
    let mut speed = crate::recipe::draft::support::stored_parameter(1, "SPEED", id(INT), None);
    speed.ordinal = crate::recipe::Ordinal::parse(5).unwrap();
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![speed]);
    let trees = trees_on(PlcPlatform::TwinCat3);
    let temp = new_parameter(5, "TEMP", INT, "0");
    assert_eq!(temp.ordinal.value(), 5);
    super::support::try_add(&mut draft, &trees, temp).unwrap();
    let ordinals: Vec<u32> = draft
        .parameters()
        .iter()
        .map(|p| p.ordinal().value())
        .collect();
    assert_eq!(ordinals, vec![5, 5]);
}
