//! The requirement of an operation (`api.md` section 3.3, `domain-model.md` section 5.5).

use recipe_domain::identity::{Permission, Role};

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
    /// Any logged-in person passes.
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
            // An `ExportClient` has no session and no login (ADR-0109 (a), `api.md` section 3.3).
            Self::Authenticated | Self::NoSession => role.is_person(),
        }
    }
}
