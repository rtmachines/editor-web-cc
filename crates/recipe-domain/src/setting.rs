//! Stored settings of the server (`data-model.md` section 3.17).
//!
//! The table `app_setting` holds one row for each setting key. This module holds the
//! identifier of a row, the key, and the value. The key and the value follow the checks
//! `app_setting_key_format` and `app_setting_value_length` of the table.

use std::fmt;

use sha2::Digest;
use uuid::Uuid;

use crate::export::Sha256;

/// Largest number of characters of a setting value (`setting.value_max`, `reliability.md` section 7.11).
pub const SETTING_VALUE_MAX: usize = 1000;

/// Prefix of the fingerprint of a secret value (`data-model.md` section 3.17).
const FINGERPRINT_PREFIX: &str = "sha256:";

/// Stable identity of a setting row (`data-model.md` section 3.17).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SettingId(Uuid);

impl From<Uuid> for SettingId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl SettingId {
    /// Return the UUID of this identifier.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for SettingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Key of a setting, for example `approval.separation_of_duties` (`data-model.md` section 3.17).
///
/// The key has two or more segments that a dot separates. A segment starts with `a` to `z`.
/// The other characters of a segment are `a` to `z`, `0` to `9`, or an underscore.
/// This grammar is the check `app_setting_key_format` of the table.
/// The type solves one problem: the code never holds a key that the database refuses.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SettingKey(String);

/// Reason why a text is not a valid setting key (ADR-0025).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKeyReason {
    /// The text does not follow the grammar of a setting key.
    Format,
}

impl fmt::Display for SettingKeyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => f.write_str(
                "a setting key has two or more dot-separated segments that start with a to z and hold a to z, 0 to 9, or underscore",
            ),
        }
    }
}

/// Return whether the text is one segment of a setting key.
fn is_key_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

impl SettingKey {
    /// Parse a setting key. The check does not change the input.
    ///
    /// # Errors
    ///
    /// Returns [`SettingKeyReason::Format`] when the text does not follow the grammar.
    pub fn parse(input: &str) -> Result<Self, SettingKeyReason> {
        if input.contains('.') && input.split('.').all(is_key_segment) {
            Ok(Self(input.to_owned()))
        } else {
            Err(SettingKeyReason::Format)
        }
    }

    /// Return the stored text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SettingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Canonical text of the value of a setting (`data-model.md` section 3.17).
///
/// The text has at most [`SETTING_VALUE_MAX`] characters. The empty text is valid.
/// The length rule is the check `app_setting_value_length` of the table.
/// The text has no U+0000 character, because a PostgreSQL `text` column refuses it.
/// The type solves one problem: the code never holds a value that the database refuses.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SettingValue(String);

/// Reason why a text is not a valid setting value (ADR-0025).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingValueReason {
    /// The text has more than [`SETTING_VALUE_MAX`] characters.
    TooLong,
    /// The text holds the character U+0000, which a PostgreSQL `text` column refuses.
    NulCharacter,
}

impl fmt::Display for SettingValueReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => write!(
                f,
                "a setting value has more than {SETTING_VALUE_MAX} characters"
            ),
            Self::NulCharacter => f.write_str("a setting value has a NUL character"),
        }
    }
}

impl SettingValue {
    /// Parse a setting value. The check does not change the input.
    ///
    /// # Errors
    ///
    /// Returns [`SettingValueReason::TooLong`] for more than [`SETTING_VALUE_MAX`] characters.
    /// Returns [`SettingValueReason::NulCharacter`] when the text holds U+0000.
    pub fn parse(input: &str) -> Result<Self, SettingValueReason> {
        if input.contains('\0') {
            return Err(SettingValueReason::NulCharacter);
        }
        if input.chars().count() > SETTING_VALUE_MAX {
            return Err(SettingValueReason::TooLong);
        }
        Ok(Self(input.to_owned()))
    }

    /// Build the fingerprint of a secret text.
    ///
    /// The fingerprint is `sha256:` and the 64 lower-case hexadecimal characters of the
    /// SHA-256 hash of the UTF-8 bytes of the text (ADR-0063). The stored row holds the
    /// fingerprint and never the secret text. The result is always a valid value.
    /// With no webhook, the caller stores the empty text and does not call this function (ADR-0063 (c)).
    #[must_use]
    pub fn fingerprint(text: &str) -> Self {
        let digest: [u8; 32] = sha2::Sha256::digest(text.as_bytes()).into();
        Self(format!("{FINGERPRINT_PREFIX}{}", Sha256::from(digest)))
    }

    /// Return the stored text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SettingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
