//! Password of a PLC connection (ADR-0154, design decision D2).

use std::fmt;

use zeroize::Zeroize;

use super::text::ConnectionTextReason;

/// Highest number of characters of a password.
const MAX_PASSWORD_LENGTH: usize = 128;

/// Password of a PLC connection: 1 to 128 characters.
///
/// A password is a secret. The type has no `Display`, no `Serialize`, no `Clone`, and no
/// `PartialEq`, because a secret gets no compare that stops at the first difference.
/// Its `Debug` hides the text. It clears its buffer on drop.
/// A password permits control characters, because a PLC defines its own rules.
pub struct PlcPassword(String);

impl PlcPassword {
    /// Parse a password from the text of a form field or from a decrypted value.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectionTextReason::Empty`] for an empty text.
    /// Returns [`ConnectionTextReason::TooLong`] for a text of more than 128 characters.
    pub fn parse(text: &str) -> Result<Self, ConnectionTextReason> {
        if text.is_empty() {
            return Err(ConnectionTextReason::Empty);
        }
        if text.chars().count() > MAX_PASSWORD_LENGTH {
            return Err(ConnectionTextReason::TooLong {
                max: MAX_PASSWORD_LENGTH,
            });
        }
        Ok(Self(text.to_owned()))
    }

    /// Return the password text.
    ///
    /// Only two callers may use this accessor. The cipher adapter calls it to
    /// seal the password. The transfer answer of the connection calls it to
    /// hand the password to the operator of the machine. Any other caller
    /// leaks the secret.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PlcPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PlcPassword(<redacted>)")
    }
}

impl Drop for PlcPassword {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests;
