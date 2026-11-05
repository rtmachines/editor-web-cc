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
mod tests;
