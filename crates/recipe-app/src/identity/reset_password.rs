//! The use case `ResetPassword` (design D8, `data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::identity::{Login, PasswordHash};

use super::one_time::hashed_one_time_password;
use super::{Authenticated, IdentityContext, IssuedPassword};
use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::{UseCase, audit_context};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Locked, NonEmpty, UserRow};

/// Resets the password of a user to a one-time password.
///
/// The use case solves one problem: an Administrator must regain access for a user who lost the
/// password, and every old session of that user must end in the same transaction. The epoch rises
/// by 1, so a session that a concurrent login creates meanwhile fails its second check.
#[derive(Clone)]
pub struct ResetPassword {
    ctx: IdentityContext,
}

impl UseCase for ResetPassword {
    const OPERATION: Operation = Operation::ResetPassword;
}

impl ResetPassword {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Reset the password of `login` and return the one-time password.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the login is unknown.
    /// Returns `Timeout` (`E-WEB-009`) when no Argon2 permit is free in time.
    /// Returns `TaskFailed` (`E-WEB-011`) when the hash task or the read of random bytes fails.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        login: &Login,
    ) -> Result<IssuedPassword, AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        self.require_known(login).await?;
        let (password, hash) = hashed_one_time_password(&self.ctx).await?;
        self.replace(auth, meta, login, &hash).await?;
        Ok(IssuedPassword::new(login.clone(), password))
    }

    /// Read the login in a read-only transaction, so an unknown login costs no hash.
    async fn require_known(&self, login: &Login) -> Result<(), AppError> {
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadOnly).await?;
        tx.users()
            .find_by_login(login)
            .await?
            .map(|_row: UserRow| ())
            .ok_or_else(|| not_found(login))
    }

    /// Lock the user, replace the hash, end every session, and write `PasswordReset`.
    async fn replace(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        login: &Login,
        hash: &PasswordHash,
    ) -> Result<(), AppError> {
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadCommitted).await?;
        let Locked {
            state: locked,
            before,
        } = tx
            .users()
            .lock_by_login(login)
            .await?
            .ok_or_else(|| not_found(login))?;
        let after = tx.users().reset_password(locked.id, hash).await?;
        tx.sessions().delete_all_of_user(locked.id).await?;
        let entry = AuditEntry::row(
            RowAction::PasswordReset,
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

/// Build the error `E-WEB-005` for a login.
fn not_found(login: &Login) -> AppError {
    AppError::NotFound {
        entity: Missing::User(login.clone()),
    }
}

#[cfg(test)]
mod tests;
