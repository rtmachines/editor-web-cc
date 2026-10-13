//! Login name of a user (`reliability.md` section 4.3, design D2).

use std::fmt;

/// Shortest number of characters of a login (`reliability.md` section 4.3).
const LOGIN_MIN_CHARS: usize = 3;

/// Longest number of characters of a login (`reliability.md` section 4.3).
const LOGIN_MAX_CHARS: usize = 64;

/// Login name of a user, folded to lower case.
///
/// The grammar is `[a-z0-9._-]{3,64}` after ASCII lower-case folding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Login(String);

/// Reason why a text is not a login name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginReason {
    /// The text does not match the grammar of a login.
    Format,
}

impl fmt::Display for LoginReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => f.write_str(
                "a login is 3 to 64 characters from a to z, 0 to 9, period, underscore, and hyphen",
            ),
        }
    }
}

impl Login {
    /// Parse a login name. ASCII letters fold to lower case.
    ///
    /// # Errors
    ///
    /// Returns [`LoginReason::Format`] when the folded text does not match the grammar.
    pub fn parse(text: &str) -> Result<Self, LoginReason> {
        let folded = text.to_ascii_lowercase();
        let length = folded.chars().count();
        let well_formed = (LOGIN_MIN_CHARS..=LOGIN_MAX_CHARS).contains(&length)
            && folded.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')
            });
        if !well_formed {
            return Err(LoginReason::Format);
        }
        Ok(Self(folded))
    }

    /// Return the folded text of the login.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Login {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn upper_case_letters_fold_to_lower_case() {
        assert_eq!(
            Login::parse("Admin.User").map(|login| login.as_str().to_owned()),
            Ok("admin.user".to_owned())
        );
    }

    #[test]
    fn three_characters_pass() {
        assert!(Login::parse("abc").is_ok());
    }

    #[test]
    fn sixty_four_characters_pass() {
        let text = "a".repeat(64);
        assert!(Login::parse(&text).is_ok());
    }

    #[test]
    fn two_characters_fail_with_format() {
        assert_eq!(Login::parse("ab"), Err(LoginReason::Format));
    }

    #[test]
    fn sixty_five_characters_fail_with_format() {
        let text = "a".repeat(65);
        assert_eq!(Login::parse(&text), Err(LoginReason::Format));
    }

    #[test]
    fn text_with_a_space_fails_with_format() {
        assert_eq!(Login::parse("a b c"), Err(LoginReason::Format));
    }

    #[test]
    fn non_ascii_letter_fails_with_format() {
        assert_eq!(Login::parse("ł"), Err(LoginReason::Format));
        assert_eq!(Login::parse("łab"), Err(LoginReason::Format));
    }

    proptest! {
        #[test]
        fn parse_and_display_round_trip(text in "[a-z0-9._-]{3,64}") {
            let login = Login::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(login.as_str(), text.as_str());
            prop_assert_eq!(login.to_string(), text);
        }

        #[test]
        fn upper_case_input_parses_to_lower_case(text in "[a-z0-9._-]{3,64}") {
            let login = Login::parse(&text.to_ascii_uppercase()).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(login.as_str(), text.as_str());
        }
    }
}
