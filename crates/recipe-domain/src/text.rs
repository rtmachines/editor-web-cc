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

/// Largest number of characters of a long text (`text.description_max`, `reliability.md` section 7.11).
pub const LONG_TEXT_MAX: usize = 4000;

/// A long text, such as a description or a change note (`reliability.md` section 4.3).
///
/// The text is not empty after trimming, has at most [`LONG_TEXT_MAX`] characters,
/// and has no control character except a line end. A line end is LF or CRLF.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LongText(String);

/// Reason why a text is not a valid long text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongTextReason {
    /// The text is empty after trimming.
    Empty,
    /// The text has more than [`LONG_TEXT_MAX`] characters.
    TooLong,
    /// The text has a control character other than a line end.
    ControlCharacter,
}

impl fmt::Display for LongTextReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the text is empty"),
            Self::TooLong => write!(f, "the text has more than {LONG_TEXT_MAX} characters"),
            Self::ControlCharacter => {
                f.write_str("the text has a control character other than a line end")
            }
        }
    }
}

impl LongText {
    /// Parse a long text. The check does not change the input.
    ///
    /// # Errors
    ///
    /// Returns [`LongTextReason::Empty`] for a blank text, [`LongTextReason::TooLong`]
    /// for more than [`LONG_TEXT_MAX`] characters, and [`LongTextReason::ControlCharacter`]
    /// for a control character other than a line end.
    pub fn parse(input: &str) -> Result<Self, LongTextReason> {
        if input.trim().is_empty() {
            return Err(LongTextReason::Empty);
        }
        if input.chars().count() > LONG_TEXT_MAX {
            return Err(LongTextReason::TooLong);
        }
        if !has_only_line_ends_as_control(input) {
            return Err(LongTextReason::ControlCharacter);
        }
        Ok(Self(input.to_owned()))
    }

    /// Return the stored text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LongText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Return whether every control character of the input is part of a line end.
fn has_only_line_ends_as_control(input: &str) -> bool {
    let mut chars = input.chars().peekable();
    while let Some(current) = chars.next() {
        let is_line_end = current == '\n' || (current == '\r' && chars.peek() == Some(&'\n'));
        if current.is_control() && !is_line_end {
            return false;
        }
    }
    true
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

    #[test]
    fn long_text_of_4000_multi_byte_characters_passes() {
        let text = "é".repeat(4000);
        assert!(LongText::parse(&text).is_ok());
    }

    #[test]
    fn long_text_of_4001_characters_fails_with_too_long() {
        let text = "a".repeat(4001);
        assert_eq!(LongText::parse(&text), Err(LongTextReason::TooLong));
    }

    #[test]
    fn long_text_with_lf_passes() {
        assert!(LongText::parse("a\nb").is_ok());
    }

    #[test]
    fn long_text_with_crlf_passes_and_keeps_the_input() {
        let parsed = LongText::parse("a\r\nb").expect("valid text");
        assert_eq!(parsed.as_str(), "a\r\nb");
    }

    #[test]
    fn long_text_of_blank_characters_fails_with_empty() {
        assert_eq!(LongText::parse("   "), Err(LongTextReason::Empty));
        assert_eq!(LongText::parse(""), Err(LongTextReason::Empty));
    }

    #[test]
    fn long_text_with_lone_cr_fails_with_control_character() {
        assert_eq!(
            LongText::parse("a\rb"),
            Err(LongTextReason::ControlCharacter)
        );
        assert_eq!(
            LongText::parse("a\r"),
            Err(LongTextReason::ControlCharacter)
        );
    }

    #[test]
    fn long_text_with_tab_fails_with_control_character() {
        assert_eq!(
            LongText::parse("a\tb"),
            Err(LongTextReason::ControlCharacter)
        );
    }
}
