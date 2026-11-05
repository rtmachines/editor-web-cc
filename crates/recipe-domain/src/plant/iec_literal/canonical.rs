//! Canonical text of a duration literal, and the S5TIME steps (`domain-model.md` section 2.1,
//! base-unit table of Assumption A43).

use std::fmt;

use crate::plant::{LeafClass, Primitive};

use super::duration::{base_nanos, units_of};
use super::{IecLiteral, IecLiteralReason, TimeType};

/// Largest S5TIME count in milliseconds (`9 990 s`).
const S5TIME_MAX_MS: i128 = 9_990_000;

/// Check a stored count against the rules of its type, without a parsed text.
///
/// # Errors
///
/// Returns [`IecLiteralReason::Step`] or [`IecLiteralReason::OutOfRange`] for a count
/// that the literal cannot show.
pub(super) fn check_count(ty: TimeType, count: i128) -> Result<(), IecLiteralReason> {
    match ty.primitive() {
        Primitive::S5Time => check_s5time_step(count),
        Primitive::Time | Primitive::Ltime => Ok(()),
        _ => super::instant::check_count(ty, count),
    }
}

/// Check that an S5TIME count in milliseconds is one of its steps.
///
/// # Errors
///
/// Returns [`IecLiteralReason::Step`] between two steps, and
/// [`IecLiteralReason::OutOfRange`] above 9 990 000 ms or below zero.
pub(super) fn check_s5time_step(count: i128) -> Result<(), IecLiteralReason> {
    if !(0..=S5TIME_MAX_MS).contains(&count) {
        return Err(IecLiteralReason::OutOfRange);
    }
    let step = match count {
        0..=9_990 => 10,
        9_991..=99_900 => 100,
        99_901..=999_000 => 1_000,
        _ => 10_000,
    };
    if count.checked_rem(step) == Some(0) {
        Ok(())
    } else {
        Err(IecLiteralReason::Step)
    }
}

/// Return the prefix of a duration type, such as `LT#` for `LTIME`.
fn duration_prefix_text(primitive: Primitive) -> Option<&'static str> {
    match primitive {
        Primitive::Time => Some("T#"),
        Primitive::Ltime => Some("LT#"),
        Primitive::S5Time => Some("S5T#"),
        _ => None,
    }
}

impl TryFrom<(TimeType, i128)> for IecLiteral {
    type Error = IecLiteralReason;

    fn try_from((ty, count): (TimeType, i128)) -> Result<Self, Self::Error> {
        Self::from_count(ty, count, super::RuleSet::Current)
    }
}

impl IecLiteral {
    /// Build a literal from a stored count under the rule set `rules`.
    ///
    /// A caller that reads a count that it does not write passes
    /// [`RuleSet::Earlier`](crate::plant::RuleSet::Earlier)
    /// (ADR-0120 (d)).
    ///
    /// # Errors
    ///
    /// Returns the [`IecLiteralReason`] of the first rule that the count breaks.
    pub fn from_count(
        ty: TimeType,
        count: i128,
        rules: super::RuleSet,
    ) -> Result<Self, IecLiteralReason> {
        check_count(ty, count)?;
        super::range::check(ty, count, rules)?;
        Ok(IecLiteral { ty, count })
    }
}

/// Render a duration literal. A count that its type cannot show gives `None`.
fn render_duration(literal: &IecLiteral) -> Option<String> {
    let primitive = literal.ty.primitive();
    let mut text = String::from(duration_prefix_text(primitive)?);
    if literal.count < 0 {
        text.push('-');
    }
    let mut remaining = literal
        .count
        .checked_abs()?
        .checked_mul(base_nanos(primitive).ok()?)?;
    let units = units_of(primitive);
    if remaining == 0 {
        text.push('0');
        text.push_str(units.last()?.letters());
        return Some(text);
    }
    for unit in units {
        let span = unit.nanos();
        let amount = remaining.checked_div(span)?;
        if amount > 0 {
            text.push_str(&amount.to_string());
            text.push_str(unit.letters());
        }
        remaining = remaining.checked_rem(span)?;
    }
    Some(text)
}

/// `Display` never returns an error, because `to_string` panics on one. A count that the
/// constructors reject cannot reach this code. If one did, the fallback is the plain count.
impl fmt::Display for IecLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = if self.ty.primitive().leaf_class() == LeafClass::Duration {
            render_duration(self)
        } else {
            super::instant::render_instant(self)
        };
        match text {
            Some(text) => f.write_str(&text),
            None => write!(f, "{}", self.count),
        }
    }
}

#[cfg(test)]
mod tests;
