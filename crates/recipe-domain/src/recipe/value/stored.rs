//! The stored form of a parameter value (design D6, D8).

use serde_json::Value;

use super::ParameterValue;
use super::fault::ValueFault;
use super::leaf::Mode;
use super::walk::canonicalize;
use crate::plant::{RuleSet, TypeTree};

/// The JSONB value as storage returns it.
///
/// The type is not checked (design D6). The conversion `TryFrom<(&StoredValue, &TypeTree)>`
/// for [`ParameterValue`] checks it. The type solves one problem: a raw database value cannot
/// pass for a checked value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredValue(Value);

impl StoredValue {
    /// Return the JSON of the stored value.
    #[must_use]
    pub const fn as_json(&self) -> &Value {
        &self.0
    }
}

impl From<Value> for StoredValue {
    fn from(json: Value) -> Self {
        Self(json)
    }
}

impl ParameterValue {
    /// Return the value in the form that storage keeps.
    ///
    /// A time leaf is a count in both forms, so the JSON does not change.
    #[must_use]
    pub fn to_stored(&self) -> StoredValue {
        StoredValue(self.json.clone())
    }
}

impl ParameterValue {
    /// Check a stored value against `tree` under the rule set `rules` (ADR-0120).
    ///
    /// The walk is the walk of [`ParameterValue::parse`]. A time leaf is a JSON number with the
    /// count and not an IEC literal. The rule set [`RuleSet::Earlier`] skips the check of the
    /// significant digits of ADR-0117.
    ///
    /// # Errors
    ///
    /// Returns the [`ValueFault`] of the first part that does not match the tree.
    pub fn from_stored(
        stored: &StoredValue,
        tree: &TypeTree,
        rules: RuleSet,
    ) -> Result<Self, ValueFault> {
        canonicalize(stored.0.clone(), tree, (Mode::Stored, rules))
    }
}

impl TryFrom<(&StoredValue, &TypeTree)> for ParameterValue {
    type Error = ValueFault;

    /// Check a stored value against `tree` under the earlier rule set.
    ///
    /// The conversion is the read of a stored part that nobody writes. A check under another rule
    /// set uses [`ParameterValue::from_stored`].
    fn try_from((stored, tree): (&StoredValue, &TypeTree)) -> Result<Self, Self::Error> {
        Self::from_stored(stored, tree, RuleSet::Earlier)
    }
}

#[cfg(test)]
mod tests;
