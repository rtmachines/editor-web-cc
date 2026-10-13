//! The stored form of a parameter value (design D6, D8).

use serde_json::Value;

use super::ParameterValue;
use super::fault::ValueFault;
use super::leaf::Mode;
use super::walk::canonicalize;
use crate::plant::TypeTree;

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

impl TryFrom<(&StoredValue, &TypeTree)> for ParameterValue {
    type Error = ValueFault;

    /// Check a stored value against `tree`.
    ///
    /// The walk is the walk of [`ParameterValue::parse`]. A time leaf is a JSON number with the
    /// count and not an IEC literal.
    fn try_from((stored, tree): (&StoredValue, &TypeTree)) -> Result<Self, Self::Error> {
        canonicalize(stored.0.clone(), tree, Mode::Stored)
    }
}

#[cfg(test)]
mod tests;
