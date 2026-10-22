//! The diff of two versions of one recipe (ADR-0050).
//!
//! The function [`diff_pairs`] compares the parameters of one page by key. It reads no port and
//! knows no page. The use case reads the pairs and orders them (ADR-0050 (a), (e)).

mod definition;
mod limits;
mod pairs;
mod rows;
mod translations;
mod values;

pub use pairs::DiffPair;
pub use rows::{
    DefinitionChange, DiffBounds, DiffKind, DiffKindReason, DiffPart, DiffRow, DiffRows,
};

use crate::plant::TypeTrees;
use limits::LimitSide;
use pairs::Sides;
use rows::PairParts;

/// Compare the pairs of one page and return the rows of each section (ADR-0050 (b) to (e)).
///
/// The rows keep the order of `pairs`. One part of one parameter shows at most `leaf_rows`
/// rows and then one [`DiffPart::Omitted`] row with the count of the others. A pair on both
/// sides without a difference gives no row and counts in [`DiffRows::unchanged`].
/// A `leaf_rows` value above the range of `usize` cuts no part, because no list can be longer.
///
/// The caller requires a tree in `trees` for each type and checks each pair with
/// [`DiffPair::check_stored`] first. Then each value and each bound shows as canonical text (A43).
#[must_use]
pub fn diff_pairs(pairs: &[DiffPair<'_>], trees: &TypeTrees, leaf_rows: u32) -> DiffRows {
    let leaf_rows = usize::try_from(leaf_rows).unwrap_or(usize::MAX);
    let parts = pairs.iter().map(|pair| {
        let tag = match pair.sides() {
            Sides::Added(parameter)
            | Sides::Removed(parameter)
            | Sides::Both { new: parameter, .. } => parameter.tag().clone(),
        };
        (pair.key(), tag, pair_parts(pair.sides(), trees))
    });
    DiffRows::from_parts(parts, leaf_rows)
}

/// Return the rows of one pair, one list for each section.
fn pair_parts(sides: Sides<'_>, trees: &TypeTrees) -> PairParts {
    let none = LimitSide::default();
    match sides {
        Sides::Added(new) => PairParts {
            limits: limits::limit_rows(none, LimitSide::of(new, trees), false),
            added: vec![values::parameter_row(new, trees)],
            ..PairParts::default()
        },
        Sides::Removed(old) => PairParts {
            limits: limits::limit_rows(LimitSide::of(old, trees), none, false),
            removed: vec![values::parameter_row(old, trees)],
            ..PairParts::default()
        },
        Sides::Both { old, new } => {
            // A type change shows every limit row and every leaf of both sides (ADR-0050 (c)).
            let is_retyped = old.plc_type() != new.plc_type();
            PairParts {
                limits: limits::limit_rows(
                    LimitSide::of(old, trees),
                    LimitSide::of(new, trees),
                    is_retyped,
                ),
                values: values::value_rows(old, new, trees, is_retyped),
                definitions: definition::definition_rows(old, new),
                translations: translations::translation_rows(old, new),
                ..PairParts::default()
            }
        }
    }
}

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod kind_tests;
#[cfg(test)]
mod order_tests;
#[cfg(test)]
mod part_tests;
#[cfg(test)]
mod presence_tests;
#[cfg(test)]
mod retype_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod type_tests;
#[cfg(test)]
mod value_tests;
