//! Conversion of one request leaf to its stored form (design D13).

use serde_json::Value;

use super::expected::Expected;
use super::leaf::{LeafType, Mode, convert};
use super::texts::leaf_json;
use crate::plant::{PlcPlatform, TypeNode};

/// Convert the form text of one leaf to its canonical stored form (ADR-0039 (c)).
///
/// The class of the leaf sets the JSON form of the text. Then the conversion of
/// [`request_leaf`] applies, so one set of rules checks a text and a JSON value.
pub(in crate::recipe) fn request_text(
    text: &str,
    node: &TypeNode,
    platform: PlcPlatform,
) -> Result<Value, Expected> {
    let TypeNode::Leaf { primitive, .. } = node else {
        return Err(Expected::Scalar);
    };
    let value = leaf_json(text, *primitive)?;
    request_leaf(&value, node, platform)
}

/// Convert the value of one new leaf of a rebind to its canonical stored form.
///
/// The value has the form of a request. A time leaf is an IEC literal and the result is a count.
/// The function returns the expected form when the value does not match, or when `node` is not
/// a scalar leaf.
pub(in crate::recipe) fn request_leaf(
    value: &Value,
    node: &TypeNode,
    platform: PlcPlatform,
) -> Result<Value, Expected> {
    let TypeNode::Leaf {
        primitive,
        string_length,
        ..
    } = node
    else {
        return Err(Expected::Scalar);
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
