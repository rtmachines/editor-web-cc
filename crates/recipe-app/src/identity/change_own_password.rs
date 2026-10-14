//! The use case `ChangeOwnPassword` (design D7, ADR-0035 b and m).

use std::net::IpAddr;

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::identity::{IdentityError, NewPassword, Password, PasswordHash, TypedLogin};

use super::issue::IssuedSession;
use super::throttle::{Gate, Requester, begin_attempt, reject_throttled};
use super::{Authenticated, IdentityContext};
use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::{UseCase, audit_context};
use crate::error::{AppError, StorageError};
use crate::ports::{AttemptId, Isolation, Locked, NonEmpty, UserRow};

/// The parsed request of `ChangeOwnPassword`.
///
/// `Debug` shows no secret, because `Password` redacts itself.
#[derive(Clone, Debug)]
pub struct ChangeOwnPasswordInput {
    /// The canonical address of the client.
    pub client_ip: IpAddr,
    /// The current password of the form field.
    pub current_password: Password,
    /// The new password of the form field. The use case checks the policy of A23.
    pub new_password: Password,
}

/// Changes the password of the current user.
///
/// The use case solves one problem: a password change must not let a guessed session try the
/// current password without a limit, and it must not overwrite a reset that an Administrator
/// committed meanwhile. It applies the login throttle, checks and hashes under one permit and
/// outside any transaction, and checks the credentials again under the lock (ADR-0035 b and m).
#[derive(Clone)]
pub struct ChangeOwnPassword {
    ctx: IdentityContext,
}

impl UseCase for ChangeOwnPassword {
    const OPERATION: Operation = Operation::ChangeOwnPassword;
}

impl ChangeOwnPassword {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Change the password and replace every session of the user with a new one.
    ///
    /// # Errors
    ///
    /// Returns `PasswordPolicy` (`E-IDN-006`) when the new password breaks the policy. No attempt
    /// row and no Argon2 work exist then.
    /// Returns `Timeout` (`E-WEB-009`) when no Argon2 permit is free in time.
    /// Returns `LoginThrottled` (`E-IDN-005`) inside a delay or when the address budget is used up.
    /// Returns `InvalidCurrentPassword` (`E-IDN-008`) when the current password is wrong.
    /// Returns `Unauthenticated` (`E-WEB-004`) when the user, the epoch, or the hash changed after
    /// the check.
    /// Returns `TaskFailed` (`E-WEB-011`) when the hash task or the read of random bytes fails.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        input: ChangeOwnPasswordInput,
    ) -> Result<IssuedSession, AppError> {
        let user = auth.user();
        authorize(Self::OPERATION, user, meta, self.ctx.app.security.as_ref()).await?;
        let ChangeOwnPasswordInput {
            client_ip,
            current_password,
            new_password,
        } = input;
        let new_password =
            NewPassword::parse(new_password.as_str()).map_err(IdentityError::from)?;
        let login = TypedLogin::parse(user.login().as_str())
            .map_err(|_unreachable| AppError::Unauthenticated)?;
        let requester = Requester::of(meta, client_ip);
        let permit = self.ctx.gate.acquire().await?;
        let attempt = match begin_attempt(&self.ctx, &login, client_ip).await? {
            Gate::Open(attempt) => attempt,
            Gate::Throttled { retry_after } => {
                drop(permit);
                return reject_throttled(&self.ctx, &requester, &login, retry_after).await;
            }
        };
        let checked = self.read_own_row(auth).await?;
        let (matched, permit) = permit
            .verify(current_password, Some(checked.password_hash.clone()))
            .await?;
        if !matched {
            drop(permit);
            return Err(IdentityError::InvalidCurrentPassword.into());
        }
        let (new_hash, permit) = permit.hash(new_password).await?;
        drop(permit);
        self.replace(auth, meta, &checked, attempt, &new_hash).await
    }

    /// Read the own user row in a read-only transaction and end that transaction.
    async fn read_own_row(&self, auth: &Authenticated) -> Result<UserRow, AppError> {
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadOnly).await?;
        tx.users()
            .find_by_login(auth.user().login())
            .await?
            .ok_or(AppError::Unauthenticated)
    }

    /// Lock the user, check the credentials again, and write the change (steps 5 and 6).
    async fn replace(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        checked: &UserRow,
        attempt: AttemptId,
        new_hash: &PasswordHash,
    ) -> Result<IssuedSession, AppError> {
        let issued = IssuedSession::generate()?;
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadCommitted).await?;
        let Locked {
            state: locked,
            before,
        } = tx
            .users()
            .lock_by_login(auth.user().login())
            .await?
            .filter(|locked| is_unchanged(auth, checked, &locked.state))
            .ok_or(AppError::Unauthenticated)?;
        tx.attempts().mark_succeeded(attempt).await?;
        let after = tx.users().change_own_password(locked.id, new_hash).await?;
        tx.sessions().delete_all_of_user(locked.id).await?;
        let epoch = locked
            .session_epoch
            .checked_add(1)
            .ok_or_else(|| StorageError::invariant("the session epoch overflows"))?;
        let idle_timeout = self.ctx.settings.session_idle_timeout;
        tx.sessions()
            .insert(&issued.new_session(locked.id, epoch, idle_timeout))
            .await?;
        let entry = AuditEntry::row(
            RowAction::OwnPasswordChanged,
            AuditTarget::app_user(locked.id),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            audit_context(auth.user(), meta),
        );
        self.ctx.app.uow.commit(tx, NonEmpty::new(entry)).await?;
        Ok(issued)
    }
}

/// Tell whether the locked row still matches the session and the check of the current password.
fn is_unchanged(auth: &Authenticated, checked: &UserRow, locked: &UserRow) -> bool {
    locked.is_active
        && locked.session_epoch == auth.epoch()
        && locked.password_hash == checked.password_hash
}

#[cfg(test)]
mod tests;
