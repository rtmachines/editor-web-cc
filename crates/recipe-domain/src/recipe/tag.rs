//! Tag of a parameter (`reliability.md` section 4.3, design D1).

use std::fmt;

/// Longest number of characters of a parameter tag (`text.tag_max`, `reliability.md` section 7.11).
const TAG_MAX_CHARS: usize = 64;

/// Tag of a parameter. The tag keeps its case.
///
/// The grammar is `[A-Z][A-Z0-9_]{0,63}`. The parser does not fold case.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterTag(String);

/// Reason why a text is not a parameter tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterTagReason {
    /// The text does not match the grammar of a parameter tag.
    Format,
}

impl fmt::Display for ParameterTagReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => f.write_str(
                "a parameter tag is 1 to 64 characters from A to Z, 0 to 9, and underscore. It starts with a letter from A to Z",
            ),
        }
    }
}

impl ParameterTag {
    /// Parse a parameter tag. The check does not fold case.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterTagReason::Format`] when the text does not match the grammar.
    pub fn parse(text: &str) -> Result<Self, ParameterTagReason> {
        let starts_well = text.chars().next().is_some_and(|c| c.is_ascii_uppercase());
        let well_formed = starts_well
            && text.chars().count() <= TAG_MAX_CHARS
            && text
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        if !well_formed {
            return Err(ParameterTagReason::Format);
        }
        Ok(Self(text.to_owned()))
    }

    /// Return the text of the tag.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ParameterTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
