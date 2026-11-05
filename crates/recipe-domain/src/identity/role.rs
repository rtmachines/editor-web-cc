//! Permissions and roles (`api.md` sections 3.1 to 3.3, ADR-0013).
//!
//! A role maps to a fixed set of permissions. The conditions "(c)" of the
//! matrix are rules of the use cases and not part of the permission set.

use std::fmt;

/// One right that a role can hold (`api.md` section 3.1).
///
/// A use case checks one permission as its first statement (ADR-0013).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    /// View recipes, versions, parameters, machines, units, UDTs, and type usage.
    RecipeView,
    /// Create a recipe.
    RecipeCreate,
    /// Rename a recipe.
    RecipeEdit,
    /// Delete a recipe softly. The use case adds the condition of the matrix.
    RecipeDelete,
    /// Restore a deleted recipe and list deleted recipes.
    RecipeRestore,
    /// Create a new draft version.
    VersionCreate,
    /// Approve a version. The use case adds the condition of the matrix.
    VersionApprove,
    /// Retire a version without replacement.
    VersionRetire,
    /// Discard a draft.
    VersionDiscard,
    /// Export the approved version.
    VersionExport,
    /// Export a retired version by number.
    VersionExportHistory,
    /// Add, edit, and delete the parameters, limits, and translations of a draft.
    ParameterEdit,
    /// Create, rename, and archive a machine.
    MachineManage,
    /// Add, rename, and archive a unit.
    UnitManage,
    /// Define a UDT revision.
    UdtManage,
    /// Manage users and their roles.
    UserManage,
    /// Unlock a login.
    LoginUnlock,
    /// View the full audit log and the security events.
    AuditView,
    /// View the audit entries of recipes, versions, and parameters.
    AuditViewRecipes,
}

impl Permission {
    /// Every permission, in the order of `api.md` section 3.1.
    pub const ALL: [Self; 19] = [
        Self::RecipeView,
        Self::RecipeCreate,
        Self::RecipeEdit,
        Self::RecipeDelete,
        Self::RecipeRestore,
        Self::VersionCreate,
        Self::VersionApprove,
        Self::VersionRetire,
        Self::VersionDiscard,
        Self::VersionExport,
        Self::VersionExportHistory,
        Self::ParameterEdit,
        Self::MachineManage,
        Self::UnitManage,
        Self::UdtManage,
        Self::UserManage,
        Self::LoginUnlock,
        Self::AuditView,
        Self::AuditViewRecipes,
    ];

    /// Return the variant name.
    const fn name(self) -> &'static str {
        match self {
            Self::RecipeView => "RecipeView",
            Self::RecipeCreate => "RecipeCreate",
            Self::RecipeEdit => "RecipeEdit",
            Self::RecipeDelete => "RecipeDelete",
            Self::RecipeRestore => "RecipeRestore",
            Self::VersionCreate => "VersionCreate",
            Self::VersionApprove => "VersionApprove",
            Self::VersionRetire => "VersionRetire",
            Self::VersionDiscard => "VersionDiscard",
            Self::VersionExport => "VersionExport",
            Self::VersionExportHistory => "VersionExportHistory",
            Self::ParameterEdit => "ParameterEdit",
            Self::MachineManage => "MachineManage",
            Self::UnitManage => "UnitManage",
            Self::UdtManage => "UdtManage",
            Self::UserManage => "UserManage",
            Self::LoginUnlock => "LoginUnlock",
            Self::AuditView => "AuditView",
            Self::AuditViewRecipes => "AuditViewRecipes",
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Permissions of the `ProcessEngineer` (`api.md` section 3.3).
const PROCESS_ENGINEER_PERMISSIONS: [Permission; 14] = [
    Permission::RecipeView,
    Permission::VersionExport,
    Permission::VersionExportHistory,
    Permission::RecipeCreate,
    Permission::RecipeEdit,
    Permission::RecipeDelete,
    Permission::VersionCreate,
    Permission::ParameterEdit,
    Permission::VersionApprove,
    Permission::VersionDiscard,
    Permission::MachineManage,
    Permission::UnitManage,
    Permission::UdtManage,
    Permission::AuditViewRecipes,
];

/// Permissions of the `Operator` (`api.md` section 3.3).
const OPERATOR_PERMISSIONS: [Permission; 2] = [Permission::RecipeView, Permission::VersionExport];

/// Permissions of the `ExportClient` (ADR-0109 (a), (b)).
const EXPORT_CLIENT_PERMISSIONS: [Permission; 1] = [Permission::VersionExport];

/// Role of a user. A role maps to a fixed set of permissions (ADR-0013).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Holds every permission.
    Administrator,
    /// Edits recipes and the plant model, and approves versions.
    ProcessEngineer,
    /// Views and exports the approved recipes.
    Operator,
    /// Technical account of a transfer client. It exports the approved version
    /// and has no person behind it (ADR-0109 (a)).
    ExportClient,
}

/// Reason why a text is not a role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoleReason {
    /// The text is not one of the SQL values of a role.
    Unknown,
}

impl fmt::Display for RoleReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => {
                f.write_str("a role is administrator, process_engineer, operator, or export_client")
            }
        }
    }
}

impl Role {
    /// Every person role. It leaves out [`Role::ExportClient`], so a role list of
    /// the UI offers no technical account (ADR-0109, `design.md` D1).
    pub const ALL: [Self; 3] = [Self::Administrator, Self::ProcessEngineer, Self::Operator];

    /// Parse the SQL value of a role (`data-model.md` section 1.2).
    ///
    /// # Errors
    ///
    /// Returns [`RoleReason::Unknown`] when the text is not a SQL value of a role.
    pub fn parse(text: &str) -> Result<Self, RoleReason> {
        match text {
            "administrator" => Ok(Self::Administrator),
            "process_engineer" => Ok(Self::ProcessEngineer),
            "operator" => Ok(Self::Operator),
            "export_client" => Ok(Self::ExportClient),
            _ => Err(RoleReason::Unknown),
        }
    }

    /// Return the SQL value of the role (`app_user.role`).
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Administrator => "administrator",
            Self::ProcessEngineer => "process_engineer",
            Self::Operator => "operator",
            Self::ExportClient => "export_client",
        }
    }

    /// Return the permissions of the role. The result is fixed (`api.md` section 3.3).
    #[must_use]
    pub const fn permissions(self) -> &'static [Permission] {
        match self {
            Self::Administrator => &Permission::ALL,
            Self::ProcessEngineer => &PROCESS_ENGINEER_PERMISSIONS,
            Self::Operator => &OPERATOR_PERMISSIONS,
            Self::ExportClient => &EXPORT_CLIENT_PERMISSIONS,
        }
    }

    /// Return true when a person holds the role. An `ExportClient` is a
    /// technical account and never signs in with a session (ADR-0109 (a)).
    #[must_use]
    pub const fn is_person(self) -> bool {
        !matches!(self, Self::ExportClient)
    }

    /// Return true when the role holds the permission.
    #[must_use]
    pub fn has(self, permission: Permission) -> bool {
        self.permissions().contains(&permission)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Administrator => "Administrator",
            Self::ProcessEngineer => "ProcessEngineer",
            Self::Operator => "Operator",
            Self::ExportClient => "ExportClient",
        })
    }
}

#[cfg(test)]
mod tests;
