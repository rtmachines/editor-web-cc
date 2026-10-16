//! The operation catalog (design D7).
//!
//! The catalog solves one problem: a use case without a row in the permission matrix must not compile.
//! `Operation::requirement` has no wildcard arm, so a new variant forces a new matrix row.

use std::fmt;

mod purpose;
mod requirement;

pub use purpose::{
    FormPurpose, MachineFormPurpose, ParameterFormPurpose, RecipeFormPurpose, UdtFormPurpose,
    UnitFormPurpose, VersionFormPurpose,
};
pub use requirement::Requirement;

/// One use case or route of `api.md` section 3.3.
///
/// `ExportVersion` has two variants, because the two export routes need different permissions.
/// `ListDeletedRecipes` is `ListRecipes` with `show_deleted`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Operation {
    /// Command `CreateMachine`.
    CreateMachine,
    /// Command `RenameMachine`.
    RenameMachine,
    /// Command `ArchiveMachine`.
    ArchiveMachine,
    /// Command `AddUnit`.
    AddUnit,
    /// Command `RenameUnit`.
    RenameUnit,
    /// Command `ArchiveUnit`.
    ArchiveUnit,
    /// Command `DefineUdt`.
    DefineUdt,
    /// Command `CreateRecipe`.
    CreateRecipe,
    /// Command `RenameRecipe`.
    RenameRecipe,
    /// Command `DeleteRecipe`.
    DeleteRecipe,
    /// Command `RestoreRecipe`.
    RestoreRecipe,
    /// Command `CreateVersion`.
    CreateVersion,
    /// Command `DiscardDraft`.
    DiscardDraft,
    /// Command `ApproveVersion`.
    ApproveVersion,
    /// Command `RetireVersion`.
    RetireVersion,
    /// Command `AddParameter`.
    AddParameter,
    /// Command `UpdateParameterDefinition`.
    UpdateParameterDefinition,
    /// Command `SetParameterValue`.
    SetParameterValue,
    /// Command `ReplaceLimits`.
    ReplaceLimits,
    /// Command `UpsertTranslation`.
    UpsertTranslation,
    /// Command `RemoveTranslation`.
    RemoveTranslation,
    /// Command `RemoveParameter`.
    RemoveParameter,
    /// Command `RebindTypeRevision`.
    RebindTypeRevision,
    /// Command `Login`.
    Login,
    /// Command `Logout`.
    Logout,
    /// Command `CreateUser`.
    CreateUser,
    /// Command `SetUserRole`.
    SetUserRole,
    /// Command `DeactivateUser`.
    DeactivateUser,
    /// Command `ActivateUser`.
    ActivateUser,
    /// Command `ResetPassword`.
    ResetPassword,
    /// Command `ChangeOwnPassword`.
    ChangeOwnPassword,
    /// Command `UnlockLogin`.
    UnlockLogin,
    /// Command `ExportVersion` on the route `/approved`.
    ExportApprovedVersion,
    /// Command `ExportVersion` on the route `/versions/{n}`.
    ExportVersionByNumber,
    /// Query `ListMachines`.
    ListMachines,
    /// Query `GetMachine`.
    GetMachine,
    /// Query `GetUnitTree`.
    GetUnitTree,
    /// Query `GetUnit`.
    GetUnit,
    /// Query `ListTypes`.
    ListTypes,
    /// Query `GetTypeTree`.
    GetTypeTree,
    /// Query `GetTypeUsage`.
    GetTypeUsage,
    /// Query `ListRecipes`.
    ListRecipes,
    /// Query `ListRecipes` with `show_deleted`.
    ListDeletedRecipes,
    /// Query `GetRecipe`.
    GetRecipe,
    /// Query `ListVersions`.
    ListVersions,
    /// Query `GetVersion`.
    GetVersion,
    /// Query `DiffVersions`.
    DiffVersions,
    /// Query `GetRecipeHistory`.
    GetRecipeHistory,
    /// Query `ListParameters`.
    ListParameters,
    /// Query `GetParameter`.
    GetParameter,
    /// Query `GetLimits`.
    GetLimits,
    /// Query `GetTranslations`.
    GetTranslations,
    /// Query `ListUsers`.
    ListUsers,
    /// Query `GetUser`.
    GetUser,
    /// Query `ListAuditEntries`.
    ListAuditEntries,
    /// Query `GetAuditEntry`.
    GetAuditEntry,
    /// Query `ListSecurityEvents`.
    ListSecurityEvents,
    /// Query `GetOwnProfile`.
    GetOwnProfile,
    /// Form query `GetMachineForm`.
    GetMachineForm,
    /// Form query `GetUnitForm`.
    GetUnitForm,
    /// Form query `GetUdtForm`.
    GetUdtForm,
    /// Form query `GetRecipeForm`.
    GetRecipeForm,
    /// Form query `GetVersionForm`.
    GetVersionForm,
    /// Form query `GetParameterForm`.
    GetParameterForm,
    /// Preview query `GetApprovalPreview`.
    GetApprovalPreview,
    /// Form query `GetUserForm`.
    GetUserForm,
    /// Form query `GetLoginForm`.
    GetLoginForm,
    /// Form query `GetOwnPasswordForm`.
    GetOwnPasswordForm,
}

impl Operation {
    /// Every operation: 34 commands and 34 queries.
    pub const ALL: [Self; 68] = [
        Self::CreateMachine,
        Self::RenameMachine,
        Self::ArchiveMachine,
        Self::AddUnit,
        Self::RenameUnit,
        Self::ArchiveUnit,
        Self::DefineUdt,
        Self::CreateRecipe,
        Self::RenameRecipe,
        Self::DeleteRecipe,
        Self::RestoreRecipe,
        Self::CreateVersion,
        Self::DiscardDraft,
        Self::ApproveVersion,
        Self::RetireVersion,
        Self::AddParameter,
        Self::UpdateParameterDefinition,
        Self::SetParameterValue,
        Self::ReplaceLimits,
        Self::UpsertTranslation,
        Self::RemoveTranslation,
        Self::RemoveParameter,
        Self::RebindTypeRevision,
        Self::Login,
        Self::Logout,
        Self::CreateUser,
        Self::SetUserRole,
        Self::DeactivateUser,
        Self::ActivateUser,
        Self::ResetPassword,
        Self::ChangeOwnPassword,
        Self::UnlockLogin,
        Self::ExportApprovedVersion,
        Self::ExportVersionByNumber,
        Self::ListMachines,
        Self::GetMachine,
        Self::GetUnitTree,
        Self::GetUnit,
        Self::ListTypes,
        Self::GetTypeTree,
        Self::GetTypeUsage,
        Self::ListRecipes,
        Self::ListDeletedRecipes,
        Self::GetRecipe,
        Self::ListVersions,
        Self::GetVersion,
        Self::DiffVersions,
        Self::GetRecipeHistory,
        Self::ListParameters,
        Self::GetParameter,
        Self::GetLimits,
        Self::GetTranslations,
        Self::ListUsers,
        Self::GetUser,
        Self::ListAuditEntries,
        Self::GetAuditEntry,
        Self::ListSecurityEvents,
        Self::GetOwnProfile,
        Self::GetMachineForm,
        Self::GetUnitForm,
        Self::GetUdtForm,
        Self::GetRecipeForm,
        Self::GetVersionForm,
        Self::GetParameterForm,
        Self::GetApprovalPreview,
        Self::GetUserForm,
        Self::GetLoginForm,
        Self::GetOwnPasswordForm,
    ];

    /// Return the name of the use case.
    ///
    /// Both export variants give `ExportVersion`. `ListDeletedRecipes` gives `ListRecipes`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CreateMachine => "CreateMachine",
            Self::RenameMachine => "RenameMachine",
            Self::ArchiveMachine => "ArchiveMachine",
            Self::AddUnit => "AddUnit",
            Self::RenameUnit => "RenameUnit",
            Self::ArchiveUnit => "ArchiveUnit",
            Self::DefineUdt => "DefineUdt",
            Self::CreateRecipe => "CreateRecipe",
            Self::RenameRecipe => "RenameRecipe",
            Self::DeleteRecipe => "DeleteRecipe",
            Self::RestoreRecipe => "RestoreRecipe",
            Self::CreateVersion => "CreateVersion",
            Self::DiscardDraft => "DiscardDraft",
            Self::ApproveVersion => "ApproveVersion",
            Self::RetireVersion => "RetireVersion",
            Self::AddParameter => "AddParameter",
            Self::UpdateParameterDefinition => "UpdateParameterDefinition",
            Self::SetParameterValue => "SetParameterValue",
            Self::ReplaceLimits => "ReplaceLimits",
            Self::UpsertTranslation => "UpsertTranslation",
            Self::RemoveTranslation => "RemoveTranslation",
            Self::RemoveParameter => "RemoveParameter",
            Self::RebindTypeRevision => "RebindTypeRevision",
            Self::Login => "Login",
            Self::Logout => "Logout",
            Self::CreateUser => "CreateUser",
            Self::SetUserRole => "SetUserRole",
            Self::DeactivateUser => "DeactivateUser",
            Self::ActivateUser => "ActivateUser",
            Self::ResetPassword => "ResetPassword",
            Self::ChangeOwnPassword => "ChangeOwnPassword",
            Self::UnlockLogin => "UnlockLogin",
            Self::ExportApprovedVersion | Self::ExportVersionByNumber => "ExportVersion",
            Self::ListMachines => "ListMachines",
            Self::GetMachine => "GetMachine",
            Self::GetUnitTree => "GetUnitTree",
            Self::GetUnit => "GetUnit",
            Self::ListTypes => "ListTypes",
            Self::GetTypeTree => "GetTypeTree",
            Self::GetTypeUsage => "GetTypeUsage",
            Self::ListRecipes | Self::ListDeletedRecipes => "ListRecipes",
            Self::GetRecipe => "GetRecipe",
            Self::ListVersions => "ListVersions",
            Self::GetVersion => "GetVersion",
            Self::DiffVersions => "DiffVersions",
            Self::GetRecipeHistory => "GetRecipeHistory",
            Self::ListParameters => "ListParameters",
            Self::GetParameter => "GetParameter",
            Self::GetLimits => "GetLimits",
            Self::GetTranslations => "GetTranslations",
            Self::ListUsers => "ListUsers",
            Self::GetUser => "GetUser",
            Self::ListAuditEntries => "ListAuditEntries",
            Self::GetAuditEntry => "GetAuditEntry",
            Self::ListSecurityEvents => "ListSecurityEvents",
            Self::GetOwnProfile => "GetOwnProfile",
            Self::GetMachineForm => "GetMachineForm",
            Self::GetUnitForm => "GetUnitForm",
            Self::GetUdtForm => "GetUdtForm",
            Self::GetRecipeForm => "GetRecipeForm",
            Self::GetVersionForm => "GetVersionForm",
            Self::GetParameterForm => "GetParameterForm",
            Self::GetApprovalPreview => "GetApprovalPreview",
            Self::GetUserForm => "GetUserForm",
            Self::GetLoginForm => "GetLoginForm",
            Self::GetOwnPasswordForm => "GetOwnPasswordForm",
        }
    }

    /// Return true when the operation is a command and false when it is a query.
    #[must_use]
    pub const fn is_command(self) -> bool {
        match self {
            Self::CreateMachine
            | Self::RenameMachine
            | Self::ArchiveMachine
            | Self::AddUnit
            | Self::RenameUnit
            | Self::ArchiveUnit
            | Self::DefineUdt
            | Self::CreateRecipe
            | Self::RenameRecipe
            | Self::DeleteRecipe
            | Self::RestoreRecipe
            | Self::CreateVersion
            | Self::DiscardDraft
            | Self::ApproveVersion
            | Self::RetireVersion
            | Self::AddParameter
            | Self::UpdateParameterDefinition
            | Self::SetParameterValue
            | Self::ReplaceLimits
            | Self::UpsertTranslation
            | Self::RemoveTranslation
            | Self::RemoveParameter
            | Self::RebindTypeRevision
            | Self::Login
            | Self::Logout
            | Self::CreateUser
            | Self::SetUserRole
            | Self::DeactivateUser
            | Self::ActivateUser
            | Self::ResetPassword
            | Self::ChangeOwnPassword
            | Self::UnlockLogin
            | Self::ExportApprovedVersion
            | Self::ExportVersionByNumber => true,
            Self::ListMachines
            | Self::GetMachine
            | Self::GetUnitTree
            | Self::GetUnit
            | Self::ListTypes
            | Self::GetTypeTree
            | Self::GetTypeUsage
            | Self::ListRecipes
            | Self::ListDeletedRecipes
            | Self::GetRecipe
            | Self::ListVersions
            | Self::GetVersion
            | Self::DiffVersions
            | Self::GetRecipeHistory
            | Self::ListParameters
            | Self::GetParameter
            | Self::GetLimits
            | Self::GetTranslations
            | Self::ListUsers
            | Self::GetUser
            | Self::ListAuditEntries
            | Self::GetAuditEntry
            | Self::ListSecurityEvents
            | Self::GetOwnProfile
            | Self::GetMachineForm
            | Self::GetUnitForm
            | Self::GetUdtForm
            | Self::GetRecipeForm
            | Self::GetVersionForm
            | Self::GetParameterForm
            | Self::GetApprovalPreview
            | Self::GetUserForm
            | Self::GetLoginForm
            | Self::GetOwnPasswordForm => false,
        }
    }

    /// Return true when the operation still runs in read-only mode (Assumption A24).
    ///
    /// Every query runs. Of the commands, only `Login`, `Logout`, and the two exports run.
    #[must_use]
    pub const fn runs_in_read_only(self) -> bool {
        match self {
            Self::Login
            | Self::Logout
            | Self::ExportApprovedVersion
            | Self::ExportVersionByNumber => true,
            other => !other.is_command(),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests;
