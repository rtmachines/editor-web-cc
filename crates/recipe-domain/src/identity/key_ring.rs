//! Key file of a signing secret (design D4, ADR-0035 d, `reliability.md` 7.1).

use std::fmt;

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Number of bytes of one key.
const KEY_BYTES: usize = 32;

/// Number of hex characters of one key line.
const KEY_HEX_CHARS: usize = 64;

/// Radix of the key text.
const HEX_RADIX: u32 = 16;

type HmacSha256 = Hmac<Sha256>;

/// Reason why the text of a key file is not a key ring.
///
/// No variant holds a part of the input, because the input is a secret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyRingReason {
    /// The file holds no line.
    Empty,
    /// A line is empty. The line number starts at 1.
    EmptyLine {
        /// The number of the line.
        line: usize,
    },
    /// A line does not have exactly 64 characters.
    Length {
        /// The number of the line.
        line: usize,
    },
    /// A line has a character that is not a lower-case hex digit.
    NotLowerHex {
        /// The number of the line.
        line: usize,
    },
}

impl fmt::Display for KeyRingReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the key file holds no key"),
            Self::EmptyLine { line } => write!(f, "line {line} of the key file is empty"),
            Self::Length { line } => {
                write!(f, "line {line} of the key file does not have 64 characters")
            }
            Self::NotLowerHex { line } => {
                write!(
                    f,
                    "line {line} of the key file has a character outside 0-9 and a-f"
                )
            }
        }
    }
}

/// The signing keys of one secret, the newest key first.
///
/// It solves the rotation problem: the server signs with the newest key and accepts every key
/// until the operator removes the old line. `Debug` redacts the keys. The type has no `Display`
/// and no `serde`.
#[derive(Clone)]
pub struct KeyRing {
    current: HmacSha256,
    older: Vec<HmacSha256>,
}

impl KeyRing {
    /// Parse the text of a key file.
    ///
    /// Each line holds one key as 64 lower-case hex characters. The first line is the newest key.
    /// A trailing line end is allowed.
    ///
    /// # Errors
    ///
    /// Returns a [`KeyRingReason`] for an empty file, an empty line, a line of another length, or a
    /// character outside `0-9a-f`.
    pub fn parse(text: &str) -> Result<Self, KeyRingReason> {
        let mut macs = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let number = index.saturating_add(1);
            macs.push(parse_line(line, number)?);
        }
        let mut macs = macs.into_iter();
        let current = macs.next().ok_or(KeyRingReason::Empty)?;
        Ok(Self {
            current,
            older: macs.collect(),
        })
    }

    /// Sign a message with HMAC-SHA256 under the newest key.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> [u8; 32] {
        let mut mac = self.current.clone();
        mac.update(message);
        mac.finalize().into_bytes().into()
    }

    /// Tell whether `tag` signs `message` under any key of the ring.
    ///
    /// The comparison runs in constant time (`Mac::verify_slice`).
    #[must_use]
    pub fn verify(&self, message: &[u8], tag: &[u8]) -> bool {
        std::iter::once(&self.current)
            .chain(&self.older)
            .any(|keyed| {
                let mut mac = keyed.clone();
                mac.update(message);
                mac.verify_slice(tag).is_ok()
            })
    }
}

impl fmt::Debug for KeyRing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeyRing([redacted])")
    }
}

/// Parse one key line into a keyed MAC.
fn parse_line(line: &str, number: usize) -> Result<HmacSha256, KeyRingReason> {
    if line.is_empty() {
        return Err(KeyRingReason::EmptyLine { line: number });
    }
    if line.chars().count() != KEY_HEX_CHARS {
        return Err(KeyRingReason::Length { line: number });
    }
    let not_hex = KeyRingReason::NotLowerHex { line: number };
    let digits = line
        .chars()
        .map(|c| {
            if c.is_ascii_digit() || matches!(c, 'a'..='f') {
                c.to_digit(HEX_RADIX)
            } else {
                None
            }
        })
        .collect::<Option<Vec<u32>>>()
        .ok_or(not_hex)?;
    let bytes = digits
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[high, low]| {
            high.checked_mul(HEX_RADIX)
                .and_then(|shifted| shifted.checked_add(*low))
                .and_then(|value| u8::try_from(value).ok())
        })
        .collect::<Option<Vec<u8>>>()
        .ok_or(not_hex)?;
    debug_assert_eq!(bytes.len(), KEY_BYTES);
    HmacSha256::new_from_slice(&bytes).map_err(|_| KeyRingReason::Length { line: number })
}

#[cfg(test)]
mod tests;
