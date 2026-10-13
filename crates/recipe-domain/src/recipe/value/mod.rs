//! Parameter values of the Recipe Management context (`domain-model.md` section 2.2).
//!
//! The module holds the numeric ranges of the leaf types and the canonical form of a value.

mod check;
mod decimal;
mod fault;
mod leaf;
mod leaves;
mod numeric;
mod parse;
#[cfg(test)]
mod prop;
mod request_leaf;
mod stored;
#[cfg(test)]
pub(in crate::recipe) mod test_support;
mod walk;

pub(in crate::recipe) use decimal::{canonical, parse_decimal};
pub use fault::ValueFault;
pub(crate) use numeric::{is_integer, numeric_range};
pub(in crate::recipe) use request_leaf::request_leaf;
pub use stored::StoredValue;
pub(in crate::recipe) use walk::plc_index;

use serde_json::Value;

use crate::plant::{PlcPlatform, PlcTypeId};

/// The canonical stored form of the value of one parameter (invariant I7).
///
/// The type solves one problem: code cannot hold a value that did not pass the shape and range
/// check of its type tree. Only [`ParameterValue::parse`] and the conversion from a stored value
/// create it. A numeric, duration, or instant leaf is a JSON number with canonical decimal
/// text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterValue {
    root: PlcTypeId,
    platform: PlcPlatform,
    json: Value,
}

impl ParameterValue {
    /// Return the root type of the tree that the value matches.
    #[must_use]
    pub const fn root_type(&self) -> PlcTypeId {
        self.root
    }

    /// Return the platform of the tree that the value matches.
    #[must_use]
    pub const fn platform(&self) -> PlcPlatform {
        self.platform
    }

    /// Return the canonical JSON of the value.
    #[must_use]
    pub const fn as_json(&self) -> &Value {
        &self.json
    }
}
