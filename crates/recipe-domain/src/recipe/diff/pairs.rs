//! The pairs of parameters that the diff compares (ADR-0050 (b)).

use std::collections::HashMap;

use crate::plant::TypeTrees;
use crate::recipe::draft::{stored_limits, stored_value};
use crate::recipe::{Parameter, ParameterKey, RecipeError};

/// The sides of one pair. At least one side holds a parameter.
#[derive(Clone, Copy, Debug)]
pub(super) enum Sides<'a> {
    /// The key is only in the new version.
    Added(&'a Parameter),
    /// The key is only in the old version.
    Removed(&'a Parameter),
    /// The key is in both versions.
    Both {
        /// The parameter of the old version.
        old: &'a Parameter,
        /// The parameter of the new version.
        new: &'a Parameter,
    },
}

/// The parameters of one key in the old and the new version (A40, ADR-0050 (b)).
///
/// The type solves one problem: the diff never gets a pair without a parameter or a pair of two
/// keys. Only [`DiffPair::match_by_key`] builds it.
#[derive(Clone, Copy, Debug)]
pub struct DiffPair<'a> {
    key: ParameterKey,
    sides: Sides<'a>,
}

impl<'a> DiffPair<'a> {
    /// Match the parameters of both versions by key, in the order of `keys`.
    ///
    /// The use case gives the keys of one page in the order of the parameter table. A key that
    /// neither side holds gives no pair. A first approval has no old parameters, so every pair
    /// is added.
    pub fn match_by_key(
        keys: &[ParameterKey],
        old: impl IntoIterator<Item = &'a Parameter>,
        new: impl IntoIterator<Item = &'a Parameter>,
    ) -> Vec<Self> {
        let by_key = |side: Vec<&'a Parameter>| -> HashMap<ParameterKey, &'a Parameter> {
            side.into_iter().map(|p| (p.key(), p)).collect()
        };
        let old = by_key(old.into_iter().collect());
        let new = by_key(new.into_iter().collect());
        keys.iter()
            .filter_map(|key| {
                let sides = match (old.get(key), new.get(key)) {
                    (None, Some(new)) => Sides::Added(new),
                    (Some(old), None) => Sides::Removed(old),
                    (Some(old), Some(new)) => Sides::Both { old, new },
                    (None, None) => return None,
                };
                Some(Self { key: *key, sides })
            })
            .collect()
    }

    /// Return the key of the pair.
    #[must_use]
    pub const fn key(&self) -> ParameterKey {
        self.key
    }

    /// Return the parameter of the old version, or `None` for an added parameter.
    #[must_use]
    pub const fn old_side(&self) -> Option<&'a Parameter> {
        match self.sides {
            Sides::Removed(old) | Sides::Both { old, .. } => Some(old),
            Sides::Added(_) => None,
        }
    }

    /// Return the parameter of the new version, or `None` for a removed parameter.
    #[must_use]
    pub const fn new_side(&self) -> Option<&'a Parameter> {
        match self.sides {
            Sides::Added(new) | Sides::Both { new, .. } => Some(new),
            Sides::Removed(_) => None,
        }
    }

    /// Check the stored limit rows and the stored value of each side against its tree (A43).
    ///
    /// The method solves one problem: a row that does not fit its tree has no canonical text.
    /// The caller checks each pair before [`diff_pairs`](crate::recipe::diff_pairs). Then no
    /// value shows as its stored JSON, and no time bound shows as a count. The method does not
    /// check a type without a tree in `trees`.
    ///
    /// # Errors
    ///
    /// Returns the error of the snapshot check for the first row that does not fit (ADR-0028).
    /// A limit row gives its limit code, for example `E-RCP-006`. A value gives `E-RCP-005`. The
    /// old side comes first, and on each side the limit rows come before the value.
    pub fn check_stored(&self, trees: &TypeTrees) -> Result<(), RecipeError> {
        [self.old_side(), self.new_side()]
            .into_iter()
            .flatten()
            .try_for_each(|parameter| {
                let Some(tree) = trees.tree(parameter.plc_type()) else {
                    return Ok(());
                };
                stored_limits(parameter.tag(), parameter.limits(), tree)?;
                stored_value(parameter.tag(), parameter.value(), tree).map(drop)
            })
    }

    /// Return the sides of the pair.
    pub(super) const fn sides(&self) -> Sides<'a> {
        self.sides
    }
}
