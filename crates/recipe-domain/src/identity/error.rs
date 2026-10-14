//! The error type of the Identity and Access context.

use super::{Login, PasswordReason};
use crate::error_codes::ErrorCode;

/// Cause of a failed credential check.
///
/// The cause is for the event log and for the `security_event` kind only.
/// The user always gets the one message of `E-IDN-003` (assumption A42).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialFailure {
    /// No user has the typed login.
    UnknownLogin,
    /// The user exists and the password is wrong.
    WrongPassword,
}

impl CredentialFailure {
    /// Return the stable name of the cause for the event log.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownLogin => "unknown_login",
            Self::WrongPassword => "wrong_password",
        }
    }
}

/// Error of the Identity and Access context (`reliability.md` section 1.1).
///
/// The enum has one variant for each code in the range `E-IDN-001` to `E-IDN-010`.
/// No variant carries a password, a hash, a token, or a cookie value.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    /// A user with this login exists (`E-IDN-001`).
    #[error("Login {login} is already in use.")]
    DuplicateLogin {
        /// The login that exists.
        login: Login,
    },
    /// The change would leave the system without an active administrator (`E-IDN-002`).
    #[error("{login} is the last active administrator. Add another administrator first.")]
    LastAdministrator {
        /// The login of the last administrator.
        login: Login,
    },
    /// The login or the password is wrong (`E-IDN-003`).
    #[error("Login or password is not correct.")]
    InvalidCredentials {
        /// The exact cause, for the event log only.
        cause: CredentialFailure,
    },
    /// The user is inactive (`E-IDN-004`). The user sees the message of `E-IDN-003`.
    #[error("Login or password is not correct.")]
    UserInactive,
    /// The attempt is inside the delay of the login throttle (`E-IDN-005`).
    #[error("Too many failed logins from this address. Try again in {retry_after} seconds.")]
    LoginThrottled {
        /// The whole seconds until the next attempt can pass.
        retry_after: u64,
    },
    /// The new password breaks the policy (`E-IDN-006`).
    #[error(
        "The password is not accepted: {reason}. Use 12 to 128 characters that are not in the breached-password list."
    )]
    PasswordPolicy {
        /// The rule that the password breaks.
        reason: PasswordReason,
    },
    /// The session must change its password first (`E-IDN-007`).
    #[error("Change your password first.")]
    MustChangePassword,
    /// The current password in a password change is wrong (`E-IDN-008`).
    #[error("The current password is not correct.")]
    InvalidCurrentPassword,
    /// The user is already active (`E-IDN-009`).
    #[error("User {login} is already active.")]
    UserAlreadyActive {
        /// The login of the user.
        login: Login,
    },
    /// The user is already inactive (`E-IDN-010`).
    #[error("User {login} is already inactive.")]
    UserAlreadyInactive {
        /// The login of the user.
        login: Login,
    },
}

impl From<PasswordReason> for IdentityError {
    fn from(reason: PasswordReason) -> Self {
        Self::PasswordPolicy { reason }
    }
}

impl IdentityError {
    /// Return the stable error code of this error.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::DuplicateLogin { .. } => ErrorCode::IDN_001,
            Self::LastAdministrator { .. } => ErrorCode::IDN_002,
            Self::InvalidCredentials { .. } => ErrorCode::IDN_003,
            Self::UserInactive => ErrorCode::IDN_004,
            Self::LoginThrottled { .. } => ErrorCode::IDN_005,
            Self::PasswordPolicy { .. } => ErrorCode::IDN_006,
            Self::MustChangePassword => ErrorCode::IDN_007,
            Self::InvalidCurrentPassword => ErrorCode::IDN_008,
            Self::UserAlreadyActive { .. } => ErrorCode::IDN_009,
            Self::UserAlreadyInactive { .. } => ErrorCode::IDN_010,
        }
    }
}

#[cfg(test)]
mod tests;
