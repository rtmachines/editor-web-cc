//! The use case `SetUserRole` (design D8, ADR-0035 c).

use recipe_domain::audit::RowAction;
use recipe_domain::identity::{AdminChange, Login, Role};

use super::admin_change::run_admin_change;
use super::{Authenticated, IdentityContext};
use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::UseCase;
use crate::error::AppError;

/// The parsed request of `SetUserRole`.
#[derive(Clone, Debug)]
pub struct SetUserRoleInput {
    /// The login of the target user.
    pub login: Login,
    /// The role that the target gets.
    pub role: Role,
}

/// Sets the role of a user.
///
/// The use case solves one problem: a role change must end every session of the user, because a
/// session must never keep a role that the row no longer holds. It also must never remove the
/// last active Administrator. It runs `SERIALIZABLE` and once more after a serialization failure.
#[derive(Clone)]
pub struct SetUserRole {
    ctx: IdentityContext,
}

impl UseCase for SetUserRole {
    const OPERATION: Operation = Operation::SetUserRole;
}

impl SetUserRole {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Set the role and end every session of the target.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the login is unknown.
    /// Returns `LastAdministrator` (`E-IDN-002`) when the change removes the last Administrator.
    /// Returns `AppError::Storage` with `E-DB-001` after a second serialization failure or a deadlock.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` for another database failure.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        input: SetUserRoleInput,
    ) -> Result<(), AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        let SetUserRoleInput { login, role } = input;
        run_admin_change(
            &self.ctx,
            auth,
            meta,
            &login,
            AdminChange::SetRole(role),
            RowAction::UserRoleSet,
        )
        .await
    }
}

#[cfg(test)]
mod tests;
