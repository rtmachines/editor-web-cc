use std::collections::BTreeSet;

use recipe_domain::identity::{Permission, Role};

use super::*;

/// The 33 commands of `domain-model.md` sections 5.1 to 5.4.
const COMMAND_NAMES: [&str; 33] = [
    "CreateMachine",
    "RenameMachine",
    "ArchiveMachine",
    "AddUnit",
    "RenameUnit",
    "ArchiveUnit",
    "DefineUdt",
    "CreateRecipe",
    "RenameRecipe",
    "DeleteRecipe",
    "RestoreRecipe",
    "CreateVersion",
    "DiscardDraft",
    "ApproveVersion",
    "RetireVersion",
    "AddParameter",
    "UpdateParameterDefinition",
    "SetParameterValue",
    "ReplaceLimits",
    "UpsertTranslation",
    "RemoveTranslation",
    "RemoveParameter",
    "RebindTypeRevision",
    "Login",
    "Logout",
    "CreateUser",
    "SetUserRole",
    "DeactivateUser",
    "ActivateUser",
    "ResetPassword",
    "ChangeOwnPassword",
    "UnlockLogin",
    "ExportVersion",
];

/// The 33 queries of `domain-model.md` section 5.5.
const QUERY_NAMES: [&str; 33] = [
    "ListMachines",
    "GetMachine",
    "GetUnitTree",
    "GetUnit",
    "ListTypes",
    "GetTypeTree",
    "GetTypeUsage",
    "ListRecipes",
    "GetRecipe",
    "ListVersions",
    "GetVersion",
    "DiffVersions",
    "GetRecipeHistory",
    "ListParameters",
    "GetParameter",
    "GetLimits",
    "GetTranslations",
    "ListUsers",
    "GetUser",
    "ListAuditEntries",
    "GetAuditEntry",
    "ListSecurityEvents",
    "GetOwnProfile",
    "GetMachineForm",
    "GetUnitForm",
    "GetUdtForm",
    "GetRecipeForm",
    "GetVersionForm",
    "GetParameterForm",
    "GetApprovalPreview",
    "GetUserForm",
    "GetLoginForm",
    "GetOwnPasswordForm",
];

#[test]
fn catalog_has_68_distinct_operations() {
    let distinct: BTreeSet<Operation> = Operation::ALL.iter().copied().collect();
    assert_eq!(Operation::ALL.len(), 68);
    assert_eq!(distinct.len(), 68);
}

#[test]
fn names_equal_the_commands_and_queries_of_the_domain_model() {
    let expected: BTreeSet<&str> = COMMAND_NAMES.iter().chain(&QUERY_NAMES).copied().collect();
    let actual: BTreeSet<&str> = Operation::ALL.iter().map(|op| op.name()).collect();
    assert_eq!(expected.len(), 66);
    assert_eq!(actual, expected);
}

#[test]
fn both_export_variants_and_the_deleted_list_share_a_use_case_name() {
    assert_eq!(Operation::ExportApprovedVersion.name(), "ExportVersion");
    assert_eq!(Operation::ExportVersionByNumber.name(), "ExportVersion");
    assert_eq!(Operation::ListDeletedRecipes.name(), "ListRecipes");
    assert_eq!(Operation::ListRecipes.name(), "ListRecipes");
}

#[test]
fn display_gives_the_use_case_name() {
    assert_eq!(Operation::ApproveVersion.to_string(), "ApproveVersion");
    assert_eq!(
        Operation::ExportVersionByNumber.to_string(),
        "ExportVersion"
    );
}

#[test]
fn exactly_34_variants_are_commands() {
    let commands = Operation::ALL.iter().filter(|op| op.is_command()).count();
    assert_eq!(commands, 34);
    let command_names: BTreeSet<&str> = Operation::ALL
        .iter()
        .filter(|op| op.is_command())
        .map(|op| op.name())
        .collect();
    let expected: BTreeSet<&str> = COMMAND_NAMES.iter().copied().collect();
    assert_eq!(command_names, expected);
}

#[test]
fn requirement_of_a_sample_matches_the_matrix() {
    let rows = [
        (
            Operation::CreateMachine,
            Requirement::Permission(Permission::MachineManage),
        ),
        (
            Operation::GetUnitForm,
            Requirement::Permission(Permission::UnitManage),
        ),
        (
            Operation::DeleteRecipe,
            Requirement::Permission(Permission::RecipeDelete),
        ),
        (
            Operation::ListDeletedRecipes,
            Requirement::Permission(Permission::RecipeRestore),
        ),
        (
            Operation::RebindTypeRevision,
            Requirement::Permission(Permission::ParameterEdit),
        ),
        (
            Operation::ExportApprovedVersion,
            Requirement::Permission(Permission::VersionExport),
        ),
        (
            Operation::ExportVersionByNumber,
            Requirement::Permission(Permission::VersionExportHistory),
        ),
        (
            Operation::GetAuditEntry,
            Requirement::AnyOf(Permission::AuditView, Permission::AuditViewRecipes),
        ),
        (
            Operation::ListSecurityEvents,
            Requirement::Permission(Permission::AuditView),
        ),
        (
            Operation::GetRecipeHistory,
            Requirement::Permission(Permission::RecipeView),
        ),
        (Operation::GetLoginForm, Requirement::NoSession),
        (Operation::GetOwnProfile, Requirement::Authenticated),
    ];
    for (operation, requirement) in rows {
        assert_eq!(operation.requirement(), requirement, "{operation}");
    }
}

#[test]
fn read_only_mode_runs_every_query_and_four_commands() {
    let allowed: BTreeSet<Operation> = Operation::ALL
        .iter()
        .copied()
        .filter(|op| op.runs_in_read_only())
        .collect();
    let queries = Operation::ALL.iter().filter(|op| !op.is_command()).copied();
    let commands = [
        Operation::Login,
        Operation::Logout,
        Operation::ExportApprovedVersion,
        Operation::ExportVersionByNumber,
    ];
    let expected: BTreeSet<Operation> = queries.chain(commands).collect();
    assert_eq!(allowed.len(), 38);
    assert_eq!(allowed, expected);
}

#[test]
fn requirement_allows_follows_the_role() {
    let manage = Requirement::Permission(Permission::MachineManage);
    assert!(manage.allows(Role::ProcessEngineer));
    assert!(!manage.allows(Role::Operator));
    let audit = Requirement::AnyOf(Permission::AuditView, Permission::AuditViewRecipes);
    assert!(audit.allows(Role::Administrator));
    assert!(audit.allows(Role::ProcessEngineer));
    assert!(!audit.allows(Role::Operator));
    for role in Role::ALL {
        assert!(Requirement::Authenticated.allows(role));
        assert!(Requirement::NoSession.allows(role));
    }
}
