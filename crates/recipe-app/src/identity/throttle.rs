//! T1 of `Login` and of `ChangeOwnPassword` and the refusal of a throttled attempt (design D6).

use std::net::IpAddr;

use recipe_domain::audit::{LoginHmac, RequestId, SecurityEvent, UserAgent};
use recipe_domain::identity::{IdentityError, LoginThrottle, ThrottleDecision, TypedLogin};

use super::IdentityContext;
use crate::auth::RequestMeta;
use crate::error::{AppError, StorageError};
use crate::ports::{AttemptId, AttemptKey};

/// The request data that a security event of a login attempt copies.
///
/// The type solves one problem: `Login` and `ChangeOwnPassword` write the same events, and both
/// need the request identifier, the client address, and the User-Agent header at each write.
#[derive(Clone, Debug)]
pub(super) struct Requester {
    pub(super) request_id: RequestId,
    pub(super) client_ip: IpAddr,
    pub(super) user_agent: Option<UserAgent>,
}

impl Requester {
    /// Copy the request data. The client address comes from the input, not from `meta`.
    pub(super) fn of(meta: &RequestMeta, client_ip: IpAddr) -> Self {
        Self {
            request_id: meta.request_id(),
            client_ip,
            user_agent: meta.user_agent().cloned(),
        }
    }
}

/// The answer of T1.
pub(super) enum Gate {
    /// The attempt may go on. The key is the row that T1 inserted.
    Open(AttemptId),
    /// The attempt is inside a delay or the budget of the address is used up.
    Throttled {
        /// Whole seconds until the attempt can pass.
        retry_after: u64,
    },
}

/// Run T1 and decide with the login throttle.
///
/// The caller holds a permit and no connection (design D5).
pub(super) async fn begin_attempt(
    ctx: &IdentityContext,
    login: &TypedLogin,
    client_ip: IpAddr,
) -> Result<Gate, AppError> {
    let key = AttemptKey {
        login: login.clone(),
        client_ip,
    };
    let policy = ctx.settings.throttle;
    let outcome = ctx.attempts.begin_attempt(&key, &policy).await?;
    if let ThrottleDecision::Throttled { retry_after } =
        LoginThrottle::new(policy).decide(&outcome.count)
    {
        return Ok(Gate::Throttled { retry_after });
    }
    outcome
        .attempt
        .map(Gate::Open)
        .ok_or_else(|| StorageError::invariant("the attempt of an open gate has no row").into())
}

/// Write the event `login_throttled` and return `E-IDN-005`.
///
/// # Errors
///
/// Returns the storage error when the event cannot be written (ADR-0035 t). Else it returns
/// `IdentityError::LoginThrottled` with `retry_after`.
pub(super) async fn reject_throttled<T>(
    ctx: &IdentityContext,
    requester: &Requester,
    login: &TypedLogin,
    retry_after: u64,
) -> Result<T, AppError> {
    let event = SecurityEvent::login_throttled(
        LoginHmac::compute(&ctx.login_keys, login),
        requester.request_id,
        requester.client_ip,
        requester.user_agent.clone(),
    );
    ctx.app.security.record(event).await?;
    Err(IdentityError::LoginThrottled { retry_after }.into())
}
