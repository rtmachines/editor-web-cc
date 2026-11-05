//! Expected S7 width of a type tree (`domain-model.md` section 2.1 width
//! table, `data-model.md` section 7 column "S7 width").

use super::{Primitive, S7Width, TypeNode, TypeTree};

impl S7Width {
    /// Return the width that an address of a value of this tree must use.
    ///
    /// A `BOOL` root uses `X`. A root of 2 bytes uses `W` and a root of 4 bytes
    /// uses `D`. Every other root uses `B`, including a `STRING` of any length.
    #[must_use]
    pub fn expected_for(tree: &TypeTree) -> Self {
        match tree.root() {
            TypeNode::Leaf {
                primitive,
                size_bytes,
                ..
            } => leaf_width(*primitive, *size_bytes),
            TypeNode::Array { .. } | TypeNode::Struct { .. } => Self::B,
        }
    }
}

/// Return the width of a root leaf from its primitive and its S7 size.
fn leaf_width(primitive: Primitive, size_bytes: u32) -> S7Width {
    match (primitive, size_bytes) {
        (Primitive::Bool, _) => S7Width::X,
        (Primitive::String | Primitive::Wstring, _) => S7Width::B,
        (_, 2) => S7Width::W,
        (_, 4) => S7Width::D,
        (_, _) => S7Width::B,
    }
}

#[cfg(test)]
mod tests;
