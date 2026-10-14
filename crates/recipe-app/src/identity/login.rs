//! The use case `Login` (`domain-model.md` section 5.3, `data-model.md` section 6, design D6).

use std::net::IpAddr;

use recipe_domain::audit::{
    AuditEntry, AuditTarget, LoginHmac, LoginSubject, SecurityEvent, StateAction,
};
use recipe_domain::identity::{
    CredentialFailure, CsrfToken, IdentityError, Login as UserLogin, Password, SessionId,
    SessionToken, TypedLogin, UserId,
};
use serde_json::json;

use super::IdentityContext;
use super::issue::IssuedSession;
use super::no_session::require_no_session;
use super::throttle::{Gate, Requester, begin_attempt, reject_throttled};
use crate::auth::{CurrentUser, Operation, RequestMeta};
use crate::context::{UseCase, audit_context};
use crate::error::AppError;
use crate::ports::{AttemptId, Isolation, NonEmpty, UserRow};

use recipe_domain::audit::SessionRef;

/// The parsed request of `Login`.
///
/// `Debug` shows no secret, because `Password`, `TypedLogin`, and `SessionId` redact themselves.
#[derive(Clone, Debug)]
pub struct LoginInput {
    /// The typed login of the form field.
    pub login: TypedLogin,
    /// The typed password of the form field.
    pub password: Password,
    /// The canonical address of the client.
    pub client_ip: IpAddr,
    /// The session of the request cookie, when the request carries one. T2 deletes it.
    pub old_session: Option<SessionId>,
}

/// The result of a successful `Login`.
///
/// `Debug` redacts the token and the CSRF token.
#[derive(Clone, Debug)]
pub struct LoginOutcome {
    session: IssuedSession,
    must_change_password: bool,
}

impl LoginOutcome {
    /// Return the token for the session cookie.
    #[must_use]
    pub const fn token(&self) -> &SessionToken {
        self.session.token()
    }

    /// Return the CSRF token of the new session.
    #[must_use]
    pub const fn csrf_token(&self) -> &CsrfToken {
        self.session.csrf_token()
    }

    /// Return the reference of the new session.
    #[must_use]
    pub const fn session_ref(&self) -> &SessionRef {
        self.session.session_ref()
    }

    /// Tell whether the user must change the password before any other request.
    #[must_use]
    pub const fn must_change_password(&self) -> bool {
        self.must_change_password
    }
}

/// Logs a user in.
///
/// The use case solves one problem: a login must neither hold a connection during the slow
/// Argon2 check nor create a session for a user that changed meanwhile. It takes a permit, runs
/// T1, checks the password outside any transaction, and runs T2 with the credentials checked
/// again under the lock of the user row (design D6, ADR-0035 m).
#[derive(Clone)]
pub struct Login {
    ctx: IdentityContext,
}

impl UseCase for Login {
    const OPERATION: Operation = Operation::Login;
}

/// The error for a user that changed after the check of the password (ADR-0035 m).
const fn changed_user() -> IdentityError {
    IdentityError::InvalidCredentials {
        cause: CredentialFailure::WrongPassword,
    }
}

impl Login {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Run the login.
    ///
    /// # Errors
    ///
    /// Returns `Timeout` (`E-WEB-009`) when no Argon2 permit is free in time.
    /// Returns `LoginThrottled` (`E-IDN-005`) inside a delay or when the address budget is used up.
    /// Returns `InvalidCredentials` (`E-IDN-003`) for an unknown login, a wrong password, or a user
    /// that changed after the check.
    /// Returns `UserInactive` (`E-IDN-004`) for an inactive user.
    /// Returns `TaskFailed` (`E-WEB-011`) when the hash task or the read of random bytes fails.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails. A
    /// failed write of a security event also returns the storage error (ADR-0035 t).
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        meta: &RequestMeta,
        input: LoginInput,
    ) -> Result<LoginOutcome, AppError> {
        require_no_session(Self::OPERATION)?;
        let LoginInput {
            login,
            password,
            client_ip,
            old_session,
        } = input;
        let requester = Requester::of(meta, client_ip);
        let permit = self.ctx.gate.acquire().await?;
        let attempt = match begin_attempt(&self.ctx, &login, client_ip).await? {
            Gate::Open(attempt) => attempt,
            Gate::Throttled { retry_after } => {
                drop(permit);
                return reject_throttled(&self.ctx, &requester, &login, retry_after).await;
            }
        };
        let stored = self.read_user(&login).await?;
        let hash = stored
            .as_ref()
            .filter(|row| row.is_active)
            .map(|row| row.password_hash.clone());
        let (matched, permit) = permit.verify(password, hash).await?;
        drop(permit);
        let user = self.judge(&requester, &login, stored, matched).await?;
        self.grant(meta, &requester, user, attempt, old_session)
            .await
    }

    /// Read the user in a read-only transaction and end that transaction.
    ///
    /// A typed login outside the grammar of `Login` counts as unknown (design D6).
    async fn read_user(&self, typed: &TypedLogin) -> Result<Option<UserRow>, AppError> {
        let Ok(login) = UserLogin::parse(typed.as_str()) else {
            return Ok(None);
        };
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadOnly).await?;
        Ok(tx.users().find_by_login(&login).await?)
    }

    /// Return the active user whose password matched. Else write the event and return the error.
    async fn judge(
        &self,
        requester: &Requester,
        typed: &TypedLogin,
        stored: Option<UserRow>,
        matched: bool,
    ) -> Result<UserRow, AppError> {
        let Some(row) = stored else {
            let hmac = LoginHmac::compute(&self.ctx.login_keys, typed);
            let subject = LoginSubject::Unknown(hmac);
            return self
                .refuse(
                    login_failed(requester, subject),
                    IdentityError::InvalidCredentials {
                        cause: CredentialFailure::UnknownLogin,
                    },
                )
                .await;
        };
        if !row.is_active {
            let event = SecurityEvent::login_rejected_inactive(
                row.id,
                requester.request_id,
                requester.client_ip,
                requester.user_agent.clone(),
            );
            return self.refuse(event, IdentityError::UserInactive).await;
        }
        if !matched {
            let event = login_failed(requester, LoginSubject::User(row.id));
            return self.refuse(event, changed_user()).await;
        }
        Ok(row)
    }

    /// Write a failure event and return the error of the login.
    async fn refuse<T>(&self, event: SecurityEvent, error: IdentityError) -> Result<T, AppError> {
        self.ctx.app.security.record(event).await?;
        Err(error.into())
    }

    /// Write the event for a user that changed after the check and return the error (ADR-0035 m).
    ///
    /// The caller has dropped the transaction. The sink uses its own short transaction.
    async fn refuse_changed<T>(
        &self,
        requester: &Requester,
        id: UserId,
        still_active: bool,
    ) -> Result<T, AppError> {
        if still_active {
            let event = login_failed(requester, LoginSubject::User(id));
            return self.refuse(event, changed_user()).await;
        }
        let event = SecurityEvent::login_rejected_inactive(
            id,
            requester.request_id,
            requester.client_ip,
            requester.user_agent.clone(),
        );
        self.refuse(event, changed_user()).await
    }

    /// T2: lock the user, check the credentials again, and create the session.
    async fn grant(
        &self,
        meta: &RequestMeta,
        requester: &Requester,
        checked: UserRow,
        attempt: AttemptId,
        old_session: Option<SessionId>,
    ) -> Result<LoginOutcome, AppError> {
        let issued = IssuedSession::generate()?;
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadCommitted).await?;
        let locked = tx
            .users()
            .lock_by_login(&checked.login)
            .await?
            .map(|locked| locked.state);
        let Some(user) = locked.as_ref().filter(|row| is_unchanged(&checked, row)) else {
            let still_active = locked.as_ref().is_none_or(|row| row.is_active);
            drop(tx);
            return self
                .refuse_changed(requester, checked.id, still_active)
                .await;
        };
        tx.attempts().mark_succeeded(attempt).await?;
        if let Some(old) = &old_session {
            tx.sessions().delete(old).await?;
        }
        let idle_timeout = self.ctx.settings.session_idle_timeout;
        let session = issued.new_session(user.id, user.session_epoch, idle_timeout);
        tx.sessions().insert(&session).await?;
        let actor = CurrentUser::new(user.id, user.login.clone(), user.role);
        let entry = AuditEntry::state(
            StateAction::UserLoggedIn,
            AuditTarget::app_user(user.id),
            json!({ "session_ref": issued.session_ref().as_str() }),
            audit_context(&actor, meta),
        );
        self.ctx.app.uow.commit(tx, NonEmpty::new(entry)).await?;
        Ok(LoginOutcome {
            session: issued,
            must_change_password: user.must_change_password,
        })
    }
}

/// Build the event `login_failed` of a request.
fn login_failed(requester: &Requester, subject: LoginSubject) -> SecurityEvent {
    SecurityEvent::login_failed(
        subject,
        requester.request_id,
        requester.client_ip,
        requester.user_agent.clone(),
    )
}

/// Tell whether the locked row still matches the row that the password check read.
fn is_unchanged(checked: &UserRow, locked: &UserRow) -> bool {
    locked.is_active
        && locked.session_epoch == checked.session_epoch
        && locked.password_hash == checked.password_hash
}

#[cfg(test)]
mod tests;
