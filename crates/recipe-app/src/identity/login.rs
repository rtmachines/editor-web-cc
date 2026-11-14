//! The use case `Login` (`domain-model.md` section 5.3, `data-model.md` section 6, design D6).

use std::net::IpAddr;
use std::sync::Arc;

use recipe_domain::audit::{
    AuditEntry, AuditTarget, LoginHmac, LoginSubject, SecurityEvent, StateAction,
};
use recipe_domain::identity::{
    CredentialFailure, CsrfToken, IdentityError, Login as UserLogin, Password, SessionId,
    SessionToken, TypedLogin, UserId,
};
use serde_json::json;

use super::detached::{RefusalSender, record_security_event};
use super::issue::IssuedSession;
use super::no_session::require_no_session;
use super::throttle::{Gate, Requester, begin_attempt, throttled_event};
use super::{HashPermit, IdentityContext};
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
    /// The request future takes the Argon2 permit and a slot of `identity.detached_tasks`. A
    /// detached task then runs T1, the password check, and the write of the failure event, so a
    /// dropped request still leaves its event (ADR-0172). T2 runs here, only for a waiting caller.
    /// After a passed check, the task keeps its slot until T2 decides. A refusal of T2 sends its
    /// event to the task, which writes it, so a drop during that write loses nothing (ADR-0173).
    ///
    /// # Errors
    ///
    /// Returns `Timeout` (`E-WEB-009`) when no Argon2 permit or no detached slot is free in time.
    /// Returns `LoginThrottled` (`E-IDN-005`) inside a delay or when the address budget is used up.
    /// Returns `InvalidCredentials` (`E-IDN-003`) for an unknown login, a wrong password, or a user
    /// that changed after the check.
    /// Returns `UserInactive` (`E-IDN-004`) for an inactive user.
    /// Returns `TaskFailed` (`E-WEB-011`) when the hash task, the detached task, or the read of
    /// random bytes fails.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails. A
    /// failed write of a security event gives the refusal as without the fault and one `WARN` line
    /// of the task (ADR-0172 (q)).
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
        let slot = self.ctx.detached.acquire().await?;
        let attempt = Attempt {
            ctx: self.ctx.clone(),
            requester: requester.clone(),
            login,
            password,
        };
        let sink = Arc::clone(&self.ctx.app.security);
        let ((user, attempt), refusal) = self
            .ctx
            .detached
            .spawn_judged(slot, sink, attempt.judge(permit))
            .await??;
        self.grant(meta, &requester, (user, attempt), old_session, refusal)
            .await
    }

    /// Send the event for a user that changed after the check and return the error (ADR-0035 m).
    ///
    /// The caller has dropped the transaction. The detached task writes the event in the short
    /// transaction of the sink, and this call waits until it has done so (ADR-0173 (e), (g)).
    async fn refuse_changed<T>(
        requester: &Requester,
        id: UserId,
        still_active: bool,
        refusal: RefusalSender,
    ) -> Result<T, AppError> {
        let event = if still_active {
            login_failed(requester, LoginSubject::User(id))
        } else {
            SecurityEvent::login_rejected_inactive(
                id,
                requester.request_id,
                requester.client_ip,
                requester.user_agent.clone(),
            )
        };
        refusal.refuse(event).await;
        Err(changed_user().into())
    }

    /// T2: lock the user, check the credentials again, and create the session.
    ///
    /// `passed` holds the checked row and the key of the attempt. The decision of T2 ends the wait
    /// of the task: a refusal sends the event through `refusal`, and a grant drops it.
    async fn grant(
        &self,
        meta: &RequestMeta,
        requester: &Requester,
        passed: (UserRow, AttemptId),
        old_session: Option<SessionId>,
        refusal: RefusalSender,
    ) -> Result<LoginOutcome, AppError> {
        let (checked, attempt) = passed;
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
            return Self::refuse_changed(requester, checked.id, still_active, refusal).await;
        };
        drop(refusal);
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

/// The data of the detached span of `Login`: T1, the password check, and the failure event.
///
/// The type solves one problem: the task must own everything it uses, because the request future
/// can drop at any `.await` (ADR-0172).
struct Attempt {
    ctx: IdentityContext,
    requester: Requester,
    login: TypedLogin,
    password: Password,
}

impl Attempt {
    /// Run T1 and the password check, and write the event of a refusal.
    ///
    /// The task holds the permit during the hash and drops it before the event write. A refusal
    /// writes its event here and returns its error. A pass returns the checked row and the key of
    /// the attempt for T2.
    async fn judge(self, permit: HashPermit) -> Result<(UserRow, AttemptId), AppError> {
        let Self {
            ctx,
            requester,
            login,
            password,
        } = self;
        let attempt = match begin_attempt(&ctx, &login, requester.client_ip).await? {
            Gate::Open(attempt) => attempt,
            Gate::Throttled { retry_after } => {
                drop(permit);
                return throttle(&ctx, &requester, &login, retry_after).await;
            }
        };
        let (stored, matched, permit) = check_password(&ctx, permit, &login, password).await?;
        drop(permit);
        let user = pass_or_refuse(&ctx, &requester, &login, stored, matched).await?;
        Ok((user, attempt))
    }
}

/// Return the active person whose password matched. Else write the event and return the error.
async fn pass_or_refuse(
    ctx: &IdentityContext,
    requester: &Requester,
    typed: &TypedLogin,
    stored: Option<UserRow>,
    matched: bool,
) -> Result<UserRow, AppError> {
    let Some(row) = stored else {
        let hmac = LoginHmac::compute(&ctx.login_keys, typed);
        let subject = LoginSubject::Unknown(hmac);
        let error = IdentityError::InvalidCredentials {
            cause: CredentialFailure::UnknownLogin,
        };
        return refuse(ctx, login_failed(requester, subject), error).await;
    };
    if !row.is_active {
        let event = SecurityEvent::login_rejected_inactive(
            row.id,
            requester.request_id,
            requester.client_ip,
            requester.user_agent.clone(),
        );
        return refuse(ctx, event, IdentityError::UserInactive).await;
    }
    if !matched {
        let event = login_failed(requester, LoginSubject::User(row.id));
        return refuse(ctx, event, changed_user()).await;
    }
    if !row.role.is_person() {
        // The hash check ran in full, so the time does not show the role (ADR-0109 (c)).
        let event = SecurityEvent::login_failed_technical_account(
            row.id,
            requester.request_id,
            requester.client_ip,
            requester.user_agent.clone(),
        );
        return refuse(ctx, event, changed_user()).await;
    }
    Ok(row)
}

/// Write the event `login_throttled` and return `E-IDN-005`.
async fn throttle<T>(
    ctx: &IdentityContext,
    requester: &Requester,
    login: &TypedLogin,
    retry_after: u64,
) -> Result<T, AppError> {
    let event = throttled_event(ctx, requester, login);
    refuse(ctx, event, IdentityError::LoginThrottled { retry_after }).await
}

/// Write a failure event and return the error of the login.
///
/// A failed write gives one `WARN` line and no storage error (ADR-0172 (o), (q)).
async fn refuse<T>(
    ctx: &IdentityContext,
    event: SecurityEvent,
    error: IdentityError,
) -> Result<T, AppError> {
    record_security_event(ctx.app.security.as_ref(), event).await;
    Err(error.into())
}

/// Read the user in a read-only transaction and end that transaction.
///
/// A typed login outside the grammar of `Login` counts as unknown (design D6).
async fn read_user(ctx: &IdentityContext, typed: &TypedLogin) -> Result<Option<UserRow>, AppError> {
    let Ok(login) = UserLogin::parse(typed.as_str()) else {
        return Ok(None);
    };
    let mut tx = ctx.app.uow.begin(Isolation::ReadOnly).await?;
    Ok(tx.users().find_by_login(&login).await?)
}

/// Read the user and check the password in `spawn_blocking`, with no transaction open.
///
/// The function solves one problem: `Login` and the Basic check must do the same hash work for
/// every outcome (A42). An unknown or inactive user gets the dummy hash, so the verifier runs
/// once with the same parameters in every case. The function returns the row, the answer of the
/// check, and the permit.
///
/// # Errors
///
/// Returns the storage error when the read fails. Returns `TaskFailed` (`E-WEB-011`) when the
/// blocking task fails.
pub(super) async fn check_password(
    ctx: &IdentityContext,
    permit: HashPermit,
    typed: &TypedLogin,
    password: Password,
) -> Result<(Option<UserRow>, bool, HashPermit), AppError> {
    let stored = read_user(ctx, typed).await?;
    let hash = stored
        .as_ref()
        .filter(|row| row.is_active)
        .map(|row| row.password_hash.clone());
    let (matched, permit) = permit.verify(password, hash).await?;
    Ok((stored, matched, permit))
}

/// Build the event `login_failed` of a request.
pub(super) fn login_failed(requester: &Requester, subject: LoginSubject) -> SecurityEvent {
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
mod detached;
#[cfg(test)]
mod tests;
