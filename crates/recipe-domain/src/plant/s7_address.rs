//! Address of a parameter in a data block of an S7 PLC (`domain-model.md`
//! section 2.1, design D8, `data-model.md` section 3.12).

use std::fmt;

use super::lexical::{is_canonical_digits, is_digits};

/// Width letter of an S7 address: the access width of the parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum S7Width {
    /// One bit, written `DBX<byte>.<bit>`.
    X,
    /// One byte or more, written `DBB<offset>`.
    B,
    /// One word, written `DBW<offset>`.
    W,
    /// One double word, written `DBD<offset>`.
    D,
}

impl S7Width {
    /// Return the width letter of the address text.
    #[must_use]
    pub const fn letter(self) -> char {
        match self {
            Self::X => 'X',
            Self::B => 'B',
            Self::W => 'W',
            Self::D => 'D',
        }
    }
}

impl fmt::Display for S7Width {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.letter())
    }
}

/// An address of an S7 data block, such as `DB1.DBW0`.
///
/// The width matches the type of the parameter. The bit is present only
/// for the width `X`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct S7Address {
    db: u16,
    width: S7Width,
    offset: u16,
    bit: Option<u8>,
}

/// Reason why a text is not an S7 address (ADR-0025).
///
/// The enum has no code. It is not an error type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum S7AddressReason {
    /// The text does not match the grammar of an S7 address.
    Grammar,
    /// The data block number is outside `1..=65535`.
    DbOutOfRange,
    /// The offset is outside `0..=65535`.
    OffsetOutOfRange,
}

impl fmt::Display for S7AddressReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grammar => f.write_str("the S7 address does not match the grammar"),
            Self::DbOutOfRange => f.write_str("the data block number is outside 1 to 65535"),
            Self::OffsetOutOfRange => f.write_str("the offset is outside 0 to 65535"),
        }
    }
}

impl S7Address {
    /// Parse the text form of an S7 address.
    ///
    /// # Errors
    ///
    /// Returns [`S7AddressReason::Grammar`] when the text does not match the grammar.
    /// Returns [`S7AddressReason::DbOutOfRange`] when the data block number is too large.
    /// Returns [`S7AddressReason::OffsetOutOfRange`] when the offset is too large.
    pub fn parse(text: &str) -> Result<Self, S7AddressReason> {
        let rest = text.strip_prefix("DB").ok_or(S7AddressReason::Grammar)?;
        let (db_text, rest) = rest.split_once('.').ok_or(S7AddressReason::Grammar)?;
        let db = parse_db(db_text)?;
        let rest = rest.strip_prefix("DB").ok_or(S7AddressReason::Grammar)?;
        if let Some(tail) = rest.strip_prefix('X') {
            let (offset_text, bit_text) = tail.split_once('.').ok_or(S7AddressReason::Grammar)?;
            return Ok(Self {
                db,
                width: S7Width::X,
                offset: parse_offset(offset_text)?,
                bit: Some(parse_bit(bit_text)?),
            });
        }
        let (width, offset_text) = split_width(rest).ok_or(S7AddressReason::Grammar)?;
        Ok(Self {
            db,
            width,
            offset: parse_offset(offset_text)?,
            bit: None,
        })
    }

    /// Return the data block number.
    #[must_use]
    pub const fn db(&self) -> u16 {
        self.db
    }

    /// Return the width letter.
    #[must_use]
    pub const fn width(&self) -> S7Width {
        self.width
    }

    /// Return the byte offset in the data block.
    #[must_use]
    pub const fn offset(&self) -> u16 {
        self.offset
    }

    /// Return the bit number, which is present only for the width `X`.
    #[must_use]
    pub const fn bit(&self) -> Option<u8> {
        self.bit
    }
}

/// Parse the data block number `[1-9][0-9]{0,4}`.
fn parse_db(text: &str) -> Result<u16, S7AddressReason> {
    if !is_canonical_digits(text, 5) {
        return Err(S7AddressReason::Grammar);
    }
    let value: u32 = text.parse().map_err(|_| S7AddressReason::Grammar)?;
    u16::try_from(value).map_err(|_| S7AddressReason::DbOutOfRange)
}

/// Parse the byte offset `[0-9]{1,5}`. Leading zeros are allowed.
fn parse_offset(text: &str) -> Result<u16, S7AddressReason> {
    if !is_digits(text, 5) {
        return Err(S7AddressReason::Grammar);
    }
    let value: u32 = text.parse().map_err(|_| S7AddressReason::Grammar)?;
    u16::try_from(value).map_err(|_| S7AddressReason::OffsetOutOfRange)
}

/// Parse the bit number `[0-7]`, one character only.
fn parse_bit(text: &str) -> Result<u8, S7AddressReason> {
    let [byte] = text.as_bytes() else {
        return Err(S7AddressReason::Grammar);
    };
    byte.checked_sub(b'0')
        .filter(|bit| *bit <= 7)
        .ok_or(S7AddressReason::Grammar)
}

/// Split the width letter `B`, `W`, or `D` from the text that follows `DB`.
fn split_width(text: &str) -> Option<(S7Width, &str)> {
    if let Some(tail) = text.strip_prefix('B') {
        return Some((S7Width::B, tail));
    }
    if let Some(tail) = text.strip_prefix('W') {
        return Some((S7Width::W, tail));
    }
    text.strip_prefix('D').map(|tail| (S7Width::D, tail))
}

impl fmt::Display for S7Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.bit {
            Some(bit) => write!(f, "DB{}.DBX{}.{bit}", self.db, self.offset),
            None => write!(f, "DB{}.DB{}{}", self.db, self.width, self.offset),
        }
    }
}

#[cfg(test)]
mod tests;
