//! Reason type and check shared by the text value objects of a PLC connection.

use std::fmt;

/// Reason why a text is not a valid text field of a PLC connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionTextReason {
    /// The text is empty.
    Empty,
    /// The text has more characters than the field permits.
    TooLong {
        /// Highest permitted number of characters.
        max: usize,
    },
    /// The text has a control character.
    ControlCharacter,
}

impl fmt::Display for ConnectionTextReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a text is not empty"),
            Self::TooLong { max } => write!(f, "a text has at most {max} characters"),
            Self::ControlCharacter => f.write_str("a text has no control character"),
        }
    }
}

/// Check that a text has 1 to `max` characters and no control character.
pub(super) fn check_text(text: &str, max: usize) -> Result<(), ConnectionTextReason> {
    if text.is_empty() {
        return Err(ConnectionTextReason::Empty);
    }
    if text.chars().count() > max {
        return Err(ConnectionTextReason::TooLong { max });
    }
    if text.chars().any(char::is_control) {
        return Err(ConnectionTextReason::ControlCharacter);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
