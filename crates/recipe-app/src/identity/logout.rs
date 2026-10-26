//! The use case `Logout` (`domain-model.md` section 5.3, `data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, StateAction};
use serde_json::json;

use super::{Authenticated, IdentityContext};
use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::{UseCase, audit_context};
use crate::error::AppError;
use crate::ports::{Isolation, NonEmpty};

/// Ends the session of the current request.
///
/// The use case solves one problem: a logout must delete the session and leave the entry
/// `UserLoggedOut` in one transaction, so a failed commit keeps both unchanged.
#[derive(Clone)]
pub struct Logout {
    ctx: IdentityContext,
}

impl UseCase for Logout {
    const OPERATION: Operation = Operation::Logout;
}

impl Logout {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Delete the session of `auth` and write `UserLoggedOut` with its `session_ref`.
    ///
    /// `recipe-web` clears the cookie after the call.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the permission check fails, which no role causes today.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(&self, auth: &Authenticated, meta: &RequestMeta) -> Result<(), AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        let user = auth.user();
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadCommitted).await?;
        tx.sessions().delete(auth.session_id()).await?;
        let entry = AuditEntry::state(
            StateAction::UserLoggedOut,
            AuditTarget::app_user(user.user()),
            json!({ "session_ref": auth.session_ref().as_str() }),
            audit_context(user, meta),
        );
        self.ctx.app.uow.commit(tx, NonEmpty::new(entry)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
