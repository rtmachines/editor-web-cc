use std::collections::BTreeSet;

use recipe_domain::identity::{Permission, Role};

use super::*;

/// The 36 commands of `domain-model.md` sections 5.1 to 5.4.
const COMMAND_NAMES: [&str; 36] = [
    "CreateMachine",
    "RenameMachine",
    "SetMachineDataBlock",
    "ArchiveMachine",
    "AddUnit",
    "RenameUnit",
    "ArchiveUnit",
    "DefineUdt",
    "SetPlcConnection",
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
    "ExportPlcConnection",
];

/// The 35 queries of `domain-model.md` section 5.5.
const QUERY_NAMES: [&str; 35] = [
    "ListMachines",
    "GetMachine",
    "GetUnitTree",
    "GetUnit",
    "ListTypes",
    "GetTypeTree",
    "GetTypeUsage",
    "ListRecipes",
    "ListExportableRecipes",
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
    "GetPlcConnectionForm",
];

/// One row per variant: variant, requirement, use case name, command flag.
type Row = (Operation, Requirement, &'static str, bool);

/// The pinned facts of every operation, in the order of `Operation::ALL`.
const TABLE: [Row; 73] = [
    (
        Operation::CreateMachine,
        Requirement::Permission(Permission::MachineManage),
        "CreateMachine",
        true,
    ),
    (
        Operation::RenameMachine,
        Requirement::Permission(Permission::MachineManage),
        "RenameMachine",
        true,
    ),
    (
        Operation::SetMachineDataBlock,
        Requirement::Permission(Permission::MachineManage),
        "SetMachineDataBlock",
        true,
    ),
    (
        Operation::ArchiveMachine,
        Requirement::Permission(Permission::MachineManage),
        "ArchiveMachine",
        true,
    ),
    (
        Operation::AddUnit,
        Requirement::Permission(Permission::UnitManage),
        "AddUnit",
        true,
    ),
    (
        Operation::RenameUnit,
        Requirement::Permission(Permission::UnitManage),
        "RenameUnit",
        true,
    ),
    (
        Operation::ArchiveUnit,
        Requirement::Permission(Permission::UnitManage),
        "ArchiveUnit",
        true,
    ),
    (
        Operation::DefineUdt,
        Requirement::Permission(Permission::UdtManage),
        "DefineUdt",
        true,
    ),
    (
        Operation::SetPlcConnection,
        Requirement::Permission(Permission::MachineManage),
        "SetPlcConnection",
        true,
    ),
    (
        Operation::CreateRecipe,
        Requirement::Permission(Permission::RecipeCreate),
        "CreateRecipe",
        true,
    ),
    (
        Operation::RenameRecipe,
        Requirement::Permission(Permission::RecipeEdit),
        "RenameRecipe",
        true,
    ),
    (
        Operation::DeleteRecipe,
        Requirement::Permission(Permission::RecipeDelete),
        "DeleteRecipe",
        true,
    ),
    (
        Operation::RestoreRecipe,
        Requirement::Permission(Permission::RecipeRestore),
        "RestoreRecipe",
        true,
    ),
    (
        Operation::CreateVersion,
        Requirement::Permission(Permission::VersionCreate),
        "CreateVersion",
        true,
    ),
    (
        Operation::DiscardDraft,
        Requirement::Permission(Permission::VersionDiscard),
        "DiscardDraft",
        true,
    ),
    (
        Operation::ApproveVersion,
        Requirement::Permission(Permission::VersionApprove),
        "ApproveVersion",
        true,
    ),
    (
        Operation::RetireVersion,
        Requirement::Permission(Permission::VersionRetire),
        "RetireVersion",
        true,
    ),
    (
        Operation::AddParameter,
        Requirement::Permission(Permission::ParameterEdit),
        "AddParameter",
        true,
    ),
    (
        Operation::UpdateParameterDefinition,
        Requirement::Permission(Permission::ParameterEdit),
        "UpdateParameterDefinition",
        true,
    ),
    (
        Operation::SetParameterValue,
        Requirement::Permission(Permission::ParameterEdit),
        "SetParameterValue",
        true,
    ),
    (
        Operation::ReplaceLimits,
        Requirement::Permission(Permission::ParameterEdit),
        "ReplaceLimits",
        true,
    ),
    (
        Operation::UpsertTranslation,
        Requirement::Permission(Permission::ParameterEdit),
        "UpsertTranslation",
        true,
    ),
    (
        Operation::RemoveTranslation,
        Requirement::Permission(Permission::ParameterEdit),
        "RemoveTranslation",
        true,
    ),
    (
        Operation::RemoveParameter,
        Requirement::Permission(Permission::ParameterEdit),
        "RemoveParameter",
        true,
    ),
    (
        Operation::RebindTypeRevision,
        Requirement::Permission(Permission::ParameterEdit),
        "RebindTypeRevision",
        true,
    ),
    (Operation::Login, Requirement::NoSession, "Login", true),
    (
        Operation::Logout,
        Requirement::Authenticated,
        "Logout",
        true,
    ),
    (
        Operation::CreateUser,
        Requirement::Permission(Permission::UserManage),
        "CreateUser",
        true,
    ),
    (
        Operation::SetUserRole,
        Requirement::Permission(Permission::UserManage),
        "SetUserRole",
        true,
    ),
    (
        Operation::DeactivateUser,
        Requirement::Permission(Permission::UserManage),
        "DeactivateUser",
        true,
    ),
    (
        Operation::ActivateUser,
        Requirement::Permission(Permission::UserManage),
        "ActivateUser",
        true,
    ),
    (
        Operation::ResetPassword,
        Requirement::Permission(Permission::UserManage),
        "ResetPassword",
        true,
    ),
    (
        Operation::ChangeOwnPassword,
        Requirement::Authenticated,
        "ChangeOwnPassword",
        true,
    ),
    (
        Operation::UnlockLogin,
        Requirement::Permission(Permission::LoginUnlock),
        "UnlockLogin",
        true,
    ),
    (
        Operation::ExportApprovedVersion,
        Requirement::Permission(Permission::VersionExport),
        "ExportVersion",
        true,
    ),
    (
        Operation::ExportVersionByNumber,
        Requirement::Permission(Permission::VersionExportHistory),
        "ExportVersion",
        true,
    ),
    (
        Operation::ExportPlcConnection,
        Requirement::Permission(Permission::VersionExport),
        "ExportPlcConnection",
        true,
    ),
    (
        Operation::ListMachines,
        Requirement::Permission(Permission::RecipeView),
        "ListMachines",
        false,
    ),
    (
        Operation::GetMachine,
        Requirement::Permission(Permission::RecipeView),
        "GetMachine",
        false,
    ),
    (
        Operation::GetUnitTree,
        Requirement::Permission(Permission::RecipeView),
        "GetUnitTree",
        false,
    ),
    (
        Operation::GetUnit,
        Requirement::Permission(Permission::RecipeView),
        "GetUnit",
        false,
    ),
    (
        Operation::ListTypes,
        Requirement::Permission(Permission::RecipeView),
        "ListTypes",
        false,
    ),
    (
        Operation::GetTypeTree,
        Requirement::Permission(Permission::RecipeView),
        "GetTypeTree",
        false,
    ),
    (
        Operation::GetTypeUsage,
        Requirement::Permission(Permission::RecipeView),
        "GetTypeUsage",
        false,
    ),
    (
        Operation::ListRecipes,
        Requirement::Permission(Permission::RecipeView),
        "ListRecipes",
        false,
    ),
    (
        Operation::ListDeletedRecipes,
        Requirement::Permission(Permission::RecipeRestore),
        "ListRecipes",
        false,
    ),
    (
        Operation::ListExportableRecipes,
        Requirement::Permission(Permission::VersionExport),
        "ListExportableRecipes",
        false,
    ),
    (
        Operation::GetRecipe,
        Requirement::Permission(Permission::RecipeView),
        "GetRecipe",
        false,
    ),
    (
        Operation::ListVersions,
        Requirement::Permission(Permission::RecipeView),
        "ListVersions",
        false,
    ),
    (
        Operation::GetVersion,
        Requirement::Permission(Permission::RecipeView),
        "GetVersion",
        false,
    ),
    (
        Operation::DiffVersions,
        Requirement::Permission(Permission::RecipeView),
        "DiffVersions",
        false,
    ),
    (
        Operation::GetRecipeHistory,
        Requirement::Permission(Permission::RecipeView),
        "GetRecipeHistory",
        false,
    ),
    (
        Operation::ListParameters,
        Requirement::Permission(Permission::RecipeView),
        "ListParameters",
        false,
    ),
    (
        Operation::GetParameter,
        Requirement::Permission(Permission::RecipeView),
        "GetParameter",
        false,
    ),
    (
        Operation::GetLimits,
        Requirement::Permission(Permission::RecipeView),
        "GetLimits",
        false,
    ),
    (
        Operation::GetTranslations,
        Requirement::Permission(Permission::RecipeView),
        "GetTranslations",
        false,
    ),
    (
        Operation::ListUsers,
        Requirement::Permission(Permission::UserManage),
        "ListUsers",
        false,
    ),
    (
        Operation::GetUser,
        Requirement::Permission(Permission::UserManage),
        "GetUser",
        false,
    ),
    (
        Operation::ListAuditEntries,
        Requirement::AnyOf(Permission::AuditView, Permission::AuditViewRecipes),
        "ListAuditEntries",
        false,
    ),
    (
        Operation::GetAuditEntry,
        Requirement::AnyOf(Permission::AuditView, Permission::AuditViewRecipes),
        "GetAuditEntry",
        false,
    ),
    (
        Operation::ListSecurityEvents,
        Requirement::Permission(Permission::AuditView),
        "ListSecurityEvents",
        false,
    ),
    (
        Operation::GetOwnProfile,
        Requirement::Authenticated,
        "GetOwnProfile",
        false,
    ),
    (
        Operation::GetMachineForm,
        Requirement::Permission(Permission::MachineManage),
        "GetMachineForm",
        false,
    ),
    (
        Operation::GetUnitForm,
        Requirement::Permission(Permission::UnitManage),
        "GetUnitForm",
        false,
    ),
    (
        Operation::GetUdtForm,
        Requirement::Permission(Permission::UdtManage),
        "GetUdtForm",
        false,
    ),
    (
        Operation::GetRecipeForm,
        Requirement::AnyOf(Permission::RecipeCreate, Permission::RecipeEdit),
        "GetRecipeForm",
        false,
    ),
    (
        Operation::GetVersionForm,
        Requirement::Permission(Permission::VersionCreate),
        "GetVersionForm",
        false,
    ),
    (
        Operation::GetParameterForm,
        Requirement::Permission(Permission::ParameterEdit),
        "GetParameterForm",
        false,
    ),
    (
        Operation::GetApprovalPreview,
        Requirement::Permission(Permission::VersionApprove),
        "GetApprovalPreview",
        false,
    ),
    (
        Operation::GetUserForm,
        Requirement::Permission(Permission::UserManage),
        "GetUserForm",
        false,
    ),
    (
        Operation::GetLoginForm,
        Requirement::NoSession,
        "GetLoginForm",
        false,
    ),
    (
        Operation::GetOwnPasswordForm,
        Requirement::Authenticated,
        "GetOwnPasswordForm",
        false,
    ),
    (
        Operation::GetPlcConnectionForm,
        Requirement::Permission(Permission::MachineManage),
        "GetPlcConnectionForm",
        false,
    ),
];

#[test]
fn table_pins_requirement_name_and_command_flag_of_every_operation() {
    assert_eq!(TABLE.len(), Operation::ALL.len());
    for (operation, requirement, name, is_command) in TABLE {
        assert_eq!(
            operation.requirement(),
            requirement,
            "requirement of {operation:?}"
        );
        assert_eq!(operation.name(), name, "name of {operation:?}");
        assert_eq!(
            operation.is_command(),
            is_command,
            "command flag of {operation:?}"
        );
    }
    let listed: Vec<Operation> = TABLE.iter().map(|row| row.0).collect();
    assert_eq!(listed, Operation::ALL.to_vec());
}

#[test]
fn catalog_has_73_distinct_operations() {
    let distinct: BTreeSet<Operation> = Operation::ALL.iter().copied().collect();
    assert_eq!(Operation::ALL.len(), 73);
    assert_eq!(distinct.len(), 73);
}

#[test]
fn names_equal_the_commands_and_queries_of_the_domain_model() {
    let expected: BTreeSet<&str> = COMMAND_NAMES.iter().chain(&QUERY_NAMES).copied().collect();
    let actual: BTreeSet<&str> = Operation::ALL.iter().map(|op| op.name()).collect();
    assert_eq!(expected.len(), 71);
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
fn exactly_37_variants_are_commands() {
    let commands = Operation::ALL.iter().filter(|op| op.is_command()).count();
    assert_eq!(commands, 37);
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
            Operation::ExportPlcConnection,
            Requirement::Permission(Permission::VersionExport),
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
fn read_only_mode_runs_every_query_and_five_commands() {
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
        Operation::ExportPlcConnection,
    ];
    let expected: BTreeSet<Operation> = queries.chain(commands).collect();
    assert_eq!(allowed.len(), 41);
    assert_eq!(allowed, expected);
}

#[test]
fn set_machine_data_block_needs_machine_manage_and_stops_in_read_only_mode() {
    let operation = Operation::SetMachineDataBlock;
    assert!(operation.is_command());
    assert_eq!(operation.name(), "SetMachineDataBlock");
    assert_eq!(
        operation.requirement(),
        Requirement::Permission(Permission::MachineManage)
    );
    assert!(!operation.runs_in_read_only());
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
    assert!(!Requirement::Authenticated.allows(Role::ExportClient));
    assert!(!Requirement::NoSession.allows(Role::ExportClient));
}
