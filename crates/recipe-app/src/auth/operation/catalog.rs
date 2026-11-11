//! The table of the operation catalog (design D7, D5 of the change `low-round-c`).
//!
//! One `macro_rules!` table gives each variant its doc comment, its use case name, its command
//! flag, and its requirement. The macro generates the enum `Operation`, the list `ALL`, and the
//! accessors `name`, `is_command`, and `requirement`. The generated matches have no wildcard arm,
//! so a new row is the only way to add a variant, and a row without a requirement does not compile.

use recipe_domain::identity::Permission;

use super::Requirement;

/// Define the enum `Operation` and its accessors from one table.
///
/// The macro solves one problem: three exhaustive matches of 72 arms must not drift apart.
macro_rules! operation_catalog {
    (
        count: $count:literal;
        $(
            $(#[$meta:meta])*
            $variant:ident => {
                name: $name:literal,
                command: $command:literal,
                requires: $requires:expr $(,)?
            }
        ),+ $(,)?
    ) => {
        /// One use case or route of `api.md` section 3.3.
        ///
        /// `ExportVersion` has two variants, because the two export routes need different permissions.
        /// `ListDeletedRecipes` is `ListRecipes` with `show_deleted`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Operation {
            $(
                $(#[$meta])*
                $variant,
            )+
        }

        impl Operation {
            /// Every operation: 36 commands and 36 queries.
            pub const ALL: [Self; $count] = [$(Self::$variant),+];

            /// Return the name of the use case.
            ///
            /// Both export variants give `ExportVersion`. `ListDeletedRecipes` gives `ListRecipes`.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }

            /// Return true when the operation is a command and false when it is a query.
            #[must_use]
            pub const fn is_command(self) -> bool {
                match self {
                    $(Self::$variant => $command,)+
                }
            }

            /// Return the requirement of the operation.
            ///
            /// The match has no wildcard arm. A new variant without a row does not compile.
            #[must_use]
            pub const fn requirement(self) -> Requirement {
                match self {
                    $(Self::$variant => $requires,)+
                }
            }
        }
    };
}

operation_catalog! {
    count: 72;
    /// Command `CreateMachine`.
    CreateMachine => { name: "CreateMachine", command: true, requires: Requirement::Permission(Permission::MachineManage) },
    /// Command `RenameMachine`.
    RenameMachine => { name: "RenameMachine", command: true, requires: Requirement::Permission(Permission::MachineManage) },
    /// Command `ArchiveMachine`.
    ArchiveMachine => { name: "ArchiveMachine", command: true, requires: Requirement::Permission(Permission::MachineManage) },
    /// Command `AddUnit`.
    AddUnit => { name: "AddUnit", command: true, requires: Requirement::Permission(Permission::UnitManage) },
    /// Command `RenameUnit`.
    RenameUnit => { name: "RenameUnit", command: true, requires: Requirement::Permission(Permission::UnitManage) },
    /// Command `ArchiveUnit`.
    ArchiveUnit => { name: "ArchiveUnit", command: true, requires: Requirement::Permission(Permission::UnitManage) },
    /// Command `DefineUdt`.
    DefineUdt => { name: "DefineUdt", command: true, requires: Requirement::Permission(Permission::UdtManage) },
    /// Command `SetPlcConnection`.
    SetPlcConnection => { name: "SetPlcConnection", command: true, requires: Requirement::Permission(Permission::MachineManage) },
    /// Command `CreateRecipe`.
    CreateRecipe => { name: "CreateRecipe", command: true, requires: Requirement::Permission(Permission::RecipeCreate) },
    /// Command `RenameRecipe`.
    RenameRecipe => { name: "RenameRecipe", command: true, requires: Requirement::Permission(Permission::RecipeEdit) },
    /// Command `DeleteRecipe`.
    DeleteRecipe => { name: "DeleteRecipe", command: true, requires: Requirement::Permission(Permission::RecipeDelete) },
    /// Command `RestoreRecipe`.
    RestoreRecipe => { name: "RestoreRecipe", command: true, requires: Requirement::Permission(Permission::RecipeRestore) },
    /// Command `CreateVersion`.
    CreateVersion => { name: "CreateVersion", command: true, requires: Requirement::Permission(Permission::VersionCreate) },
    /// Command `DiscardDraft`.
    DiscardDraft => { name: "DiscardDraft", command: true, requires: Requirement::Permission(Permission::VersionDiscard) },
    /// Command `ApproveVersion`.
    ApproveVersion => { name: "ApproveVersion", command: true, requires: Requirement::Permission(Permission::VersionApprove) },
    /// Command `RetireVersion`.
    RetireVersion => { name: "RetireVersion", command: true, requires: Requirement::Permission(Permission::VersionRetire) },
    /// Command `AddParameter`.
    AddParameter => { name: "AddParameter", command: true, requires: Requirement::Permission(Permission::ParameterEdit) },
    /// Command `UpdateParameterDefinition`.
    UpdateParameterDefinition => { name: "UpdateParameterDefinition", command: true, requires: Requirement::Permission(Permission::ParameterEdit) },
    /// Command `SetParameterValue`.
    SetParameterValue => { name: "SetParameterValue", command: true, requires: Requirement::Permission(Permission::ParameterEdit) },
    /// Command `ReplaceLimits`.
    ReplaceLimits => { name: "ReplaceLimits", command: true, requires: Requirement::Permission(Permission::ParameterEdit) },
    /// Command `UpsertTranslation`.
    UpsertTranslation => { name: "UpsertTranslation", command: true, requires: Requirement::Permission(Permission::ParameterEdit) },
    /// Command `RemoveTranslation`.
    RemoveTranslation => { name: "RemoveTranslation", command: true, requires: Requirement::Permission(Permission::ParameterEdit) },
    /// Command `RemoveParameter`.
    RemoveParameter => { name: "RemoveParameter", command: true, requires: Requirement::Permission(Permission::ParameterEdit) },
    /// Command `RebindTypeRevision`.
    RebindTypeRevision => { name: "RebindTypeRevision", command: true, requires: Requirement::Permission(Permission::ParameterEdit) },
    /// Command `Login`.
    Login => { name: "Login", command: true, requires: Requirement::NoSession },
    /// Command `Logout`.
    Logout => { name: "Logout", command: true, requires: Requirement::Authenticated },
    /// Command `CreateUser`.
    CreateUser => { name: "CreateUser", command: true, requires: Requirement::Permission(Permission::UserManage) },
    /// Command `SetUserRole`.
    SetUserRole => { name: "SetUserRole", command: true, requires: Requirement::Permission(Permission::UserManage) },
    /// Command `DeactivateUser`.
    DeactivateUser => { name: "DeactivateUser", command: true, requires: Requirement::Permission(Permission::UserManage) },
    /// Command `ActivateUser`.
    ActivateUser => { name: "ActivateUser", command: true, requires: Requirement::Permission(Permission::UserManage) },
    /// Command `ResetPassword`.
    ResetPassword => { name: "ResetPassword", command: true, requires: Requirement::Permission(Permission::UserManage) },
    /// Command `ChangeOwnPassword`.
    ChangeOwnPassword => { name: "ChangeOwnPassword", command: true, requires: Requirement::Authenticated },
    /// Command `UnlockLogin`.
    UnlockLogin => { name: "UnlockLogin", command: true, requires: Requirement::Permission(Permission::LoginUnlock) },
    /// Command `ExportVersion` on the route `/approved`.
    ExportApprovedVersion => { name: "ExportVersion", command: true, requires: Requirement::Permission(Permission::VersionExport) },
    /// Command `ExportVersion` on the route `/versions/{n}`.
    ExportVersionByNumber => { name: "ExportVersion", command: true, requires: Requirement::Permission(Permission::VersionExportHistory) },
    /// Command `ExportPlcConnection` on the route `/export/v1/machines/{code}/connection`.
    ExportPlcConnection => { name: "ExportPlcConnection", command: true, requires: Requirement::Permission(Permission::VersionExport) },
    /// Query `ListMachines`.
    ListMachines => { name: "ListMachines", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetMachine`.
    GetMachine => { name: "GetMachine", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetUnitTree`.
    GetUnitTree => { name: "GetUnitTree", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetUnit`.
    GetUnit => { name: "GetUnit", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `ListTypes`.
    ListTypes => { name: "ListTypes", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetTypeTree`.
    GetTypeTree => { name: "GetTypeTree", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetTypeUsage`.
    GetTypeUsage => { name: "GetTypeUsage", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `ListRecipes`.
    ListRecipes => { name: "ListRecipes", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `ListRecipes` with `show_deleted`.
    ListDeletedRecipes => { name: "ListRecipes", command: false, requires: Requirement::Permission(Permission::RecipeRestore) },
    /// Query `ListExportableRecipes`.
    ListExportableRecipes => { name: "ListExportableRecipes", command: false, requires: Requirement::Permission(Permission::VersionExport) },
    /// Query `GetRecipe`.
    GetRecipe => { name: "GetRecipe", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `ListVersions`.
    ListVersions => { name: "ListVersions", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetVersion`.
    GetVersion => { name: "GetVersion", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `DiffVersions`.
    DiffVersions => { name: "DiffVersions", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetRecipeHistory`.
    GetRecipeHistory => { name: "GetRecipeHistory", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `ListParameters`.
    ListParameters => { name: "ListParameters", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetParameter`.
    GetParameter => { name: "GetParameter", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetLimits`.
    GetLimits => { name: "GetLimits", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `GetTranslations`.
    GetTranslations => { name: "GetTranslations", command: false, requires: Requirement::Permission(Permission::RecipeView) },
    /// Query `ListUsers`.
    ListUsers => { name: "ListUsers", command: false, requires: Requirement::Permission(Permission::UserManage) },
    /// Query `GetUser`.
    GetUser => { name: "GetUser", command: false, requires: Requirement::Permission(Permission::UserManage) },
    /// Query `ListAuditEntries`.
    ListAuditEntries => { name: "ListAuditEntries", command: false, requires: Requirement::AnyOf(Permission::AuditView, Permission::AuditViewRecipes) },
    /// Query `GetAuditEntry`.
    GetAuditEntry => { name: "GetAuditEntry", command: false, requires: Requirement::AnyOf(Permission::AuditView, Permission::AuditViewRecipes) },
    /// Query `ListSecurityEvents`.
    ListSecurityEvents => { name: "ListSecurityEvents", command: false, requires: Requirement::Permission(Permission::AuditView) },
    /// Query `GetOwnProfile`.
    GetOwnProfile => { name: "GetOwnProfile", command: false, requires: Requirement::Authenticated },
    /// Form query `GetMachineForm`.
    GetMachineForm => { name: "GetMachineForm", command: false, requires: Requirement::Permission(Permission::MachineManage) },
    /// Form query `GetUnitForm`.
    GetUnitForm => { name: "GetUnitForm", command: false, requires: Requirement::Permission(Permission::UnitManage) },
    /// Form query `GetUdtForm`.
    GetUdtForm => { name: "GetUdtForm", command: false, requires: Requirement::Permission(Permission::UdtManage) },
    /// Form query `GetRecipeForm`.
    GetRecipeForm => { name: "GetRecipeForm", command: false, requires: Requirement::AnyOf(Permission::RecipeCreate, Permission::RecipeEdit) },
    /// Form query `GetVersionForm`.
    GetVersionForm => { name: "GetVersionForm", command: false, requires: Requirement::Permission(Permission::VersionCreate) },
    /// Form query `GetParameterForm`.
    GetParameterForm => { name: "GetParameterForm", command: false, requires: Requirement::Permission(Permission::ParameterEdit) },
    /// Preview query `GetApprovalPreview`.
    GetApprovalPreview => { name: "GetApprovalPreview", command: false, requires: Requirement::Permission(Permission::VersionApprove) },
    /// Form query `GetUserForm`.
    GetUserForm => { name: "GetUserForm", command: false, requires: Requirement::Permission(Permission::UserManage) },
    /// Form query `GetLoginForm`.
    GetLoginForm => { name: "GetLoginForm", command: false, requires: Requirement::NoSession },
    /// Form query `GetOwnPasswordForm`.
    GetOwnPasswordForm => { name: "GetOwnPasswordForm", command: false, requires: Requirement::Authenticated },
    /// Form query `GetPlcConnectionForm`.
    GetPlcConnectionForm => { name: "GetPlcConnectionForm", command: false, requires: Requirement::Permission(Permission::MachineManage) },
}
