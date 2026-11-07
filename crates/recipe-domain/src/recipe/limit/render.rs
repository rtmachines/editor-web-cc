//! Message text of a limit bound.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::plant::{IecLiteral, PlcPlatform, Primitive, RuleSet, TimeType};

/// Write a bound as message text.
///
/// A time bound is a count of the base unit. The text is its canonical IEC literal
/// (`domain-model.md` section 2.1). Any other bound is its canonical decimal text.
///
/// The function solves one problem: a stored bound that the current set refuses still needs
/// its literal. It writes under [`RuleSet::Earlier`], which accepts every count that the
/// current set accepts. A count of the current set gets the same literal (ADR-0120 (d)).
pub(in crate::recipe) fn bound_text(
    primitive: Primitive,
    platform: PlcPlatform,
    bound: Decimal,
) -> String {
    time_literal(primitive, platform, bound).unwrap_or_else(|| bound.to_string())
}

/// Write a count as an IEC literal, or return `None` when it has none.
fn time_literal(primitive: Primitive, platform: PlcPlatform, count: Decimal) -> Option<String> {
    let time_type = TimeType::of(primitive, platform)?;
    if !count.fract().is_zero() {
        return None;
    }
    let literal = IecLiteral::from_count(time_type, count.to_i128()?, RuleSet::Earlier).ok()?;
    Some(literal.to_string())
}
