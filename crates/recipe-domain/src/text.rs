//! Text value objects of the domain (`reliability.md` section 4.3).

use std::fmt;

/// Largest number of characters of a name (`text.name_max`, `reliability.md` section 7.11).
pub const NAME_MAX: usize = 200;

/// A name of a machine, a unit, or a recipe (`reliability.md` section 4.3).
///
/// The text is not empty after trimming, has at most [`NAME_MAX`] characters,
/// and has no control character. The stored text keeps its spaces.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Text(String);

/// Reason why a text is not a valid name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextReason {
    /// The text is empty after trimming.
    Empty,
    /// The text has more than [`NAME_MAX`] characters.
    TooLong,
    /// The text has a control character.
    ControlCharacter,
}

impl fmt::Display for TextReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the text is empty"),
            Self::TooLong => write!(f, "the text has more than {NAME_MAX} characters"),
            Self::ControlCharacter => f.write_str("the text has a control character"),
        }
    }
}

impl Text {
    /// Parse a name. The check does not change the input.
    ///
    /// # Errors
    ///
    /// Returns [`TextReason::Empty`] for a blank text, [`TextReason::TooLong`]
    /// for more than [`NAME_MAX`] characters, and [`TextReason::ControlCharacter`]
    /// for a control character.
    pub fn parse(input: &str) -> Result<Self, TextReason> {
        if input.trim().is_empty() {
            return Err(TextReason::Empty);
        }
        if input.chars().count() > NAME_MAX {
            return Err(TextReason::TooLong);
        }
        if input.chars().any(char::is_control) {
            return Err(TextReason::ControlCharacter);
        }
        Ok(Self(input.to_owned()))
    }

    /// Return the stored text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_of_one_character_passes() {
        assert!(Text::parse("a").is_ok());
    }

    #[test]
    fn name_of_200_characters_passes_even_with_more_bytes() {
        let name = "é".repeat(200);
        assert!(Text::parse(&name).is_ok());
    }

    #[test]
    fn name_of_201_characters_fails_with_too_long() {
        let name = "a".repeat(201);
        assert_eq!(Text::parse(&name), Err(TextReason::TooLong));
    }

    #[test]
    fn empty_name_fails_with_empty() {
        assert_eq!(Text::parse(""), Err(TextReason::Empty));
    }

    #[test]
    fn blank_name_fails_with_empty() {
        assert_eq!(Text::parse("   "), Err(TextReason::Empty));
    }

    #[test]
    fn tab_inside_name_fails_with_control_character() {
        assert_eq!(Text::parse("a\tb"), Err(TextReason::ControlCharacter));
    }

    #[test]
    fn bell_inside_name_fails_with_control_character() {
        assert_eq!(Text::parse("a\u{7}"), Err(TextReason::ControlCharacter));
    }

    #[test]
    fn as_str_returns_the_input_without_trimming() {
        let parsed = Text::parse("  a b ").expect("valid text");
        assert_eq!(parsed.as_str(), "  a b ");
        assert_eq!(parsed.to_string(), "  a b ");
    }
}
