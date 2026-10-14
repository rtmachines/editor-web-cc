//! The new session that `Login` and `ChangeOwnPassword` issue (design D4, D5).

use std::fmt;
use std::time::Duration;

use recipe_domain::audit::{RequestId, SessionRef};
use recipe_domain::identity::{CsrfToken, SessionId, SessionToken, UserId};

use crate::error::{AppError, TaskStage};
use crate::ports::NewSession;

/// Number of random bytes of a session token.
const TOKEN_BYTES: usize = 16;
/// Number of random bytes of a CSRF token.
const CSRF_BYTES: usize = 32;
/// Number of random bytes of a UUID.
const UUID_BYTES: usize = 16;
/// Text that `Debug` prints in place of a secret.
const REDACTED: &str = "[redacted]";

/// Read `N` random bytes from the system.
///
/// # Errors
///
/// Returns `AppError::TaskFailed` with the stage `Token` (`E-WEB-011`) when the read fails.
pub(super) fn random_bytes<const N: usize>() -> Result<[u8; N], AppError> {
    let mut bytes = [0_u8; N];
    getrandom::fill(&mut bytes).map_err(|_failed| AppError::TaskFailed {
        stage: TaskStage::Token,
    })?;
    Ok(bytes)
}

/// Number of random bytes of a pre-session nonce: 128 bits (`api.md` section 7.3).
pub const NONCE_BYTES: usize = 16;

/// Read a 128-bit nonce for the pre-session CSRF cookie from the system.
///
/// The function solves one problem: `Uuid::new_v4` has 122 random bits and panics when the
/// operating system gives no random bytes. This function gives 128 bits and returns an error.
///
/// # Errors
///
/// Returns `AppError::TaskFailed` with the stage `Token` (`E-WEB-011`) when the read fails.
pub fn random_nonce() -> Result<[u8; NONCE_BYTES], AppError> {
    random_bytes::<NONCE_BYTES>()
}

/// Create the identifier of a request that has no HTTP request, such as `create-admin`.
///
/// The function solves one problem: `recipe-server` must name a `RequestId` without a `uuid`
/// dependency, and `Uuid::new_v4` panics when the operating system gives no random bytes.
/// The result is a version 4 UUID.
///
/// # Errors
///
/// Returns `AppError::TaskFailed` with the stage `Token` (`E-WEB-011`) when the read fails.
pub fn new_request_id() -> Result<RequestId, AppError> {
    Ok(RequestId::from(random_uuid()?))
}

/// Create a version 4 UUID from the random bytes of the system.
///
/// The function solves one problem: `Uuid::new_v4` panics when the operating system gives no
/// random bytes. `new_request_id`, `CreateUser`, and `bootstrap_administrator` share this function.
///
/// # Errors
///
/// Returns `AppError::TaskFailed` with the stage `Token` (`E-WEB-011`) when the read fails.
pub(super) fn random_uuid() -> Result<uuid::Uuid, AppError> {
    let bytes = random_bytes::<UUID_BYTES>()?;
    Ok(uuid::Builder::from_random_bytes(bytes).into_uuid())
}

/// A session that a use case created.
///
/// The type solves one problem: `recipe-web` sets the cookie from the token and renders the CSRF
/// token, and both secrets must stay out of every log line. `Debug` redacts them.
#[derive(Clone)]
pub struct IssuedSession {
    token: SessionToken,
    csrf_token: CsrfToken,
    session_ref: SessionRef,
}

impl IssuedSession {
    /// Create a token and a CSRF token from random bytes.
    pub(super) fn generate() -> Result<Self, AppError> {
        let token = SessionToken::from_bytes(random_bytes::<TOKEN_BYTES>()?);
        let csrf_token = CsrfToken::from_bytes(random_bytes::<CSRF_BYTES>()?);
        let session_ref = SessionRef::of(&SessionId::of(&token));
        Ok(Self {
            token,
            csrf_token,
            session_ref,
        })
    }

    /// Return the token for the session cookie.
    #[must_use]
    pub const fn token(&self) -> &SessionToken {
        &self.token
    }

    /// Return the CSRF token of the session.
    #[must_use]
    pub const fn csrf_token(&self) -> &CsrfToken {
        &self.csrf_token
    }

    /// Return the reference that audit entries and logs may hold.
    #[must_use]
    pub const fn session_ref(&self) -> &SessionRef {
        &self.session_ref
    }

    /// Build the row for `SessionRepository::insert`.
    pub(super) fn new_session(
        &self,
        user_id: UserId,
        epoch: i64,
        idle_timeout: Duration,
    ) -> NewSession {
        NewSession {
            id: SessionId::of(&self.token),
            user_id,
            epoch,
            csrf_token: self.csrf_token.clone(),
            idle_timeout,
        }
    }
}

impl fmt::Debug for IssuedSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IssuedSession")
            .field("token", &REDACTED)
            .field("csrf_token", &REDACTED)
            .field("session_ref", &self.session_ref)
            .finish()
    }
}

#[cfg(test)]
mod tests;
