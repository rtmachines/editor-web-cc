//! Reasons why a password is not accepted (`E-IDN-006`, assumption A23).

use std::fmt;

/// Fewest characters of a new password (assumption A23).
pub(crate) const PASSWORD_MIN_CHARS: usize = 12;

/// Most characters of a password (assumption A23).
pub(crate) const PASSWORD_MAX_CHARS: usize = 128;

/// Reason why a text is not an accepted password.
///
/// The enum is the `reason` field of `IdentityError::PasswordPolicy`.
/// It is not an error type and has no code. It never holds the password.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordReason {
    /// The password has no character.
    Empty,
    /// The new password has fewer characters than the policy allows.
    TooShort,
    /// The password has more characters than the policy allows.
    TooLong,
    /// The new password is in the breached-password list.
    Breached,
}

impl fmt::Display for PasswordReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("it is empty"),
            Self::TooShort => write!(f, "it has fewer than {PASSWORD_MIN_CHARS} characters"),
            Self::TooLong => write!(f, "it has more than {PASSWORD_MAX_CHARS} characters"),
            Self::Breached => f.write_str("it is in the breached-password list"),
        }
    }
}
