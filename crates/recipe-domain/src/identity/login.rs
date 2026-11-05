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
mod tests;
