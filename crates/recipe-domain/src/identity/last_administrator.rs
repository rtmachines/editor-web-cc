//! The last-administrator rule (`domain-model.md` 2.3, design D8).

use super::{IdentityError, Login, Role};

/// The state of the target user that the use case read under its row lock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserStanding {
    /// The login of the target, for the error message.
    pub login: Login,
    /// The role of the target before the change.
    pub role: Role,
    /// Whether the target is active before the change.
    pub is_active: bool,
}

/// A change that can remove an administrator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdminChange {
    /// `SetUserRole` gives the target this role.
    SetRole(Role),
    /// `DeactivateUser` makes the target inactive.
    Deactivate,
}

/// Refuse a change that would leave the system without an active administrator.
///
/// It solves one problem: `SetUserRole` and `DeactivateUser` share one rule. The use case passes
/// the target row that it locked and the number of active administrators that it counted after
/// it took the guard lock of the last administrator (ADR-0096). That number includes the target
/// when the target is active.
///
/// # Errors
///
/// Returns [`IdentityError::LastAdministrator`] when the target is an active administrator, the
/// change removes the role or the active state, and no other active administrator remains.
pub fn last_administrator_guard(
    target: &UserStanding,
    change: AdminChange,
    active_administrators: u32,
) -> Result<(), IdentityError> {
    let is_active_administrator = target.is_active && target.role == Role::Administrator;
    let removes_administrator = match change {
        AdminChange::SetRole(role) => role != Role::Administrator,
        AdminChange::Deactivate => true,
    };
    if is_active_administrator && removes_administrator && active_administrators <= 1 {
        return Err(IdentityError::LastAdministrator {
            login: target.login.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
