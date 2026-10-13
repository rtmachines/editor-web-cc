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

/// Role of a user. A role maps to a fixed set of permissions (ADR-0013).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Holds every permission.
    Administrator,
    /// Edits recipes and the plant model, and approves versions.
    ProcessEngineer,
    /// Views and exports the approved recipes.
    Operator,
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
            Self::Unknown => f.write_str("a role is administrator, process_engineer, or operator"),
        }
    }
}

impl Role {
    /// Every role.
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
        }
    }

    /// Return the permissions of the role. The result is fixed (`api.md` section 3.3).
    #[must_use]
    pub const fn permissions(self) -> &'static [Permission] {
        match self {
            Self::Administrator => &Permission::ALL,
            Self::ProcessEngineer => &PROCESS_ENGINEER_PERMISSIONS,
            Self::Operator => &OPERATOR_PERMISSIONS,
        }
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn set(items: &[Permission]) -> BTreeSet<Permission> {
        items.iter().copied().collect()
    }

    #[test]
    fn administrator_holds_every_permission() {
        assert_eq!(Role::Administrator.permissions().len(), 19);
        assert_eq!(
            set(Role::Administrator.permissions()),
            set(&Permission::ALL)
        );
    }

    #[test]
    fn process_engineer_holds_the_fourteen_permissions_of_the_matrix() {
        use Permission::*;
        let expected = set(&[
            RecipeView,
            VersionExport,
            VersionExportHistory,
            RecipeCreate,
            RecipeEdit,
            RecipeDelete,
            VersionCreate,
            ParameterEdit,
            VersionApprove,
            VersionDiscard,
            MachineManage,
            UnitManage,
            UdtManage,
            AuditViewRecipes,
        ]);
        assert_eq!(expected.len(), 14);
        assert_eq!(Role::ProcessEngineer.permissions().len(), 14);
        assert_eq!(set(Role::ProcessEngineer.permissions()), expected);
    }

    #[test]
    fn operator_holds_view_and_export_only() {
        assert_eq!(
            set(Role::Operator.permissions()),
            set(&[Permission::RecipeView, Permission::VersionExport])
        );
        assert_eq!(Role::Operator.permissions().len(), 2);
    }

    #[test]
    fn has_agrees_with_permissions_for_every_pair() {
        for role in Role::ALL {
            for permission in Permission::ALL {
                assert_eq!(
                    role.has(permission),
                    role.permissions().contains(&permission),
                    "{role} / {permission}"
                );
            }
        }
    }

    #[test]
    fn permission_all_lists_nineteen_distinct_variants() {
        assert_eq!(Permission::ALL.len(), 19);
        assert_eq!(set(&Permission::ALL).len(), 19);
    }

    #[test]
    fn permission_displays_its_variant_name() {
        assert_eq!(
            Permission::VersionExportHistory.to_string(),
            "VersionExportHistory"
        );
        assert_eq!(Permission::AuditViewRecipes.to_string(), "AuditViewRecipes");
    }

    #[test]
    fn role_displays_its_variant_name() {
        assert_eq!(Role::ProcessEngineer.to_string(), "ProcessEngineer");
    }

    #[test]
    fn process_engineer_text_parses() {
        assert_eq!(Role::parse("process_engineer"), Ok(Role::ProcessEngineer));
    }

    #[test]
    fn variant_name_admin_and_empty_text_give_a_reason() {
        assert_eq!(Role::parse("ProcessEngineer"), Err(RoleReason::Unknown));
        assert_eq!(Role::parse("admin"), Err(RoleReason::Unknown));
        assert_eq!(Role::parse(""), Err(RoleReason::Unknown));
    }

    #[test]
    fn as_sql_and_parse_round_trip() {
        for role in Role::ALL {
            assert_eq!(Role::parse(role.as_sql()), Ok(role));
        }
        assert_eq!(Role::Administrator.as_sql(), "administrator");
        assert_eq!(Role::ProcessEngineer.as_sql(), "process_engineer");
        assert_eq!(Role::Operator.as_sql(), "operator");
    }

    #[test]
    fn role_reason_displays_a_sentence() {
        assert!(!RoleReason::Unknown.to_string().is_empty());
    }
}
