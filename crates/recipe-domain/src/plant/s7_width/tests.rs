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
