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
mod tests {
    use super::*;

    #[test]
    fn sixteen_lower_case_hex_characters_pass() {
        let parsed = SessionRef::parse("0123456789abcdef").unwrap();
        assert_eq!(parsed.as_str(), "0123456789abcdef");
        assert_eq!(parsed.to_string(), "0123456789abcdef");
    }

    #[test]
    fn fifteen_characters_fail() {
        assert_eq!(
            SessionRef::parse("0123456789abcde"),
            Err(SessionRefReason::Format)
        );
    }

    #[test]
    fn seventeen_characters_fail() {
        assert_eq!(
            SessionRef::parse("0123456789abcdef0"),
            Err(SessionRefReason::Format)
        );
    }

    #[test]
    fn upper_case_hex_fails() {
        assert_eq!(
            SessionRef::parse("0123456789ABCDEF"),
            Err(SessionRefReason::Format)
        );
    }

    #[test]
    fn a_non_hex_letter_fails() {
        assert_eq!(
            SessionRef::parse("0123456789abcdeg"),
            Err(SessionRefReason::Format)
        );
    }

    #[test]
    fn sixteen_multi_byte_characters_fail() {
        assert_eq!(
            SessionRef::parse(
                "\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}"
            ),
            Err(SessionRefReason::Format)
        );
    }

    #[test]
    fn the_empty_text_fails() {
        assert_eq!(SessionRef::parse(""), Err(SessionRefReason::Format));
    }

    #[test]
    fn the_reason_displays_a_sentence() {
        assert!(!SessionRefReason::Format.to_string().is_empty());
    }
}
