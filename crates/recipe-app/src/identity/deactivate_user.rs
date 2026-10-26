//! The use case `DeactivateUser` (design D8, ADR-0096).

use recipe_domain::audit::RowAction;
use recipe_domain::identity::{AdminChange, Login};

use super::admin_change::run_admin_change;
use super::{Authenticated, IdentityContext};
use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::UseCase;
use crate::error::AppError;

/// Deactivates a user.
///
/// The use case solves one problem: a deactivated user must lose every session at once, and the
/// system must keep one active Administrator. The row stays, because audit entries refer to it.
/// It runs in one `READ COMMITTED` transaction and takes the guard lock of the last Administrator
/// first. It has no retry.
#[derive(Clone)]
pub struct DeactivateUser {
    ctx: IdentityContext,
}

impl UseCase for DeactivateUser {
    const OPERATION: Operation = Operation::DeactivateUser;
}

impl DeactivateUser {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Deactivate the user with `login` and end every session of that user.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the login is unknown.
    /// Returns `UserAlreadyInactive` (`E-IDN-010`) for an inactive user. The use case writes nothing then.
    /// Returns `LastAdministrator` (`E-IDN-002`) when the change removes the last Administrator.
    /// Returns `AppError::Storage` with `E-DB-001` after a deadlock or a lock timeout.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` for another database failure.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        login: &Login,
    ) -> Result<(), AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        run_admin_change(
            &self.ctx,
            auth,
            meta,
            login,
            AdminChange::Deactivate,
            RowAction::UserDeactivated,
        )
        .await
    }
}

#[cfg(test)]
mod tests;
