//! The requirement of an operation (`api.md` section 3.3, `domain-model.md` section 5.5).

use recipe_domain::identity::{Permission, Role};

use super::Operation;

/// What a caller must hold to run an operation.
///
/// The type solves one problem: the check of a permission, of "any of two", and of "logged in"
/// must live in one place, so no use case invents its own rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement {
    /// The role must hold this permission.
    Permission(Permission),
    /// The role must hold at least one of the two permissions.
    AnyOf(Permission, Permission),
    /// Any logged-in user passes.
    Authenticated,
    /// The operation needs no session.
    NoSession,
}

impl Requirement {
    /// Return true when a user with `role` meets the requirement.
    #[must_use]
    pub fn allows(self, role: Role) -> bool {
        match self {
            Self::Permission(permission) => role.has(permission),
            Self::AnyOf(first, second) => role.has(first) || role.has(second),
            Self::Authenticated | Self::NoSession => true,
        }
    }
}

impl Operation {
    /// Return the requirement of the operation.
    ///
    /// The match has no wildcard arm. A new variant without a row does not compile.
    #[must_use]
    pub const fn requirement(self) -> Requirement {
        match self {
            Self::CreateMachine
            | Self::RenameMachine
            | Self::ArchiveMachine
            | Self::GetMachineForm => Requirement::Permission(Permission::MachineManage),
            Self::AddUnit | Self::RenameUnit | Self::ArchiveUnit | Self::GetUnitForm => {
                Requirement::Permission(Permission::UnitManage)
            }
            Self::DefineUdt | Self::GetUdtForm => Requirement::Permission(Permission::UdtManage),
            Self::CreateRecipe => Requirement::Permission(Permission::RecipeCreate),
            // The two purposes need two permissions (ADR-0046 (c)). The query checks the one of
            // its purpose, so this requirement only names who may open the form at all.
            Self::GetRecipeForm => {
                Requirement::AnyOf(Permission::RecipeCreate, Permission::RecipeEdit)
            }
            Self::RenameRecipe => Requirement::Permission(Permission::RecipeEdit),
            Self::DeleteRecipe => Requirement::Permission(Permission::RecipeDelete),
            Self::RestoreRecipe | Self::ListDeletedRecipes => {
                Requirement::Permission(Permission::RecipeRestore)
            }
            Self::CreateVersion | Self::GetVersionForm => {
                Requirement::Permission(Permission::VersionCreate)
            }
            Self::AddParameter
            | Self::UpdateParameterDefinition
            | Self::SetParameterValue
            | Self::ReplaceLimits
            | Self::UpsertTranslation
            | Self::RemoveTranslation
            | Self::RemoveParameter
            | Self::RebindTypeRevision
            | Self::GetParameterForm => Requirement::Permission(Permission::ParameterEdit),
            Self::ApproveVersion | Self::GetApprovalPreview => {
                Requirement::Permission(Permission::VersionApprove)
            }
            Self::RetireVersion => Requirement::Permission(Permission::VersionRetire),
            Self::DiscardDraft => Requirement::Permission(Permission::VersionDiscard),
            Self::ExportApprovedVersion => Requirement::Permission(Permission::VersionExport),
            Self::ExportVersionByNumber => {
                Requirement::Permission(Permission::VersionExportHistory)
            }
            Self::CreateUser
            | Self::SetUserRole
            | Self::DeactivateUser
            | Self::ActivateUser
            | Self::ResetPassword
            | Self::ListUsers
            | Self::GetUser
            | Self::GetUserForm => Requirement::Permission(Permission::UserManage),
            Self::UnlockLogin => Requirement::Permission(Permission::LoginUnlock),
            Self::ListAuditEntries | Self::GetAuditEntry => {
                Requirement::AnyOf(Permission::AuditView, Permission::AuditViewRecipes)
            }
            Self::ListSecurityEvents => Requirement::Permission(Permission::AuditView),
            Self::ListMachines
            | Self::GetMachine
            | Self::GetUnitTree
            | Self::GetUnit
            | Self::ListTypes
            | Self::GetTypeTree
            | Self::GetTypeUsage
            | Self::ListRecipes
            | Self::GetRecipe
            | Self::ListVersions
            | Self::GetVersion
            | Self::DiffVersions
            | Self::GetRecipeHistory
            | Self::ListParameters
            | Self::GetParameter
            | Self::GetLimits
            | Self::GetTranslations => Requirement::Permission(Permission::RecipeView),
            Self::Login | Self::GetLoginForm => Requirement::NoSession,
            Self::Logout
            | Self::ChangeOwnPassword
            | Self::GetOwnPasswordForm
            | Self::GetOwnProfile => Requirement::Authenticated,
        }
    }
}
