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
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn lower_case_input_is_folded_to_upper_case() {
        assert_eq!(
            ReferenceNumber::parse("r-100").map(|r| r.as_str().to_owned()),
            Ok("R-100".to_owned())
        );
    }

    #[test]
    fn fifty_characters_pass() {
        let text = "A".repeat(50);
        assert!(ReferenceNumber::parse(&text).is_ok());
    }

    #[test]
    fn fifty_one_characters_fail() {
        let text = "A".repeat(51);
        assert_eq!(
            ReferenceNumber::parse(&text),
            Err(ReferenceNumberReason::Format)
        );
    }

    #[test]
    fn invalid_texts_fail_with_format() {
        for text in ["", "-R1", "_R1", "R 1", "Rß", "R.1"] {
            assert_eq!(
                ReferenceNumber::parse(text),
                Err(ReferenceNumberReason::Format),
                "{text}"
            );
        }
    }

    proptest! {
        #[test]
        fn parse_and_as_str_round_trip(text in "[A-Z0-9][A-Z0-9_-]{0,49}") {
            let reference = ReferenceNumber::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(reference.as_str(), text.as_str());
            prop_assert_eq!(reference.to_string(), text);
        }

        #[test]
        fn lower_case_input_parses_to_upper_case(text in "[A-Z0-9][A-Z0-9_-]{0,49}") {
            let reference = ReferenceNumber::parse(&text.to_ascii_lowercase()).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(reference.as_str(), text.as_str());
        }
    }
}
