use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::ExportError;
use recipe_domain::identity::{IdentityError, Login};
use recipe_domain::plant::{MachineCode, MachineId, PlantError, TypeName, UnitPath};
use recipe_domain::recipe::{DraftError, RecipeError, RecipeErrors, ReferenceNumber};
use uuid::Uuid;

use super::*;
use crate::auth::Operation;
use crate::error::StorageError;

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-100").unwrap()
}

fn machine_code() -> MachineCode {
    MachineCode::parse("M12").unwrap()
}

fn unit_path() -> UnitPath {
    UnitPath::parse("U1.2").unwrap()
}

fn recipe_error() -> RecipeError {
    RecipeError::VersionNotDraft {
        reference: reference(),
        number: 3,
    }
}

fn plant_error() -> PlantError {
    PlantError::DuplicateMachineCode {
        code: machine_code(),
    }
}

fn concurrent_edit() -> AppError {
    AppError::ConcurrentEdit {
        reference: reference(),
        number: 3,
        expected: 4,
        actual: 5,
    }
}

#[test]
fn own_variants_have_their_codes_and_messages() {
    let cases = [
        (
            concurrent_edit(),
            ErrorCode::WEB_001,
            "Another user changed version 3 of recipe R-100. Reload the page and apply your change again.",
        ),
        (
            AppError::Forbidden {
                operation: Operation::ApproveVersion,
            },
            ErrorCode::WEB_002,
            "Your role does not permit the operation ApproveVersion.",
        ),
        (
            AppError::Unauthenticated,
            ErrorCode::WEB_004,
            "Log in to continue.",
        ),
        (
            AppError::NotFound {
                entity: Missing::Recipe(reference()),
            },
            ErrorCode::WEB_005,
            "Recipe R-100 does not exist.",
        ),
        (
            AppError::Timeout {
                stage: TimeoutStage::Validation,
            },
            ErrorCode::WEB_009,
            "The request took too long and was stopped. Try again.",
        ),
        (
            AppError::TaskFailed {
                stage: TaskStage::PasswordHash,
            },
            ErrorCode::WEB_011,
            "The operation password hash did not complete. Try again.",
        ),
        (
            AppError::AuditWriteFailed {
                cause: StorageError::Unavailable,
            },
            ErrorCode::AUD_001,
            "The change was not saved. The system could not record the audit entry. Contact the administrator.",
        ),
    ];
    for (error, code, message) in cases {
        assert_eq!(error.code(), code, "{error:?}");
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn stages_display_in_plain_words() {
    assert_eq!(
        TimeoutStage::Argon2Permit.to_string(),
        "password hash permit"
    );
    assert_eq!(TimeoutStage::Validation.to_string(), "validation");
    assert_eq!(TimeoutStage::Export.to_string(), "export");
    assert_eq!(TimeoutStage::Request.to_string(), "request");
    assert_eq!(TaskStage::PasswordHash.to_string(), "password hash");
    assert_eq!(TaskStage::Validation.to_string(), "validation");
    assert_eq!(TaskStage::Token.to_string(), "token generation");
}

#[test]
fn the_token_stage_gives_e_web_011() {
    let error = AppError::TaskFailed {
        stage: TaskStage::Token,
    };
    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert_eq!(
        error.to_string(),
        "The operation token generation did not complete. Try again."
    );
}

#[test]
fn domain_variants_keep_code_and_message() {
    let plant = AppError::from(plant_error());
    assert_eq!(plant.code(), ErrorCode::PLT_001);
    assert_eq!(plant.to_string(), plant_error().to_string());

    let recipe = AppError::from(recipe_error());
    assert_eq!(recipe.code(), ErrorCode::RCP_007);
    assert_eq!(recipe.to_string(), recipe_error().to_string());

    let export = AppError::from(ExportError::NoApprovedVersion {
        reference: reference(),
    });
    assert_eq!(export.code(), ErrorCode::EXP_001);
    assert_eq!(export.to_string(), "Recipe R-100 has no approved version.");
}

#[test]
fn recipe_errors_keep_the_code_of_the_first_entry() {
    let mut list = RecipeErrors::from(recipe_error());
    list.push(RecipeError::RecipeDeleted {
        reference: reference(),
    });
    let error = AppError::from(list);
    assert!(matches!(error, AppError::Recipes(_)));
    assert_eq!(error.code(), ErrorCode::RCP_007);
}

#[test]
fn serialize_error_gives_exp_004() {
    let source = serde_json::from_str::<u8>("x").unwrap_err();
    let error = AppError::from(ExportError::from(source));
    assert_eq!(error.code(), ErrorCode::EXP_004);
}

#[test]
fn storage_error_keeps_its_code() {
    let deadlock = StorageError::Deadlock {
        cause: crate::error::LockFailure::Deadlock,
    };
    let text = deadlock.to_string();
    let error = AppError::from(deadlock);
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(error.to_string(), text);
}

#[test]
fn identity_error_keeps_its_code_and_text() {
    let inner = IdentityError::MustChangePassword;
    let text = inner.to_string();
    let code = inner.code();
    let error = AppError::from(inner);
    assert!(matches!(error, AppError::Identity(_)));
    assert_eq!(error.code(), code);
    assert_eq!(error.code(), ErrorCode::IDN_007);
    assert_eq!(error.to_string(), text);
}

#[test]
fn identity_error_with_a_login_passes_its_code() {
    let error = AppError::from(IdentityError::UserAlreadyActive {
        login: Login::parse("anna").unwrap(),
    });
    assert_eq!(error.code(), ErrorCode::IDN_009);
}

#[test]
fn draft_error_unwraps_the_recipe_kind() {
    let error = AppError::from(DraftError::from(recipe_error()));
    assert!(matches!(error, AppError::Recipe(_)));
    assert_eq!(error.code(), ErrorCode::RCP_007);
}

#[test]
fn draft_error_unwraps_the_plant_kind() {
    let error = AppError::from(DraftError::from(plant_error()));
    assert!(matches!(error, AppError::Plant(_)));
    assert_eq!(error.code(), ErrorCode::PLT_001);
}

#[test]
fn draft_error_unwraps_the_list_kind() {
    let error = AppError::from(DraftError::from(RecipeErrors::from(recipe_error())));
    assert!(matches!(error, AppError::Recipes(_)));
    assert_eq!(error.code(), ErrorCode::RCP_007);
}

#[test]
fn missing_malformed_login_names_a_user_and_never_the_typed_text() {
    let text = Missing::MalformedLogin.to_string();
    assert_eq!(text, "User");
    let error = AppError::NotFound {
        entity: Missing::MalformedLogin,
    };
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(!error.to_string().contains("Bad Login!"));
}

#[test]
fn missing_entities_display_their_text() {
    let id = MachineId::from(Uuid::from_u128(9));
    let cases = [
        (Missing::Machine(machine_code()), "Machine M12".to_owned()),
        (Missing::MachineId(id), format!("Machine {id}")),
        (
            Missing::Unit {
                code: machine_code(),
                path: unit_path(),
            },
            "Unit U1.2 of machine M12".to_owned(),
        ),
        (
            Missing::Type {
                code: machine_code(),
                name: TypeName::parse("ST_Zone").unwrap(),
            },
            "Type ST_Zone of machine M12".to_owned(),
        ),
        (Missing::Recipe(reference()), "Recipe R-100".to_owned()),
        (
            Missing::Version {
                reference: reference(),
                number: 3,
            },
            "Version 3 of recipe R-100".to_owned(),
        ),
        (
            Missing::User(Login::parse("anna").unwrap()),
            "User anna".to_owned(),
        ),
    ];
    for (missing, text) in cases {
        assert_eq!(missing.to_string(), text);
    }
}
