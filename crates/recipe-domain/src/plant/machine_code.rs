//! Machine code of the Plant Model context (`domain-model.md` section 2.1).

use std::fmt;

/// Longest number of digits of a machine code.
const MAX_DIGITS: usize = 7;

/// Human-readable code of a machine, such as `M1` (`domain-model.md` section 2.1).
///
/// The grammar is `M` followed by 1 to 7 ASCII digits. The first digit is not zero.
/// The number is in `1..=9999999`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MachineCode(u32);

/// Reason why a text is not a machine code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineCodeReason {
    /// The text does not match the grammar of a machine code.
    Format,
}

impl fmt::Display for MachineCodeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => f.write_str("a machine code is M and a number from 1 to 9999999"),
        }
    }
}

impl MachineCode {
    /// Parse a machine code such as `M1`.
    ///
    /// # Errors
    ///
    /// Returns [`MachineCodeReason::Format`] when the text does not match the grammar.
    pub fn parse(text: &str) -> Result<Self, MachineCodeReason> {
        let digits = text.strip_prefix('M').ok_or(MachineCodeReason::Format)?;
        let well_formed = (1..=MAX_DIGITS).contains(&digits.len())
            && digits.bytes().all(|b| b.is_ascii_digit())
            && !digits.starts_with('0');
        if !well_formed {
            return Err(MachineCodeReason::Format);
        }
        let number = digits
            .parse::<u32>()
            .map_err(|_| MachineCodeReason::Format)?;
        Ok(Self(number))
    }

    /// Return the number of the code, without the `M` prefix.
    #[must_use]
    pub const fn number(self) -> u32 {
        self.0
    }
}

impl fmt::Display for MachineCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "M{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn smallest_code_passes() {
        assert_eq!(MachineCode::parse("M1").map(MachineCode::number), Ok(1));
    }

    #[test]
    fn largest_code_passes() {
        assert_eq!(
            MachineCode::parse("M9999999").map(MachineCode::number),
            Ok(9_999_999)
        );
    }

    #[test]
    fn zero_fails() {
        assert_eq!(MachineCode::parse("M0"), Err(MachineCodeReason::Format));
    }

    #[test]
    fn leading_zero_fails() {
        assert_eq!(MachineCode::parse("M01"), Err(MachineCodeReason::Format));
    }

    #[test]
    fn eight_digits_fail() {
        assert_eq!(
            MachineCode::parse("M10000000"),
            Err(MachineCodeReason::Format)
        );
    }

    #[test]
    fn lower_case_prefix_fails() {
        assert_eq!(MachineCode::parse("m1"), Err(MachineCodeReason::Format));
    }

    #[test]
    fn prefix_without_digits_fails() {
        assert_eq!(MachineCode::parse("M"), Err(MachineCodeReason::Format));
    }

    #[test]
    fn letter_inside_digits_fails() {
        assert_eq!(MachineCode::parse("M12a"), Err(MachineCodeReason::Format));
    }

    #[test]
    fn leading_space_fails() {
        assert_eq!(MachineCode::parse(" M1"), Err(MachineCodeReason::Format));
    }

    #[test]
    fn plus_sign_fails() {
        assert_eq!(MachineCode::parse("M+1"), Err(MachineCodeReason::Format));
    }

    #[test]
    fn arabic_digit_fails() {
        assert_eq!(
            MachineCode::parse("M\u{0661}"),
            Err(MachineCodeReason::Format)
        );
    }

    proptest! {
        #[test]
        fn parse_and_display_round_trip(n in 1u32..=9_999_999) {
            let text = format!("M{n}");
            let code = MachineCode::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(code.to_string(), text);
            prop_assert_eq!(code.number(), n);
        }
    }
}
