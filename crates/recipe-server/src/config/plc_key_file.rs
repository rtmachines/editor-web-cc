//! The key file of the PLC credential key (ADR-0154 (g), design D5).
//!
//! The file holds exactly one key: 64 lower-case hex characters and one optional final line feed.
//! There is no key ring, because ADR-0154 has no key rotation. The check refuses a carriage
//! return anywhere, as the other secret files of ADR-0031 do. No buffer of this module outlives
//! the call without clearing itself, and no fault shows a part of the file.

use std::fmt;
use std::path::Path;

use anyhow::{Result, anyhow};
use zeroize::Zeroizing;

use crate::credential_cipher::{CipherKey, KEY_LENGTH};

/// The number of hex characters of the key.
const HEX_LENGTH: usize = KEY_LENGTH * 2;

/// The key of the cipher that seals the PLC password, as the key file gives it.
///
/// The type solves one problem: the loaded key must not leak through a log line, a `Debug` text,
/// or a copy that stays in memory. The wrapped [`CipherKey`] clears itself on drop. The type has
/// no `Display` and no `Clone`. The wiring takes the key once with [`Self::into_cipher_key`].
pub struct PlcCredentialKey(CipherKey);

impl PlcCredentialKey {
    /// Give up the key to the builder of the cipher.
    #[must_use]
    pub fn into_cipher_key(self) -> CipherKey {
        self.0
    }
}

impl fmt::Debug for PlcCredentialKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PlcCredentialKey(<redacted>)")
    }
}

/// Why the text of a key file is not a PLC credential key.
///
/// The type solves one problem: an error about the key file must never show a part of the key.
/// No variant holds a part of the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlcKeyFault {
    /// The file holds a carriage return anywhere.
    CarriageReturn,
    /// The file is empty or holds only a line feed.
    Empty,
    /// The file holds a line after the key line.
    SecondLine,
    /// The key line does not have 64 characters.
    Length,
    /// The key line holds a character outside `0-9a-f`.
    NotLowerHex,
}

impl fmt::Display for PlcKeyFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CarriageReturn => "holds a carriage return",
            Self::Empty => "holds no key",
            Self::SecondLine => "holds more than one line",
            Self::Length => "is not a key of 64 characters",
            Self::NotLowerHex => "holds a character outside 0-9 and a-f",
        })
    }
}

/// Parse the text of the key file into the PLC credential key.
///
/// The text is 64 lower-case hex characters and one optional final line feed. Nothing else is
/// valid.
///
/// # Errors
///
/// Returns [`PlcKeyFault::CarriageReturn`] for a carriage return anywhere. Returns
/// [`PlcKeyFault::Empty`] for an empty file. Returns [`PlcKeyFault::SecondLine`] for a line feed
/// before the end of the text. Returns [`PlcKeyFault::Length`] for a line of another length. Returns
/// [`PlcKeyFault::NotLowerHex`] for a character outside `0-9a-f`.
pub fn parse_plc_key_file(text: &str) -> Result<PlcCredentialKey, PlcKeyFault> {
    if text.contains('\r') {
        return Err(PlcKeyFault::CarriageReturn);
    }
    let line = text.strip_suffix('\n').unwrap_or(text);
    if line.is_empty() {
        return Err(PlcKeyFault::Empty);
    }
    if line.contains('\n') {
        return Err(PlcKeyFault::SecondLine);
    }
    if line.chars().count() != HEX_LENGTH {
        return Err(PlcKeyFault::Length);
    }
    if !line.bytes().all(is_lower_hex) {
        return Err(PlcKeyFault::NotLowerHex);
    }
    let mut bytes = Zeroizing::new([0_u8; KEY_LENGTH]);
    let (pairs, _rest) = line.as_bytes().as_chunks::<2>();
    for (slot, &[high, low]) in bytes.iter_mut().zip(pairs) {
        *slot = byte_of(high, low).ok_or(PlcKeyFault::NotLowerHex)?;
    }
    Ok(PlcCredentialKey(CipherKey::from_bytes(bytes)))
}

/// Whether `digit` is one of `0-9a-f`.
const fn is_lower_hex(digit: u8) -> bool {
    matches!(digit, b'0'..=b'9' | b'a'..=b'f')
}

/// The value of one hex digit, or `None` for a character outside `0-9a-f`.
fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => digit.checked_sub(b'0'),
        b'a'..=b'f' => digit.checked_sub(b'a')?.checked_add(10),
        _ => None,
    }
}

/// The byte that two hex digits give.
fn byte_of(high: u8, low: u8) -> Option<u8> {
    nibble(high)?.checked_mul(16)?.checked_add(nibble(low)?)
}

/// Read and check the key file of the configuration key `key`.
///
/// The error names the key. It never shows the path or a part of the content.
///
/// # Errors
///
/// Returns an error when the file cannot be read, is not UTF-8, or breaks the rule of
/// [`parse_plc_key_file`].
pub fn read_plc_key_file(key: &str, path: &Path) -> Result<PlcCredentialKey> {
    let bytes = std::fs::read(path)
        .map(Zeroizing::new)
        .map_err(|fault| anyhow!("the file of {key} cannot be read: {}", fault.kind()))?;
    let text = std::str::from_utf8(&bytes).map_err(|_invalid| {
        anyhow!(
            "the file of {key} cannot be read: {}",
            std::io::ErrorKind::InvalidData
        )
    })?;
    parse_plc_key_file(text).map_err(|fault| anyhow!("the file of {key} {fault}"))
}

#[cfg(test)]
mod tests;
