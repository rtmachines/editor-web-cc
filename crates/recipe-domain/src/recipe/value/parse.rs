//! Parse of a request value (design D8).

use serde_json::Value;

use super::ParameterValue;
use super::fault::ValueFault;
use super::leaf::Mode;
use super::walk::canonicalize;
use crate::plant::TypeTree;

impl ParameterValue {
    /// Parse the JSON text of a request and check it against `tree`.
    ///
    /// The result holds the canonical form of every leaf. A time leaf is an IEC literal in the
    /// text and a count in the result. A syntax error and the nesting limit of the JSON parser
    /// give a fault at the empty leaf path.
    ///
    /// # Errors
    ///
    /// Returns the [`ValueFault`] of the first part that does not match the tree.
    pub fn parse(json: &str, tree: &TypeTree) -> Result<Self, ValueFault> {
        let parsed = serde_json::from_str::<Value>(json)
            .map_err(|_| ValueFault::mismatch(Vec::new(), "valid JSON text"))?;
        canonicalize(parsed, tree, Mode::Request)
    }
}

#[cfg(test)]
mod tests;
