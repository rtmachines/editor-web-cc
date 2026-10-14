//! The use case `ActivateUser` (design D8, ADR-0035 a).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::identity::{IdentityError, Login};

use super::{Authenticated, IdentityContext};
use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::{UseCase, audit_context};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Locked, NonEmpty};

/// Activates an inactive user.
///
/// The use case solves one problem: an Administrator must give a deactivated user the access back.
/// The row keeps its epoch, so no old session returns. The user logs in again.
#[derive(Clone)]
pub struct ActivateUser {
    ctx: IdentityContext,
}

impl UseCase for ActivateUser {
    const OPERATION: Operation = Operation::ActivateUser;
}

impl ActivateUser {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Activate the user with `login`.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the login is unknown.
    /// Returns `UserAlreadyActive` (`E-IDN-009`) for an active user. The use case writes nothing then.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
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
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadCommitted).await?;
        let Locked {
            state: locked,
            before,
        } = tx
            .users()
            .lock_by_login(login)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::User(login.clone()),
            })?;
        if locked.is_active {
            return Err(IdentityError::UserAlreadyActive {
                login: login.clone(),
            }
            .into());
        }
        let after = tx.users().activate(locked.id).await?;
        let entry = AuditEntry::row(
            RowAction::UserActivated,
            AuditTarget::app_user(locked.id),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            audit_context(auth.user(), meta),
        );
        self.ctx.app.uow.commit(tx, NonEmpty::new(entry)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
