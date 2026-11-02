//! The one-time password that `CreateUser` and `ResetPassword` issue (design D4, D8).

use std::fmt;

use recipe_domain::identity::{Login, ONE_TIME_BYTES, OneTimePassword, PasswordHash, Role};

use super::IdentityContext;
use super::issue::random_bytes;
use crate::error::AppError;

/// Text that `Debug` prints in place of a secret.
const REDACTED: &str = "[redacted]";

/// A one-time password that a command returns once.
///
/// The type solves one problem: the Administrator must read the password once, and the password
/// must stay out of every log line and audit entry. `Debug` redacts it.
#[derive(Clone)]
pub struct IssuedPassword {
    login: Login,
    role: Role,
    password: OneTimePassword,
}

impl IssuedPassword {
    /// Return the login of the user that the password belongs to.
    #[must_use]
    pub const fn login(&self) -> &Login {
        &self.login
    }

    /// Return the role of the user that the password belongs to.
    ///
    /// The page needs it to tell a person from a technical account (ADR-0109 (a)).
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// Return the one-time password. The caller shows it once.
    #[must_use]
    pub const fn password(&self) -> &OneTimePassword {
        &self.password
    }

    /// Pair a login and its role with the password that the use case hashed.
    pub(super) const fn new(login: Login, role: Role, password: OneTimePassword) -> Self {
        Self {
            login,
            role,
            password,
        }
    }
}

impl fmt::Debug for IssuedPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IssuedPassword")
            .field("login", &self.login)
            .field("role", &self.role)
            .field("password", &REDACTED)
            .finish()
    }
}

/// Create a one-time password and its hash outside any transaction (design D5, D8).
///
/// The function takes a permit, hashes in `spawn_blocking`, and returns the permit to the gate
/// before it returns. The caller opens its transaction afterwards.
///
/// # Errors
///
/// Returns `TaskFailed` (`E-WEB-011`) when the read of random bytes or the hash task fails.
/// Returns `Timeout` (`E-WEB-009`) when no Argon2 permit is free in time.
pub(super) async fn hashed_one_time_password(
    ctx: &IdentityContext,
) -> Result<(OneTimePassword, PasswordHash), AppError> {
    let password = OneTimePassword::from_bytes(random_bytes::<ONE_TIME_BYTES>()?);
    let permit = ctx.gate.acquire().await?;
    let (hash, permit) = permit.hash(password.to_new_password()).await?;
    drop(permit);
    Ok((password, hash))
}
