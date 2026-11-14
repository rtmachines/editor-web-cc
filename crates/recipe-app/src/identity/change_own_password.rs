//! The use case `ChangeOwnPassword` (design D7, ADR-0035 b and m).

use std::net::IpAddr;

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::identity::{
    IdentityError, Login, NewPassword, Password, PasswordHash, TypedLogin,
};

use super::issue::IssuedSession;
use super::throttle::{Gate, Requester, begin_attempt, throttled_event};
use super::{Authenticated, HashPermit, IdentityContext, record_security_event};
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
    /// The new password of the form field. The caller parsed it with the policy of A23.
    pub new_password: NewPassword,
}

/// Changes the password of the current user.
///
/// The use case solves one problem: a password change must not let a guessed session try the
/// current password without a limit, and it must not overwrite a reset that an Administrator
/// committed meanwhile. It applies the login throttle, checks and hashes under one permit and
/// outside any transaction, and checks the credentials again under the lock (ADR-0035 b and m).
///
/// A detached task runs T1, the throttle event, and the check of the current password. A dropped
/// request future therefore cannot lose the event of a committed attempt (ADR-0172 (i)). The
/// request future runs the hash and `replace`, and only when it receives the result of the task.
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
    /// Returns `Timeout` (`E-WEB-009`) when no Argon2 permit or no detached slot is free in time.
    /// Returns `LoginThrottled` (`E-IDN-005`) inside a delay or when the address budget is used up.
    /// Returns `InvalidCurrentPassword` (`E-IDN-008`) when the current password is wrong.
    /// Returns `Unauthenticated` (`E-WEB-004`) when the user, the epoch, or the hash changed after
    /// the check.
    /// Returns `TaskFailed` (`E-WEB-011`) when the hash task, the detached task, or the read of
    /// random bytes fails.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        input: ChangeOwnPasswordInput,
    ) -> Result<IssuedSession, AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        let user = auth.user();
        let ChangeOwnPasswordInput {
            client_ip,
            current_password,
            new_password,
        } = input;
        let login = TypedLogin::parse(user.login().as_str())
            .map_err(|_unreachable| AppError::Unauthenticated)?;
        let requester = Requester::of(meta, client_ip);
        let permit = self.ctx.gate.acquire().await?;
        let slot = self.ctx.detached().acquire().await?;
        let judging = Judging {
            own: user.login().clone(),
            login,
            client_ip,
            requester,
            permit,
            current_password,
        };
        let judge = self
            .ctx
            .detached()
            .spawn(slot, judge(self.ctx.clone(), judging));
        let Judged {
            checked,
            attempt,
            permit,
        } = judge.await??;
        let (new_hash, permit) = permit.hash(new_password).await?;
        drop(permit);
        self.replace(auth, meta, &checked, attempt, &new_hash).await
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

/// What the detached task needs to judge the current password.
struct Judging {
    own: Login,
    login: TypedLogin,
    client_ip: IpAddr,
    requester: Requester,
    permit: HashPermit,
    current_password: Password,
}

/// The result of a task that found the current password right.
///
/// The type solves one problem: the request future runs the hash and `replace` only when it
/// receives this result. The permit comes back with it, so a dropped request frees the permit.
struct Judged {
    checked: UserRow,
    attempt: AttemptId,
    permit: HashPermit,
}

/// Run T1, the throttle event, and the check of the current password (ADR-0172 (i)).
///
/// The function runs in a detached task. It writes the event `login_throttled` itself and never
/// returns the storage fault of that write (ADR-0172 (o), (q)).
async fn judge(ctx: IdentityContext, judging: Judging) -> Result<Judged, AppError> {
    let Judging {
        own,
        login,
        client_ip,
        requester,
        permit,
        current_password,
    } = judging;
    let attempt = match begin_attempt(&ctx, &login, client_ip).await? {
        Gate::Open(attempt) => attempt,
        Gate::Throttled { retry_after } => {
            drop(permit);
            return Err(refuse_throttled(&ctx, &requester, &login, retry_after).await);
        }
    };
    let checked = read_own_row(&ctx, &own).await?;
    let (matched, permit) = permit
        .verify(current_password, Some(checked.password_hash.clone()))
        .await?;
    if !matched {
        drop(permit);
        return Err(IdentityError::InvalidCurrentPassword.into());
    }
    Ok(Judged {
        checked,
        attempt,
        permit,
    })
}

/// Write the event `login_throttled` and return `E-IDN-005`.
async fn refuse_throttled(
    ctx: &IdentityContext,
    requester: &Requester,
    login: &TypedLogin,
    retry_after: u64,
) -> AppError {
    let event = throttled_event(ctx, requester, login);
    record_security_event(ctx.app.security.as_ref(), event).await;
    IdentityError::LoginThrottled { retry_after }.into()
}

/// Read the own user row in a read-only transaction and end that transaction.
async fn read_own_row(ctx: &IdentityContext, own: &Login) -> Result<UserRow, AppError> {
    let mut tx = ctx.app.uow.begin(Isolation::ReadOnly).await?;
    tx.users()
        .find_by_login(own)
        .await?
        .ok_or(AppError::Unauthenticated)
}

/// Tell whether the locked row still matches the session and the check of the current password.
fn is_unchanged(auth: &Authenticated, checked: &UserRow, locked: &UserRow) -> bool {
    locked.is_active
        && locked.session_epoch == auth.epoch()
        && locked.password_hash == checked.password_hash
}

#[cfg(test)]
mod tests;
