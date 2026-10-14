//! Authentication of a request (design D10). It is not a use case: it has no permission.

use std::fmt;

use recipe_domain::audit::SessionRef;
use recipe_domain::identity::{CsrfToken, SessionId, SessionToken};

use super::IdentityContext;
use crate::auth::CurrentUser;
use crate::error::AppError;

/// Text that `Debug` prints in place of a secret.
const REDACTED: &str = "[redacted]";

/// A request that carries a valid session.
///
/// The type solves one problem: `recipe-web` needs the user, the session reference, and the CSRF
/// token of the session after one statement, and it must not log the secrets. The role comes from
/// the `app_user` row of this request, never from a cache (ADR-0014). `Debug` redacts the session
/// id and the CSRF token.
#[derive(Clone)]
pub struct Authenticated {
    user: CurrentUser,
    session_id: SessionId,
    session_ref: SessionRef,
    epoch: i64,
    csrf_token: CsrfToken,
    must_change_password: bool,
}

impl Authenticated {
    /// Return the user of the session with the role of this request.
    #[must_use]
    pub const fn user(&self) -> &CurrentUser {
        &self.user
    }

    /// Return the id of the session (the SHA-256 of the cookie token).
    #[must_use]
    pub const fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Return the reference that audit entries and logs may hold.
    #[must_use]
    pub const fn session_ref(&self) -> &SessionRef {
        &self.session_ref
    }

    /// Return the epoch of the session.
    #[must_use]
    pub const fn epoch(&self) -> i64 {
        self.epoch
    }

    /// Return the CSRF token of the session.
    #[must_use]
    pub const fn csrf_token(&self) -> &CsrfToken {
        &self.csrf_token
    }

    /// Tell whether the user must change the password before any other request.
    #[must_use]
    pub const fn must_change_password(&self) -> bool {
        self.must_change_password
    }
}

impl fmt::Debug for Authenticated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Authenticated")
            .field("user", &self.user)
            .field("session_id", &REDACTED)
            .field("session_ref", &self.session_ref)
            .field("epoch", &self.epoch)
            .field("csrf_token", &REDACTED)
            .field("must_change_password", &self.must_change_password)
            .finish()
    }
}

/// Turn the session cookie of a request into the data of its session.
///
/// The function hashes the token to the session id and runs the one statement of
/// `SessionStore::authenticate`. A valid call moves `last_seen_at` and `expires_at`. It also runs
/// in read-only mode (ADR-0035 k). A missing, idle, old, inactive, or other-epoch session gives
/// `None`, and the caller decides what that means for its route.
///
/// # Errors
///
/// Returns `AppError::Storage` when the statement fails.
pub async fn authenticate(
    ctx: &IdentityContext,
    token: &SessionToken,
) -> Result<Option<Authenticated>, AppError> {
    let id = SessionId::of(token);
    let limits = ctx.settings.session_limits();
    let Some(row) = ctx.sessions.authenticate(&id, &limits).await? else {
        return Ok(None);
    };
    Ok(Some(Authenticated {
        user: CurrentUser::new(row.user_id, row.login, row.role),
        session_ref: SessionRef::of(&id),
        session_id: id,
        epoch: row.epoch,
        csrf_token: row.csrf_token,
        must_change_password: row.must_change_password,
    }))
}

#[cfg(test)]
mod tests;
