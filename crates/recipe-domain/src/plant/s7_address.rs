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
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn reason_of(text: &str) -> Option<S7AddressReason> {
        S7Address::parse(text).err()
    }

    fn assert_round_trip(text: &str) {
        let parsed = S7Address::parse(text);
        assert!(parsed.is_ok(), "{text} must parse");
        assert_eq!(
            parsed.map(|address| address.to_string()),
            Ok(text.to_owned())
        );
    }

    #[test]
    fn s7_address_examples_round_trip() {
        for text in ["DB1.DBX0.7", "DB65535.DBD65535", "DB1.DBB0", "DB12.DBW4"] {
            assert_round_trip(text);
        }
    }

    #[test]
    fn s7_address_leading_zero_offset_displays_canonical_offset() {
        assert_eq!(
            S7Address::parse("DB1.DBW007").map(|a| a.to_string()),
            Ok("DB1.DBW7".to_owned())
        );
    }

    #[test]
    fn s7_address_accessors_return_the_parts() {
        let bit = S7Address::parse("DB12.DBX4.3");
        assert_eq!(
            bit.map(|a| (a.db(), a.width(), a.offset(), a.bit())),
            Ok((12, S7Width::X, 4, Some(3)))
        );
        let word = S7Address::parse("DB12.DBW4");
        assert_eq!(
            word.map(|a| (a.db(), a.width(), a.offset(), a.bit())),
            Ok((12, S7Width::W, 4, None))
        );
    }

    #[test]
    fn s7_address_width_letters_display_themselves() {
        assert_eq!(S7Width::X.letter(), 'X');
        assert_eq!(S7Width::B.letter(), 'B');
        assert_eq!(S7Width::W.letter(), 'W');
        assert_eq!(S7Width::D.letter(), 'D');
        assert_eq!(S7Width::D.to_string(), "D");
    }

    #[test]
    fn s7_address_db_above_u16_is_db_out_of_range() {
        assert_eq!(
            reason_of("DB65536.DBW0"),
            Some(S7AddressReason::DbOutOfRange)
        );
        assert_eq!(
            reason_of("DB99999.DBW0"),
            Some(S7AddressReason::DbOutOfRange)
        );
    }

    #[test]
    fn s7_address_db_zero_or_leading_zero_is_grammar_error() {
        assert_eq!(reason_of("DB0.DBW0"), Some(S7AddressReason::Grammar));
        assert_eq!(reason_of("DB01.DBW0"), Some(S7AddressReason::Grammar));
    }

    #[test]
    fn s7_address_offset_above_u16_is_offset_out_of_range() {
        assert_eq!(
            reason_of("DB1.DBW65536"),
            Some(S7AddressReason::OffsetOutOfRange)
        );
        assert_eq!(
            reason_of("DB1.DBX65536.0"),
            Some(S7AddressReason::OffsetOutOfRange)
        );
    }

    #[test]
    fn s7_address_malformed_texts_are_grammar_errors() {
        for text in [
            "DB1.DBX0",
            "DB1.DBW0.1",
            "DB1.DBX0.8",
            "DB1.DBX0.9",
            "DB1.DBQ0",
            "db1.dbw0",
            "DB1.DBW",
            "DB1.DBW123456",
            "DB1.DBX0.7 ",
            "",
        ] {
            assert_eq!(reason_of(text), Some(S7AddressReason::Grammar), "{text}");
        }
    }

    fn width() -> impl Strategy<Value = S7Width> {
        prop_oneof![
            Just(S7Width::X),
            Just(S7Width::B),
            Just(S7Width::W),
            Just(S7Width::D),
        ]
    }

    proptest! {
        #[test]
        fn s7_address_generated_text_round_trips(
            db in 1_u16..=u16::MAX,
            width in width(),
            offset in 0_u16..=u16::MAX,
            bit in 0_u8..=7,
        ) {
            let (text, expected_bit) = match width {
                S7Width::X => (format!("DB{db}.DBX{offset}.{bit}"), Some(bit)),
                other => (format!("DB{db}.DB{other}{offset}"), None),
            };
            match S7Address::parse(&text) {
                Ok(parsed) => {
                    prop_assert_eq!(parsed.to_string(), text);
                    prop_assert_eq!(parsed.db(), db);
                    prop_assert_eq!(parsed.width(), width);
                    prop_assert_eq!(parsed.offset(), offset);
                    prop_assert_eq!(parsed.bit(), expected_bit);
                }
                Err(reason) => prop_assert!(false, "{text} failed: {reason}"),
            }
        }
    }
}
