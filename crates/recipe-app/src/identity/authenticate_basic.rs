//! The Basic check of the export routes (design D2, ADR-0109 (m)).

use std::fmt;
use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{BasicRefusal, LoginHmac, LoginSubject, SecurityEvent};
use recipe_domain::identity::{CredentialFailure, IdentityError, Password, TypedLogin};

use super::login::{check_password, login_failed};
use super::throttle::{Gate, Requester, begin_attempt, throttled_event};
use super::{HashPermit, IdentityContext, record_security_event};
use crate::auth::{CurrentUser, RequestMeta};
use crate::error::AppError;
use crate::ports::{AttemptId, BasicChange, BasicConfirmation, UserRow};

/// Text that `Debug` prints in place of a secret.
const REDACTED: &str = "[redacted]";

/// The address that stands for a request without a client address.
///
/// All such requests share one throttle budget, so a missing address never gives a free budget.
const UNKNOWN_CLIENT_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

/// The credentials of an `Authorization: Basic` header after the parse.
///
/// `Debug` redacts the password by hand, because the log must never show it.
pub struct BasicCredentials {
    /// The typed login.
    pub login: TypedLogin,
    /// The typed password.
    pub password: Password,
}

impl fmt::Debug for BasicCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicCredentials")
            .field("login", &self.login)
            .field("password", &REDACTED)
            .finish()
    }
}

/// The result of a successful Basic check (ADR-0109 (mb)).
///
/// The type solves one problem: an export request has an account but no session, so the result
/// holds no session value and the use case never builds an `Authenticated`.
#[derive(Clone, Debug)]
pub struct BasicPrincipal {
    /// The account that the check accepted.
    pub user: CurrentUser,
}

/// Check the credentials of an `Authorization: Basic` header (design D2, ADR-0109 (m)).
///
/// The function solves one problem: an export route needs the account of a transfer client
/// without a session, and the answer must not tell a caller which part of the credentials is
/// wrong (A42). It takes the Basic permit, the Argon2 permit, and a slot of the detached tasks.
/// A detached task then runs T1 of `Login`, reads the user, checks the password in
/// `spawn_blocking`, and writes the security event of a refusal (ADR-0172). An unknown or inactive user gets the dummy
/// hash, so the hash work is the same in every case. A person account fails after the hash check.
/// The Basic permit travels with the Argon2 permit into the task, so both stay held until the hash
/// ends. When the caller drops the future, the task still writes the event. T2 runs in the request
/// future. The function writes no audit entry.
///
/// The client address comes from `meta`. A request without one uses the address `0.0.0.0`.
///
/// # Errors
///
/// Returns `Timeout` (`E-WEB-009`) when a permit or a slot is not free in time.
/// Returns `LoginThrottled` (`E-IDN-005`) inside a delay or when the address budget is used up.
/// Returns `InvalidCredentials` (`E-IDN-003`) for an unknown login, an inactive account, a wrong
/// password, or a person account. A failed write of the event gives the same answer and the one
/// `WARN` line of a lost event (ADR-0172 (q)).
/// Returns `TaskFailed` (`E-WEB-011`) when the hash task or the detached task fails.
/// Returns `AppError::Storage` when the database fails in T1 or T2.
#[tracing::instrument(name = "authenticate_basic", skip_all)]
pub async fn authenticate_basic(
    ctx: &IdentityContext,
    credentials: BasicCredentials,
    meta: &RequestMeta,
) -> Result<BasicPrincipal, AppError> {
    let BasicCredentials { login, password } = credentials;
    let client_ip = meta.client_ip().unwrap_or(UNKNOWN_CLIENT_IP);
    let requester = Requester::of(meta, client_ip);
    let hash_permit = ctx.acquire_basic().await?;
    let slot = ctx.detached().acquire().await?;
    let span = checked_span(ctx.clone(), requester.clone(), login, password, hash_permit);
    let Checked { row, attempt } = ctx.detached().spawn(slot, span).await??;
    confirm(ctx, &requester, attempt, &row).await?;
    Ok(BasicPrincipal {
        user: CurrentUser::new(row.id, row.login, row.role),
    })
}

/// The account and the attempt that the detached span hands to the request future.
struct Checked {
    row: UserRow,
    attempt: AttemptId,
}

/// Run T1, the throttle decision, the hash check, and the judge. Write the event of a refusal.
///
/// The function solves one problem: the committed attempt of T1 needs its event also when the
/// caller drops the request future (ADR-0172 (i)). It runs in a detached task that owns the
/// permits. The step after the judge stays in the request future (ADR-0172 (j)).
async fn checked_span(
    ctx: IdentityContext,
    requester: Requester,
    login: TypedLogin,
    password: Password,
    hash_permit: HashPermit,
) -> Result<Checked, AppError> {
    let attempt = match begin_attempt(&ctx, &login, requester.client_ip).await? {
        Gate::Open(attempt) => attempt,
        Gate::Throttled { retry_after } => {
            drop(hash_permit);
            return throttle(&ctx, &requester, &login, retry_after).await;
        }
    };
    let (stored, matched, hash_permit) =
        check_password(&ctx, hash_permit, &login, password).await?;
    drop(hash_permit);
    let row = judge(&ctx, &requester, &login, stored, matched).await?;
    Ok(Checked { row, attempt })
}

/// Write the event `login_throttled` with the scheme, and return `E-IDN-005`.
async fn throttle<T>(
    ctx: &IdentityContext,
    requester: &Requester,
    login: &TypedLogin,
    retry_after: u64,
) -> Result<T, AppError> {
    let event = throttled_event(ctx, requester, login);
    record_security_event(ctx.app.security.as_ref(), event.with_basic_scheme()).await;
    Err(IdentityError::LoginThrottled { retry_after }.into())
}

/// Return the active technical account whose password matched. Else write the event and refuse.
async fn judge(
    ctx: &IdentityContext,
    requester: &Requester,
    typed: &TypedLogin,
    stored: Option<UserRow>,
    matched: bool,
) -> Result<UserRow, AppError> {
    let Some(row) = stored else {
        let hmac = LoginHmac::compute(&ctx.login_keys, typed);
        let event = login_failed(requester, LoginSubject::Unknown(hmac));
        let cause = CredentialFailure::UnknownLogin;
        return refuse(ctx, event, BasicRefusal::UnknownLogin, cause).await;
    };
    let cause = CredentialFailure::WrongPassword;
    if !row.is_active {
        let event = SecurityEvent::login_rejected_inactive(
            row.id,
            requester.request_id,
            requester.client_ip,
            requester.user_agent.clone(),
        );
        return refuse(ctx, event, BasicRefusal::Inactive, cause).await;
    }
    let event = login_failed(requester, LoginSubject::User(row.id));
    if !matched {
        return refuse(ctx, event, BasicRefusal::WrongPassword, cause).await;
    }
    if row.role.is_person() {
        // The hash check ran in full, so the time does not show the role (ADR-0109 (c)).
        return refuse(ctx, event, BasicRefusal::PersonAccount, cause).await;
    }
    Ok(row)
}

/// Run T2 of design D2 step 6. If the account changed during the hash check, refuse.
///
/// T2 needs no permit and writes no audit entry. It runs in read-only mode too (A24).
async fn confirm(
    ctx: &IdentityContext,
    requester: &Requester,
    attempt: AttemptId,
    row: &UserRow,
) -> Result<(), AppError> {
    let confirmation = ctx
        .attempts
        .confirm_basic(attempt, row.id, &row.password_hash)
        .await?;
    let BasicConfirmation::Changed { reason } = confirmation else {
        return Ok(());
    };
    let cause = CredentialFailure::WrongPassword;
    let (event, refusal) = match reason {
        BasicChange::Inactive => {
            let event = SecurityEvent::login_rejected_inactive(
                row.id,
                requester.request_id,
                requester.client_ip,
                requester.user_agent.clone(),
            );
            (event, BasicRefusal::Inactive)
        }
        BasicChange::Missing => (
            login_failed(requester, LoginSubject::User(row.id)),
            BasicRefusal::UnknownLogin,
        ),
        BasicChange::NotExportClient => (
            login_failed(requester, LoginSubject::User(row.id)),
            BasicRefusal::PersonAccount,
        ),
        BasicChange::PasswordChanged => (
            login_failed(requester, LoginSubject::User(row.id)),
            BasicRefusal::WrongPassword,
        ),
    };
    refuse(ctx, event, refusal, cause).await
}

/// Write the event with the reason and the scheme, and return `E-IDN-003`.
///
/// A failed write gives the one `WARN` line of a lost event and the same answer (ADR-0172 (q)).
async fn refuse<T>(
    ctx: &IdentityContext,
    event: SecurityEvent,
    reason: BasicRefusal,
    cause: CredentialFailure,
) -> Result<T, AppError> {
    record_security_event(ctx.app.security.as_ref(), event.with_basic_refusal(reason)).await;
    Err(IdentityError::InvalidCredentials { cause }.into())
}

#[cfg(test)]
mod tests;
