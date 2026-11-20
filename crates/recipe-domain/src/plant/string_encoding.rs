//! STRING encoding of a machine (ADR-0202).

use std::fmt;

use crate::plant::{MachineCode, PlantError, PlcPlatform};

/// Encoding of the bytes of a `STRING` value on the PLC of a machine.
///
/// A `Latin1` machine writes one byte for each character. A `Utf8` machine writes the UTF-8
/// bytes of each character, and the capacity of `STRING(n)` counts bytes. Every S7 machine is
/// `Latin1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StringEncoding {
    /// One Latin-1 byte for each character.
    #[default]
    Latin1,
    /// UTF-8 bytes. Only `CodesysIec` and `TwinCat3` machines use it.
    Utf8,
}

/// Reason why a text is not a `STRING` encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringEncodingReason {
    /// The text is not `Latin1` or `Utf8`.
    Unknown,
}

impl fmt::Display for StringEncodingReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => f.write_str("the text is not a STRING encoding"),
        }
    }
}

impl StringEncoding {
    /// Every encoding, in the order of the SQL check.
    pub const ALL: [Self; 2] = [Self::Latin1, Self::Utf8];

    /// Parse the SQL value of an encoding, `Latin1` or `Utf8`.
    ///
    /// # Errors
    ///
    /// Returns [`StringEncodingReason::Unknown`] for any other text.
    pub fn parse(text: &str) -> Result<Self, StringEncodingReason> {
        Self::ALL
            .into_iter()
            .find(|encoding| encoding.as_str() == text)
            .ok_or(StringEncodingReason::Unknown)
    }

    /// Return the SQL value of this encoding.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Latin1 => "Latin1",
            Self::Utf8 => "Utf8",
        }
    }
}

impl fmt::Display for StringEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Apply the platform rule of the `STRING` encoding of a machine (ADR-0202).
///
/// An S7 platform has only `Latin1`. The other platforms have both encodings.
///
/// # Errors
///
/// Returns `E-PLT-053` for `Utf8` on an S7 platform.
pub fn check_string_encoding(
    code: MachineCode,
    platform: PlcPlatform,
    encoding: StringEncoding,
) -> Result<(), PlantError> {
    match encoding {
        StringEncoding::Utf8 if platform.is_s7() => Err(PlantError::StringEncodingNotOnPlatform {
            code,
            encoding,
            platform,
        }),
        StringEncoding::Latin1 | StringEncoding::Utf8 => Ok(()),
    }
}

#[cfg(test)]
mod tests;
