//! Password policy: the breached-password list and the one-time password (design D4).

use super::NewPassword;
use std::collections::HashSet;
use std::fmt;
use std::sync::LazyLock;

/// The breached-password list, built by `tools/breached-passwords/filter.py`.
///
/// The file header names the source path, the commit, the license, and the line count.
const BREACHED_LIST: &str = include_str!("../../data/breached-passwords.txt");

/// Characters of a one-time password: digits and capital letters without `0`, `O`, `1`, `I`, and `l`.
///
/// The alphabet has 32 characters, so one random byte maps to one character without bias.
const ONE_TIME_ALPHABET: &str = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// Number of random bytes, and of characters, of a one-time password.
///
/// The password has one character for each byte, so the byte count sets the password length.
pub const ONE_TIME_BYTES: usize = 20;

/// Number of characters in [`ONE_TIME_ALPHABET`].
const ONE_TIME_ALPHABET_LEN: u8 = 32;

/// The set of listed passwords. The first use builds it.
static BREACHED: LazyLock<HashSet<&'static str>> = LazyLock::new(|| parse_list(BREACHED_LIST));

/// Return the passwords of a list text. A line that starts with `#` and a blank line do not count.
fn parse_list(text: &str) -> HashSet<&str> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// Return true when the list holds `text`. The match is exact and case-sensitive.
pub(crate) fn is_breached(text: &str) -> bool {
    BREACHED.contains(text)
}

/// Load the breached-password list and return the number of passwords in it.
///
/// The application calls this function at startup, so a request never pays for the load.
#[must_use]
pub fn breached_password_count() -> usize {
    BREACHED.len()
}

/// Generated password that an administrator hands to a user once (`CreateUser`, `ResetPassword`).
///
/// The type solves one problem: the one-time password is never a plain `String`, so it cannot
/// reach a log line by accident. It has 20 characters of a 32-character alphabet.
/// The alphabet has no `0`, `O`, `1`, `I`, or `l`, so a person can read the text without doubt.
#[derive(Clone)]
pub struct OneTimePassword(String);

impl OneTimePassword {
    /// Build the password from 20 random bytes that the application supplies.
    ///
    /// Each byte selects the character at the index `byte % 32` of the alphabet
    /// `23456789ABCDEFGHJKLMNPQRSTUVWXYZ`.
    #[must_use]
    pub fn from_bytes(bytes: [u8; ONE_TIME_BYTES]) -> Self {
        let text = bytes
            .iter()
            .map(|byte| {
                let index = usize::from(byte % ONE_TIME_ALPHABET_LEN);
                // The index is below 32, so the lookup always succeeds.
                ONE_TIME_ALPHABET
                    .chars()
                    .nth(index)
                    .unwrap_or(FIRST_ALPHABET_CHAR)
            })
            .collect();
        Self(text)
    }

    /// Return the password text. The command result shows it once to the administrator.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Turn the password into a [`NewPassword`] for hashing.
    ///
    /// The conversion skips the breached-password list. The bytes are random, so a listed
    /// text has a negligible chance, and the length and the alphabet always meet the policy.
    #[must_use]
    pub fn to_new_password(&self) -> NewPassword {
        NewPassword::from_generated(self.0.clone())
    }
}

/// First character of [`ONE_TIME_ALPHABET`]. The lookup falls back to it, which never happens.
const FIRST_ALPHABET_CHAR: char = '2';

impl fmt::Debug for OneTimePassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

#[cfg(test)]
mod tests;
