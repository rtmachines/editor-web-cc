//! Check of a limit row against a type tree (design D9).

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::fault::{LimitFault, LimitFaults};
use super::list::LimitInput;
use super::process::{Bounds, ProcessLimit, StoredLimit};
use super::render::bound_text;
use crate::plant::{
    IecLiteral, LeafClass, LeafPath, LeafTarget, PlcPlatform, Primitive, RuleSet, TimeType,
    TypeTree,
};
use crate::recipe::PrecisionField;
use crate::recipe::value::{
    canonical, is_integer, max_significant_digits, numeric_range, parse_decimal, significant_digits,
};

/// The kind of bound that a leaf class takes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `min` and `max` of a numeric, duration, or instant leaf.
    Range,
    /// `max_length` of a string leaf.
    Length,
}

impl ProcessLimit {
    /// Check a limit row of a request against `tree`.
    ///
    /// The checks run in this order: leaf, class, bounds, order of the bounds. A time bound is
    /// an IEC literal of the leaf type. The result holds it as a count.
    ///
    /// # Errors
    ///
    /// Returns [`LimitFault::NoLeaf`] when the tree has no leaf at the path.
    /// Returns [`LimitFault::NotApplicable`] when the leaf class does not accept the fields.
    /// Returns [`LimitFault::OutsideTypeRange`] when a bound or a length is outside the type.
    /// Returns [`LimitFault::MinGreaterThanMax`] when the lower bound is above the upper bound.
    pub fn parse(input: &LimitInput, tree: &TypeTree) -> Result<Self, LimitFault> {
        Self::parse_all(input, tree).map_err(LimitFaults::into_first)
    }

    /// Check a limit row of a request against `tree` and return every fault of the row.
    ///
    /// The checks are the checks of [`ProcessLimit::parse`]. A `REAL` or `LREAL` row with too
    /// many significant digits in both bounds gives one precision fault for each bound, `min`
    /// first (ADR-0041, ADR-0117 (a)). Every other row gives one fault.
    pub(in crate::recipe) fn parse_all(
        input: &LimitInput,
        tree: &TypeTree,
    ) -> Result<Self, LimitFaults> {
        let has_range = input.min.is_some() || input.max.is_some();
        let (target, kind) = classify(&input.leaf_path, tree, has_range, input.max_length)?;
        let outside = || LimitFault::OutsideTypeRange {
            leaf: input.leaf_path.clone(),
        };
        let read = |text: &Option<String>| match text {
            None => Ok(None),
            Some(text) => read_text(&target, tree.platform(), text)
                .map(Some)
                .ok_or_else(outside),
        };
        let min = read(&input.min)?;
        let max = read(&input.max)?;
        finish(
            &input.leaf_path,
            &target,
            tree,
            kind,
            (min, max, input.max_length),
            RuleSet::Current,
        )
    }

    /// Check a stored row against `tree` under the rule set `rules`.
    ///
    /// The checks are the checks of [`ProcessLimit::parse`]. A time bound is already a count.
    /// The rule set [`RuleSet::Earlier`] skips the check of the significant digits, because
    /// the caller reads a limit that it does not write (ADR-0120).
    ///
    /// # Errors
    ///
    /// Returns the faults of [`ProcessLimit::parse`].
    pub fn from_stored(
        row: &StoredLimit,
        tree: &TypeTree,
        rules: RuleSet,
    ) -> Result<Self, LimitFault> {
        let has_range = row.min.is_some() || row.max.is_some();
        let (target, kind) = classify(&row.leaf_path, tree, has_range, row.max_length)?;
        let outside = || LimitFault::OutsideTypeRange {
            leaf: row.leaf_path.clone(),
        };
        let own = |bound: Option<Decimal>| match bound {
            None => Ok(None),
            Some(bound) => canonical(bound).map(Some).ok_or_else(outside),
        };
        let bounds = (own(row.min)?, own(row.max)?, row.max_length);
        finish(&row.leaf_path, &target, tree, kind, bounds, rules).map_err(LimitFaults::into_first)
    }
}

impl TryFrom<(&StoredLimit, &TypeTree)> for ProcessLimit {
    type Error = LimitFault;

    /// Check a stored row against `tree` under the earlier rule set.
    ///
    /// A caller that writes the limit uses [`ProcessLimit::from_stored`] with
    /// [`RuleSet::Current`].
    fn try_from((row, tree): (&StoredLimit, &TypeTree)) -> Result<Self, Self::Error> {
        Self::from_stored(row, tree, RuleSet::Earlier)
    }
}

/// Resolve the path and pick the kind of bound that the fields of the row need.
fn classify(
    path: &LeafPath,
    tree: &TypeTree,
    has_range: bool,
    max_length: Option<u16>,
) -> Result<(LeafTarget, Kind), LimitFault> {
    let leaf = || path.clone();
    let target = path
        .resolve(tree)
        .ok_or_else(|| LimitFault::NoLeaf { leaf: leaf() })?;
    let kind = match (target.leaf_class(), has_range, max_length.is_some()) {
        (LeafClass::Numeric | LeafClass::Duration | LeafClass::Instant, true, false) => Kind::Range,
        (LeafClass::String, false, true) => Kind::Length,
        _ => return Err(LimitFault::NotApplicable { leaf: leaf() }),
    };
    Ok((target, kind))
}

/// Check the bounds against the leaf type and build the limit.
fn finish(
    path: &LeafPath,
    target: &LeafTarget,
    tree: &TypeTree,
    kind: Kind,
    (min, max, max_length): (Option<Decimal>, Option<Decimal>, Option<u16>),
    rules: RuleSet,
) -> Result<ProcessLimit, LimitFaults> {
    let outside = || LimitFault::OutsideTypeRange { leaf: path.clone() };
    let bounds = match (kind, max_length) {
        (Kind::Length, Some(length)) => {
            let allowed = target.string_length().unwrap_or_default();
            if length == 0 || length > allowed {
                return Err(outside().into());
            }
            Bounds::MaxLength(length)
        }
        _ => {
            let platform = tree.platform();
            for bound in [min, max].into_iter().flatten() {
                if !is_inside_type(target.primitive(), platform, bound, rules) {
                    return Err(outside().into());
                }
            }
            if let (Some(low), Some(high)) = (min, max)
                && low > high
            {
                let text = |bound| bound_text(target.primitive(), platform, bound);
                return Err(LimitFault::MinGreaterThanMax {
                    leaf: path.clone(),
                    min: text(low),
                    max: text(high),
                }
                .into());
            }
            if rules == RuleSet::Current {
                check_digits(path, target.primitive(), [min, max])?;
            }
            Bounds::Range { min, max }
        }
    };
    Ok(ProcessLimit::from_checked(
        (tree.root_type(), tree.platform()),
        path.clone(),
        bounds,
    ))
}

/// Check that a bound of a `REAL` or `LREAL` leaf has no more significant digits than the type.
///
/// The check runs after the range and order checks. Another type has no digit limit. Each bound
/// with too many digits gives its own fault, `min` first (ADR-0041).
fn check_digits(
    path: &LeafPath,
    primitive: Primitive,
    bounds: [Option<Decimal>; 2],
) -> Result<(), LimitFaults> {
    let Some(max_digits) = max_significant_digits(primitive) else {
        return Ok(());
    };
    let too_long = |bound: Option<Decimal>| {
        bound.is_some_and(|bound| significant_digits(bound) > u32::from(max_digits))
    };
    let [min, max] = bounds;
    let mut faults = [
        (PrecisionField::LimitMin, min),
        (PrecisionField::LimitMax, max),
    ]
    .into_iter()
    .filter(|(_, bound)| too_long(*bound))
    .map(|(field, _)| LimitFault::PrecisionExceeded {
        leaf: path.clone(),
        field,
        type_name: primitive,
        max_digits,
    });
    match faults.next() {
        None => Ok(()),
        Some(first) => Err(LimitFaults::new(first, faults.collect())),
    }
}

/// Read the text of a bound. A time bound is an IEC literal and becomes its count.
fn read_text(target: &LeafTarget, platform: PlcPlatform, text: &str) -> Option<Decimal> {
    match target.leaf_class() {
        LeafClass::Duration | LeafClass::Instant => {
            let time_type = TimeType::of(target.primitive(), platform)?;
            let literal = IecLiteral::parse(time_type, text).ok()?;
            Decimal::try_from_i128_with_scale(literal.count(), 0).ok()
        }
        _ => parse_decimal(text).and_then(canonical),
    }
}

/// Return `true` when a bound lies inside the range of the leaf type.
///
/// An integer type and a time type accept no fraction.
fn is_inside_type(
    primitive: Primitive,
    platform: PlcPlatform,
    bound: Decimal,
    rules: RuleSet,
) -> bool {
    if let Some((low, high)) = numeric_range(primitive) {
        let is_whole = !is_integer(primitive) || bound.fract().is_zero();
        return is_whole && bound >= low && bound <= high;
    }
    let Some((low, high)) =
        TimeType::of(primitive, platform).and_then(|time_type| time_type.range_under(rules))
    else {
        return false;
    };
    bound.fract().is_zero()
        && bound
            .to_i128()
            .is_some_and(|count| count >= low && count <= high)
}

#[cfg(test)]
mod tests;
