//! The use case `CreateUser` (design D8, `data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::identity::{IdentityError, Login, Role, UserId};

use super::issue::random_uuid;
use super::one_time::hashed_one_time_password;
use super::{Authenticated, IdentityContext, IssuedPassword};
use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::{UseCase, audit_context};
use crate::error::{APP_USER_LOGIN_UNIQUE, AppError, map_storage};
use crate::ports::{Isolation, NewUser, NonEmpty};

/// The parsed request of `CreateUser`.
#[derive(Clone, Debug)]
pub struct CreateUserInput {
    /// The login of the new user.
    pub login: Login,
    /// The role of the new user.
    pub role: Role,
}

/// Creates a user with a one-time password.
///
/// The use case solves one problem: an Administrator must give a new user a first password that
/// no log line and no audit entry holds. It hashes outside the transaction and returns the
/// password once. The first login of the user forces a change of the password.
#[derive(Clone)]
pub struct CreateUser {
    ctx: IdentityContext,
}

impl UseCase for CreateUser {
    const OPERATION: Operation = Operation::CreateUser;
}

impl CreateUser {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Create the user and return the one-time password.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `DuplicateLogin` (`E-IDN-001`) when the login exists. A read before the hash finds
    /// the usual case. The unique constraint finds the race.
    /// Returns `Timeout` (`E-WEB-009`) when no Argon2 permit is free in time.
    /// Returns `TaskFailed` (`E-WEB-011`) when the hash task or the read of random bytes fails.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        input: CreateUserInput,
    ) -> Result<IssuedPassword, AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        let CreateUserInput { login, role } = input;
        self.refuse_known_login(&login).await?;
        let (password, hash) = hashed_one_time_password(&self.ctx).await?;
        let user = NewUser {
            id: UserId::from(random_uuid()?),
            login: login.clone(),
            password_hash: hash,
            role,
            must_change_password: true,
        };
        self.insert(auth, meta, &user).await?;
        Ok(IssuedPassword::new(login, password))
    }

    /// Read the login in a read-only transaction. This saves the hash in the usual duplicate case.
    async fn refuse_known_login(&self, login: &Login) -> Result<(), AppError> {
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadOnly).await?;
        if tx.users().find_by_login(login).await?.is_some() {
            return Err(duplicate(login));
        }
        Ok(())
    }

    /// Insert the user and write `UserCreated` in one transaction.
    async fn insert(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        user: &NewUser,
    ) -> Result<(), AppError> {
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadCommitted).await?;
        let after = tx.users().insert(user).await.map_err(|error| {
            map_storage(error, |constraint| {
                (constraint == APP_USER_LOGIN_UNIQUE).then(|| duplicate(&user.login))
            })
        })?;
        let entry = AuditEntry::row(
            RowAction::UserCreated,
            AuditTarget::app_user(user.id),
            RowChange::Create {
                after: after.into_value(),
            },
            audit_context(auth.user(), meta),
        );
        self.ctx.app.uow.commit(tx, NonEmpty::new(entry)).await?;
        Ok(())
    }
}

/// Build the error `E-IDN-001` for a login.
fn duplicate(login: &Login) -> AppError {
    IdentityError::DuplicateLogin {
        login: login.clone(),
    }
    .into()
}

#[cfg(test)]
mod tests;
