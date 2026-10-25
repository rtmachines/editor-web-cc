//! The key files of `serve` (ADR-0031, ADR-0035 (d), design D4 step 2).
//!
//! A key file holds one key on each line as 64 lower-case hex characters. The first line is the
//! newest key. The check refuses a carriage return anywhere, as the secret files of ADR-0031 do.
//! `str::lines` would hide a `\r\n` line end, so the check looks for the character first.

use std::fmt;
use std::path::Path;

use anyhow::{Result, anyhow};
use recipe_domain::identity::{KeyRing, KeyRingReason};

/// Why the text of a key file is not a key ring.
///
/// The type solves one problem: an error about a key file must never show a part of the key.
/// No variant holds a part of the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyFileFault {
    /// The file holds a carriage return anywhere.
    CarriageReturn,
    /// A line breaks the grammar of a key line.
    Ring(KeyRingReason),
}

impl fmt::Display for KeyFileFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CarriageReturn => f.write_str("holds a carriage return"),
            Self::Ring(reason) => write!(f, "is not a key file: {reason}"),
        }
    }
}

/// Parse the text of a key file into a key ring.
///
/// # Errors
///
/// Returns [`KeyFileFault::CarriageReturn`] for a carriage return anywhere. Returns
/// [`KeyFileFault::Ring`] for an empty file, an empty line, a line of another length, or a
/// character outside `0-9a-f`.
pub fn parse_key_file(text: &str) -> Result<KeyRing, KeyFileFault> {
    if text.contains('\r') {
        return Err(KeyFileFault::CarriageReturn);
    }
    KeyRing::parse(text).map_err(KeyFileFault::Ring)
}

/// Read and check the key file of the configuration key `key`.
///
/// The error names the key. It never shows the path or a part of the content.
///
/// # Errors
///
/// Returns an error when the file cannot be read, is not UTF-8, or breaks the rule of
/// [`parse_key_file`].
pub fn read_key_file(key: &str, path: &Path) -> Result<KeyRing> {
    let text = std::fs::read_to_string(path)
        .map_err(|fault| anyhow!("the file of {key} cannot be read: {}", fault.kind()))?;
    parse_key_file(&text).map_err(|fault| anyhow!("the file of {key} {fault}"))
}

#[cfg(test)]
mod tests;
