//! Session reference of an audit entry (`data-model.md` 3.15, constraint `audit_log_session_ref_format`).

use crate::identity::SessionId;
use std::fmt;

/// Number of characters of a session reference (`log.session_ref_length`, `reliability.md` 7.11).
const SESSION_REF_CHARS: usize = 16;

/// The first 16 lower-case hex characters of the session id hash.
///
/// An audit entry stores this prefix and never the full session id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionRef(String);

/// Reason why a text is not a session reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionRefReason {
    /// The text is not 16 lower-case hex characters.
    Format,
}

impl fmt::Display for SessionRefReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => {
                f.write_str("a session reference is 16 characters from 0 to 9 and a to f")
            }
        }
    }
}

impl SessionRef {
    /// Parse a session reference. The text must be exactly 16 characters of `[0-9a-f]`.
    ///
    /// # Errors
    ///
    /// Returns [`SessionRefReason::Format`] when the text does not match.
    pub fn parse(text: &str) -> Result<Self, SessionRefReason> {
        let well_formed = text.len() == SESSION_REF_CHARS
            && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if !well_formed {
            return Err(SessionRefReason::Format);
        }
        Ok(Self(text.to_owned()))
    }

    /// Build the reference of a session: the first 16 characters of its id.
    #[must_use]
    pub fn of(id: &SessionId) -> Self {
        Self(id.as_str().chars().take(SESSION_REF_CHARS).collect())
    }

    /// Return the 16 characters of the reference.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
