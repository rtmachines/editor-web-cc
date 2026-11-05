//! Reference number of a recipe (`domain-model.md` section 2.2, A27).

use std::fmt;

/// Longest number of characters of a reference number (`reliability.md` section 4.3).
const MAX_CHARS: usize = 50;

/// Reference number of a recipe, folded to upper case.
///
/// The grammar is `[A-Z0-9][A-Z0-9_-]{0,49}` after upper-case folding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReferenceNumber(String);

/// Reason why a text is not a reference number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceNumberReason {
    /// The text does not match the grammar of a reference number.
    Format,
}

impl fmt::Display for ReferenceNumberReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => f.write_str(
                "a reference number is 1 to 50 characters from A to Z, 0 to 9, underscore, and hyphen. The first character is a letter or a digit",
            ),
        }
    }
}

impl ReferenceNumber {
    /// Parse a reference number. ASCII letters fold to upper case.
    ///
    /// # Errors
    ///
    /// Returns [`ReferenceNumberReason::Format`] when the folded text does not match the grammar.
    pub fn parse(text: &str) -> Result<Self, ReferenceNumberReason> {
        let folded = text.to_ascii_uppercase();
        let starts_well = folded
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
        let well_formed = starts_well
            && folded.chars().count() <= MAX_CHARS
            && folded
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-');
        if !well_formed {
            return Err(ReferenceNumberReason::Format);
        }
        Ok(Self(folded))
    }

    /// Return the folded text of the reference number.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ReferenceNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
