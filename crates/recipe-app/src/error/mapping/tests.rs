use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{MachineCode, PlantError, UnitPath};
use recipe_domain::recipe::{Optimistic, RecipeError, ReferenceNumber};

use super::*;

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-100").unwrap()
}

fn duplicate(name: &str) -> StorageError {
    StorageError::Duplicate {
        constraint: name.to_owned(),
    }
}

fn missing_reference(name: &str) -> StorageError {
    StorageError::MissingReference {
        constraint: name.to_owned(),
    }
}

/// The closure of a `CreateRecipe` or `AddUnit` command.
fn known(name: &str) -> Option<AppError> {
    match name {
        RECIPE_REFERENCE_UNIQUE => Some(
            RecipeError::DuplicateReference {
                reference: reference(),
            }
            .into(),
        ),
        MACHINE_CODE_LIVE_UNIQUE => Some(
            PlantError::DuplicateMachineCode {
                code: MachineCode::parse("M12").unwrap(),
            }
            .into(),
        ),
        UNIT_PARENT_EXISTS => Some(
            PlantError::ParentNotFound {
                path: UnitPath::parse("U1.2").unwrap(),
            }
            .into(),
        ),
        _ => None,
    }
}

#[test]
fn applied_outcome_gives_the_value() {
    let result = conflict(Optimistic::Applied(7_u8), reference(), 3);
    assert_eq!(result.unwrap(), 7);
}

#[test]
fn conflict_outcome_gives_concurrent_edit_with_both_row_versions() {
    let error = conflict(
        Optimistic::<u8>::Conflict {
            expected: 3,
            actual: 4,
        },
        reference(),
        2,
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    assert!(matches!(
        error,
        AppError::ConcurrentEdit {
            number: 2,
            expected: 3,
            actual: 4,
            ..
        }
    ));
}

#[test]
fn known_duplicate_reference_gives_rcp_001() {
    let error = map_storage(duplicate("recipe_reference_unique"), known);
    assert_eq!(error.code(), ErrorCode::RCP_001);
}

#[test]
fn known_duplicate_machine_code_gives_plt_001() {
    let error = map_storage(duplicate("machine_code_live_unique"), known);
    assert_eq!(error.code(), ErrorCode::PLT_001);
}

#[test]
fn known_missing_parent_gives_plt_003() {
    let error = map_storage(missing_reference("unit_parent_exists"), known);
    assert_eq!(error.code(), ErrorCode::PLT_003);
}

#[test]
fn unknown_duplicate_name_gives_db_006() {
    let error = map_storage(duplicate("some_other_unique"), known);
    assert_eq!(error.code(), ErrorCode::DB_006);
    assert!(matches!(
        error,
        AppError::Storage(StorageError::Duplicate { .. })
    ));
}

#[test]
fn unknown_missing_reference_name_gives_db_007() {
    let error = map_storage(missing_reference("some_other_fk"), known);
    assert_eq!(error.code(), ErrorCode::DB_007);
    assert!(matches!(
        error,
        AppError::Storage(StorageError::MissingReference { .. })
    ));
}

#[test]
fn other_storage_errors_keep_their_code_and_skip_the_closure() {
    let deadlock = StorageError::Deadlock {
        cause: crate::error::LockFailure::Deadlock,
    };
    let error = map_storage(deadlock, |_| Some(AppError::Unauthenticated));
    assert_eq!(error.code(), ErrorCode::DB_001);
}

#[test]
fn constraint_names_match_the_documents() {
    let names = [
        (MACHINE_CODE_LIVE_UNIQUE, "machine_code_live_unique"),
        (RECIPE_REFERENCE_UNIQUE, "recipe_reference_unique"),
        (UNIT_PATH_UNIQUE, "unit_path_unique"),
        (APP_USER_LOGIN_UNIQUE, "app_user_login_unique"),
        (PARAMETER_TAG_UNIQUE, "parameter_tag_unique"),
        (PLC_TYPE_NAMED_UNIQUE, "plc_type_named_unique"),
        (PLC_TYPE_MEMBER_NAME_UNIQUE, "plc_type_member_name_unique"),
        (
            RECIPE_VERSION_ONE_LIVE_DRAFT,
            "recipe_version_one_live_draft",
        ),
        (UNIT_PARENT_EXISTS, "unit_parent_exists"),
    ];
    for (constant, text) in names {
        assert_eq!(constant, text);
    }
}
