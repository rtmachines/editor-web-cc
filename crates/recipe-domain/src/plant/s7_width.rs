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
mod tests {
    use super::super::type_tree::fixtures::{
        array_row, id, primitive_row, string_row, struct_row, types, wstring_row,
    };
    use super::super::{PlatformTypes, PlcType, TypeTree};
    use super::*;

    const ROOT: u128 = 1;
    const INT: u128 = 2;

    fn tree_of(rows: Vec<PlcType>, root: u128) -> TypeTree {
        let platform_types: PlatformTypes = types(rows);
        TypeTree::resolve(id(root), &platform_types).expect("fixture tree resolves")
    }

    fn primitive_root(primitive: Primitive) -> PlcType {
        match primitive {
            Primitive::String => string_row(ROOT, 16),
            Primitive::Wstring => wstring_row(ROOT, 16),
            other => primitive_row(ROOT, other),
        }
    }

    #[test]
    fn s7_width_of_each_primitive_follows_the_data_model_column() {
        let expected = [
            (Primitive::Bool, S7Width::X),
            (Primitive::Byte, S7Width::B),
            (Primitive::Word, S7Width::W),
            (Primitive::Dword, S7Width::D),
            (Primitive::Lword, S7Width::B),
            (Primitive::Sint, S7Width::B),
            (Primitive::Usint, S7Width::B),
            (Primitive::Int, S7Width::W),
            (Primitive::Uint, S7Width::W),
            (Primitive::Dint, S7Width::D),
            (Primitive::Udint, S7Width::D),
            (Primitive::Lint, S7Width::B),
            (Primitive::Ulint, S7Width::B),
            (Primitive::Real, S7Width::D),
            (Primitive::Lreal, S7Width::B),
            (Primitive::String, S7Width::B),
            (Primitive::Wstring, S7Width::B),
            (Primitive::Char, S7Width::B),
            (Primitive::Wchar, S7Width::W),
            (Primitive::Time, S7Width::D),
            (Primitive::Ltime, S7Width::B),
            (Primitive::Date, S7Width::W),
            (Primitive::Ldate, S7Width::B),
            (Primitive::TimeOfDay, S7Width::D),
            (Primitive::Ltod, S7Width::B),
            (Primitive::DateAndTime, S7Width::B),
            (Primitive::Ldt, S7Width::B),
            (Primitive::Dtl, S7Width::B),
            (Primitive::S5Time, S7Width::W),
        ];
        assert_eq!(expected.len(), Primitive::ALL.len());
        for (primitive, width) in expected {
            let tree = tree_of(vec![primitive_root(primitive)], ROOT);
            assert_eq!(S7Width::expected_for(&tree), width, "{primitive}");
        }
    }

    #[test]
    fn s7_width_of_string_of_two_characters_is_b_despite_four_bytes() {
        let tree = tree_of(vec![string_row(ROOT, 2)], ROOT);
        assert_eq!(S7Width::expected_for(&tree), S7Width::B);
    }

    #[test]
    fn s7_width_of_struct_is_b() {
        let rows = vec![
            primitive_row(INT, Primitive::Int),
            struct_row(ROOT, "ST_Zone", &[("setpoint", INT)]),
        ];
        let tree = tree_of(rows, ROOT);
        assert_eq!(S7Width::expected_for(&tree), S7Width::B);
    }

    #[test]
    fn s7_width_of_array_is_b() {
        let rows = vec![
            primitive_row(INT, Primitive::Int),
            array_row(ROOT, INT, &[(1, 4)]),
        ];
        let tree = tree_of(rows, ROOT);
        assert_eq!(S7Width::expected_for(&tree), S7Width::B);
    }
}
