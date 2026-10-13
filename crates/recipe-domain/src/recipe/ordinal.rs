//! Ordinal of a parameter (`reliability.md` section 7.11, design D1).

use std::fmt;

/// Largest ordinal of a parameter (`parameter.ordinal_max`, `reliability.md` section 7.11).
const ORDINAL_MAX: u32 = 100_000;

/// Position of a parameter in the order of its draft, from 0 to 100 000.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ordinal(u32);

/// Reason why a number is not an ordinal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrdinalReason {
    /// The number is larger than the largest ordinal.
    TooLarge,
}

impl fmt::Display for OrdinalReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => write!(f, "an ordinal is at most {ORDINAL_MAX}"),
        }
    }
}

impl Ordinal {
    /// Parse an ordinal from a number from 0 to 100 000.
    ///
    /// # Errors
    ///
    /// Returns [`OrdinalReason::TooLarge`] when the number is larger than 100 000.
    pub fn parse(value: u32) -> Result<Self, OrdinalReason> {
        if value > ORDINAL_MAX {
            return Err(OrdinalReason::TooLarge);
        }
        Ok(Self(value))
    }

    /// Return the number of this ordinal.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_zero_passes() {
        assert_eq!(Ordinal::parse(0).map(|ordinal| ordinal.value()), Ok(0));
    }

    #[test]
    fn ordinal_100_000_passes() {
        assert!(Ordinal::parse(100_000).is_ok());
    }

    #[test]
    fn ordinal_100_001_fails_with_too_large() {
        assert_eq!(Ordinal::parse(100_001), Err(OrdinalReason::TooLarge));
    }
}
