//! Secret value objects of the Identity and Access context (design D4).
//!
//! Every type here holds a password, a hash, a token, or a session id.
//! Each type has a hand-written `Debug` that prints `[redacted]`, no `Display`,
//! and no serde derive (`reliability.md` section 3.4). The types solve one problem:
//! a secret cannot reach a log line, an error message, or an audit entry by accident.

use super::password_reason::{PASSWORD_MAX_CHARS, PASSWORD_MIN_CHARS};
use super::{PasswordReason, policy};
use sha2::{Digest, Sha256};
use std::fmt;

/// Number of random bytes of a session token (`data-model.md` section 3.6).
const SESSION_TOKEN_BYTES: usize = 16;

/// Number of random bytes of a CSRF token (`data-model.md` section 3.6).
const CSRF_TOKEN_BYTES: usize = 32;

/// Number of bytes of a SHA-256 digest.
const SESSION_ID_BYTES: usize = 32;

/// Longest PHC string that `PasswordHash::parse` accepts.
const PASSWORD_HASH_MAX_CHARS: usize = 512;

/// Prefix that every Argon2id PHC string starts with (`app_user_hash_format`).
const PASSWORD_HASH_PREFIX: &str = "$argon2id$";

/// Text printed by the `Debug` of every secret.
const REDACTED: &str = "[redacted]";

/// Write the redaction text. Every secret type uses it in its `Debug`.
fn write_redacted(f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(REDACTED)
}

/// Return the lower-case hex text of `bytes`.
fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Decode `text` into exactly `N` bytes. The text must be `2 * N` characters of `[0-9a-f]`.
fn decode_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    let is_lower_hex = |b: &u8| matches!(b, b'0'..=b'9' | b'a'..=b'f');
    let expected = N.checked_mul(2)?;
    if text.len() != expected || !text.as_bytes().iter().all(is_lower_hex) {
        return None;
    }
    let (pairs, _) = text.as_bytes().as_chunks::<2>();
    let bytes = pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    <[u8; N]>::try_from(bytes).ok()
}

/// Password that a user typed in a login form.
///
/// The password has 1 to 128 characters, counted as Unicode scalar values.
/// The type has no policy beyond the length, because an old password can be weak.
#[derive(Clone)]
pub struct Password(String);

impl Password {
    /// Parse a typed password.
    ///
    /// # Errors
    ///
    /// Returns [`PasswordReason::Empty`] for an empty text and
    /// [`PasswordReason::TooLong`] for a text above 128 characters.
    pub fn parse(text: &str) -> Result<Self, PasswordReason> {
        let length = text.chars().count();
        if length == 0 {
            return Err(PasswordReason::Empty);
        }
        if length > PASSWORD_MAX_CHARS {
            return Err(PasswordReason::TooLong);
        }
        Ok(Self(text.to_owned()))
    }

    /// Return the password text. Only the hasher needs it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&NewPassword> for Password {
    fn from(new: &NewPassword) -> Self {
        Self(new.0.clone())
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_redacted(f)
    }
}

/// New password that meets the policy of assumption A23.
///
/// The password has 12 to 128 characters, counted as Unicode scalar values.
/// It is not in the breached-password list.
#[derive(Clone)]
pub struct NewPassword(String);

impl NewPassword {
    /// Parse a new password and check it against the policy.
    ///
    /// # Errors
    ///
    /// Returns [`PasswordReason::TooShort`], [`PasswordReason::TooLong`], or
    /// [`PasswordReason::Breached`].
    pub fn parse(text: &str) -> Result<Self, PasswordReason> {
        let length = text.chars().count();
        if length < PASSWORD_MIN_CHARS {
            return Err(PasswordReason::TooShort);
        }
        if length > PASSWORD_MAX_CHARS {
            return Err(PasswordReason::TooLong);
        }
        if policy::is_breached(text) {
            return Err(PasswordReason::Breached);
        }
        Ok(Self(text.to_owned()))
    }

    /// Wrap a generated one-time password. Only [`OneTimePassword`] calls this.
    pub(super) const fn from_generated(text: String) -> Self {
        Self(text)
    }

    /// Return the password text. Only the hasher needs it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for NewPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_redacted(f)
    }
}

/// Reason why a text is not a password hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordHashReason {
    /// The text is not an Argon2id PHC string of printable ASCII characters.
    Format,
}

impl fmt::Display for PasswordHashReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => f.write_str("a password hash is an Argon2id PHC string"),
        }
    }
}

/// Argon2id hash of a password in PHC string form.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// Parse a PHC string. The text starts with `$argon2id$` and has no white space.
    ///
    /// # Errors
    ///
    /// Returns [`PasswordHashReason::Format`] when the text does not match.
    pub fn parse(text: &str) -> Result<Self, PasswordHashReason> {
        let well_formed = text.starts_with(PASSWORD_HASH_PREFIX)
            && text.len() <= PASSWORD_HASH_MAX_CHARS
            && text.bytes().all(|b| b.is_ascii_graphic());
        if !well_formed {
            return Err(PasswordHashReason::Format);
        }
        Ok(Self(text.to_owned()))
    }

    /// Return the PHC string. Only the hasher and the storage adapter need it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_redacted(f)
    }
}

/// Reason why a text is not a session token, a session id, or a CSRF token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenReason {
    /// The text is not the fixed number of lower-case hex characters.
    Format,
}

impl fmt::Display for TokenReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => {
                f.write_str("a token is a fixed number of characters from 0 to 9 and a to f")
            }
        }
    }
}

/// Random token of a session cookie: 16 bytes, 32 lower-case hex characters.
///
/// The server never stores the token. It stores [`SessionId`] only.
#[derive(Clone)]
pub struct SessionToken([u8; SESSION_TOKEN_BYTES]);

impl SessionToken {
    /// Build a token from random bytes that the application supplies.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; SESSION_TOKEN_BYTES]) -> Self {
        Self(bytes)
    }

    /// Parse the 32 hex characters of a cookie value.
    ///
    /// # Errors
    ///
    /// Returns [`TokenReason::Format`] when the text is not 32 characters of `[0-9a-f]`.
    pub fn parse(text: &str) -> Result<Self, TokenReason> {
        decode_hex(text).map(Self).ok_or(TokenReason::Format)
    }

    /// Return the 32 hex characters for the cookie value.
    #[must_use]
    pub fn to_hex(&self) -> String {
        encode_hex(&self.0)
    }

    /// Return the token bytes. Only [`SessionId::of`] needs them.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; SESSION_TOKEN_BYTES] {
        &self.0
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_redacted(f)
    }
}

/// Identifier of a stored session: the SHA-256 of the token bytes in 64 lower-case hex characters.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Derive the id of a session from its token.
    #[must_use]
    pub fn of(token: &SessionToken) -> Self {
        Self(encode_hex(Sha256::digest(token.as_bytes()).as_slice()))
    }

    /// Parse a stored id of 64 lower-case hex characters.
    ///
    /// # Errors
    ///
    /// Returns [`TokenReason::Format`] when the text is not 64 characters of `[0-9a-f]`.
    pub fn parse(text: &str) -> Result<Self, TokenReason> {
        decode_hex::<SESSION_ID_BYTES>(text)
            .map(|_| Self(text.to_owned()))
            .ok_or(TokenReason::Format)
    }

    /// Return the 64 hex characters for the storage adapter.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_redacted(f)
    }
}

/// Token against cross-site request forgery: 32 bytes, 64 lower-case hex characters.
#[derive(Clone)]
pub struct CsrfToken([u8; CSRF_TOKEN_BYTES]);

impl CsrfToken {
    /// Build a token from random bytes that the application supplies.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; CSRF_TOKEN_BYTES]) -> Self {
        Self(bytes)
    }

    /// Parse the 64 hex characters of a stored token or a form field.
    ///
    /// # Errors
    ///
    /// Returns [`TokenReason::Format`] when the text is not 64 characters of `[0-9a-f]`.
    pub fn parse(text: &str) -> Result<Self, TokenReason> {
        decode_hex(text).map(Self).ok_or(TokenReason::Format)
    }

    /// Return the 64 hex characters for storage and for the page.
    #[must_use]
    pub fn to_hex(&self) -> String {
        encode_hex(&self.0)
    }

    /// Return the token bytes. `recipe-web` compares them in constant time.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; CSRF_TOKEN_BYTES] {
        &self.0
    }
}

impl fmt::Debug for CsrfToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_redacted(f)
    }
}

#[cfg(test)]
mod tests;
