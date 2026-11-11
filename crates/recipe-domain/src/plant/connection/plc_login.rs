//! Login of a PLC connection (ADR-0154, design decision D2).

use std::fmt;

use super::text::{ConnectionTextReason, check_text};

/// Highest number of characters of a login.
const MAX_LOGIN_LENGTH: usize = 64;

/// Login of a PLC connection: 1 to 64 characters without a control character.
///
/// A login is not a secret. The page and the audit entry show it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlcLogin(String);

impl PlcLogin {
    /// Parse a login such as `adapter`.
    ///
    /// # Errors
    ///
    /// Returns the [`ConnectionTextReason`] that names the first broken rule.
    pub fn parse(text: &str) -> Result<Self, ConnectionTextReason> {
        check_text(text, MAX_LOGIN_LENGTH)?;
        Ok(Self(text.to_owned()))
    }

    /// Return the login as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlcLogin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
