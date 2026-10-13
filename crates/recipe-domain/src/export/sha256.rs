//! SHA-256 hash of an export document (`domain-model.md` section 2.2, `api.md` section 6.1).

use std::fmt;

/// Number of characters of the text form of a SHA-256 hash.
const TEXT_LEN: usize = 64;

/// SHA-256 hash of an export document.
///
/// The text form has 64 lower-case hexadecimal characters. The type solves one problem:
/// a stored or exported hash always holds the 32 bytes of a digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Sha256([u8; 32]);

/// Reason why a text is not a SHA-256 hash (ADR-0025).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sha256Reason {
    /// The text does not have 64 characters.
    Length,
    /// The text has a character that is not in `0-9a-f`.
    NotLowerHex,
}

impl fmt::Display for Sha256Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length => f.write_str("a SHA-256 hash has 64 characters"),
            Self::NotLowerHex => {
                f.write_str("a SHA-256 hash has only the characters 0 to 9 and a to f")
            }
        }
    }
}

/// Return the value of a lower-case hexadecimal character.
fn lower_hex_value(c: char) -> Option<u8> {
    match c {
        '0'..='9' | 'a'..='f' => c.to_digit(16).and_then(|digit| u8::try_from(digit).ok()),
        _ => None,
    }
}

impl Sha256 {
    /// Parse a hash of 64 lower-case hexadecimal characters.
    ///
    /// # Errors
    ///
    /// Returns [`Sha256Reason::Length`] when the text does not have 64 characters.
    /// Returns [`Sha256Reason::NotLowerHex`] when a character is not in `0-9a-f`.
    pub fn parse(text: &str) -> Result<Self, Sha256Reason> {
        if text.chars().count() != TEXT_LEN {
            return Err(Sha256Reason::Length);
        }
        let mut bytes = [0_u8; 32];
        let mut chars = text.chars();
        for byte in &mut bytes {
            let high = chars.next().and_then(lower_hex_value);
            let low = chars.next().and_then(lower_hex_value);
            let (Some(high), Some(low)) = (high, low) else {
                return Err(Sha256Reason::NotLowerHex);
            };
            let shifted = high.checked_shl(4).ok_or(Sha256Reason::NotLowerHex)?;
            *byte = shifted | low;
        }
        Ok(Self(bytes))
    }

    /// Return the 32 bytes of the digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for Sha256 {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn lower_case_hex_parses_and_displays_the_same_text() {
        let parsed = Sha256::parse(EMPTY_HASH).map(|hash| hash.to_string());
        assert_eq!(parsed, Ok(EMPTY_HASH.to_owned()));
    }

    #[test]
    fn sha256_of_the_empty_input_parses_and_round_trips() {
        let hash = Sha256::parse(EMPTY_HASH).unwrap();
        assert_eq!(hash.to_string(), EMPTY_HASH);
        assert_eq!(hash.as_bytes().first(), Some(&0xe3));
    }

    #[test]
    fn sixty_three_characters_give_length_reason() {
        let text = &EMPTY_HASH[..63];
        assert_eq!(Sha256::parse(text), Err(Sha256Reason::Length));
    }

    #[test]
    fn sixty_five_characters_give_length_reason() {
        let text = format!("{EMPTY_HASH}0");
        assert_eq!(Sha256::parse(&text), Err(Sha256Reason::Length));
    }

    #[test]
    fn upper_case_letter_gives_not_lower_hex_reason() {
        let text = format!("E{}", &EMPTY_HASH[1..]);
        assert_eq!(Sha256::parse(&text), Err(Sha256Reason::NotLowerHex));
    }

    #[test]
    fn non_hex_letter_gives_not_lower_hex_reason() {
        let text = format!("g{}", &EMPTY_HASH[1..]);
        assert_eq!(Sha256::parse(&text), Err(Sha256Reason::NotLowerHex));
    }

    #[test]
    fn bytes_zero_to_thirty_one_display_as_hex_pairs() {
        let bytes: [u8; 32] = core::array::from_fn(|index| u8::try_from(index).unwrap());
        let expected = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        assert_eq!(Sha256::from(bytes).to_string(), expected);
    }

    #[test]
    fn from_bytes_keeps_the_bytes() {
        let bytes = [7_u8; 32];
        assert_eq!(Sha256::from(bytes).as_bytes(), &bytes);
    }

    proptest! {
        #[test]
        fn any_bytes_round_trip_through_text(bytes in any::<[u8; 32]>()) {
            let hash = Sha256::from(bytes);
            let text = hash.to_string();
            prop_assert_eq!(Sha256::parse(&text), Ok(hash));
        }
    }
}
