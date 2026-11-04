//! Message text of a limit bound.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::plant::{IecLiteral, PlcPlatform, Primitive, RuleSet, TimeType};

/// Write a bound as message text.
///
/// A time bound is a count of the base unit. The text is its canonical IEC literal
/// (`domain-model.md` section 2.1). Any other bound is its canonical decimal text.
pub(in crate::recipe) fn bound_text(
    primitive: Primitive,
    platform: PlcPlatform,
    bound: Decimal,
) -> String {
    bound_text_under(primitive, platform, bound, RuleSet::Current)
}

/// Write a bound as message text under the rule set `rules`.
///
/// The function solves one problem: a stored bound that the current set refuses still needs
/// its literal. A caller that reads a bound that it does not write passes [`RuleSet::Earlier`]
/// (ADR-0120 (d)).
pub(in crate::recipe) fn bound_text_under(
    primitive: Primitive,
    platform: PlcPlatform,
    bound: Decimal,
    rules: RuleSet,
) -> String {
    time_literal(primitive, platform, bound, rules).unwrap_or_else(|| bound.to_string())
}

/// Write a count as an IEC literal, or return `None` when it has none.
fn time_literal(
    primitive: Primitive,
    platform: PlcPlatform,
    count: Decimal,
    rules: RuleSet,
) -> Option<String> {
    let time_type = TimeType::of(primitive, platform)?;
    if !count.fract().is_zero() {
        return None;
    }
    let literal = IecLiteral::from_count(time_type, count.to_i128()?, rules).ok()?;
    Some(literal.to_string())
}
