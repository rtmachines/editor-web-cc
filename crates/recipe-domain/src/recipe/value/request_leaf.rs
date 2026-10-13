//! Conversion of one request leaf to its stored form (design D13).

use serde_json::Value;

use super::leaf::{LeafType, Mode, convert};
use crate::plant::{PlcPlatform, TypeNode};

/// Convert the value of one new leaf of a rebind to its canonical stored form.
///
/// The value has the form of a request. A time leaf is an IEC literal and the result is a count.
/// The function returns the short text of the expected form when the value does not match, or
/// when `node` is not a scalar leaf.
pub(in crate::recipe) fn request_leaf(
    value: &Value,
    node: &TypeNode,
    platform: PlcPlatform,
) -> Result<Value, String> {
    let TypeNode::Leaf {
        primitive,
        string_length,
        ..
    } = node
    else {
        return Err(String::from("a scalar value"));
    };
    convert(
        value,
        LeafType {
            primitive: *primitive,
            string_length: *string_length,
            platform,
            mode: Mode::Request,
        },
    )
}
