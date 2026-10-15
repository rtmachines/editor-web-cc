use super::fixtures::{array_row, id, primitive_row, struct_row, struct_row_revision, types};
use super::*;
use crate::error_codes::ErrorCode;

const INT: u128 = 1;

fn with_seeds(mut rows: Vec<PlcType>) -> PlatformTypes {
    let mut all = vec![primitive_row(INT, Primitive::Int)];
    all.append(&mut rows);
    types(all)
}

fn resolved(root: u128, rows: Vec<PlcType>) -> TypeTree {
    TypeTree::resolve(id(root), &with_seeds(rows)).unwrap()
}

fn assert_cycle_named(tree: &TypeTree, name: &str) {
    let error = tree.check_struct_names().unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_005);
    assert!(matches!(
        &error,
        PlantError::TypeCycle { type_name } if type_name.as_str() == name
    ));
}

#[test]
fn earlier_revision_as_direct_member_is_a_cycle_by_name() {
    let tree = resolved(
        11,
        vec![
            struct_row(10, "ST_A", &[("v", INT)]),
            struct_row_revision(11, "ST_A", 2, &[("old", 10)]),
        ],
    );
    assert_cycle_named(&tree, "ST_A");
}

#[test]
fn earlier_revision_as_array_element_is_a_cycle_by_name() {
    let tree = resolved(
        12,
        vec![
            struct_row(10, "ST_A", &[("v", INT)]),
            array_row(11, 10, &[(1, 2)]),
            struct_row_revision(12, "ST_A", 2, &[("olds", 11)]),
        ],
    );
    assert_cycle_named(&tree, "ST_A");
}

#[test]
fn earlier_revision_inside_a_nested_struct_is_a_cycle_by_name() {
    let tree = resolved(
        12,
        vec![
            struct_row(10, "ST_A", &[("v", INT)]),
            struct_row(11, "ST_B", &[("old", 10)]),
            struct_row_revision(12, "ST_A", 2, &[("b", 11)]),
        ],
    );
    assert_cycle_named(&tree, "ST_A");
}

#[test]
fn the_check_by_name_does_not_change_resolve() {
    let rows = vec![
        struct_row(10, "ST_A", &[("v", INT)]),
        struct_row_revision(11, "ST_A", 2, &[("old", 10)]),
    ];
    assert!(TypeTree::resolve(id(11), &with_seeds(rows)).is_ok());
}

#[test]
fn one_struct_on_two_sibling_paths_is_not_a_cycle() {
    let tree = resolved(
        12,
        vec![
            struct_row(10, "ST_Inner", &[("v", INT)]),
            array_row(11, 10, &[(1, 2)]),
            struct_row(12, "ST_Outer", &[("a", 10), ("b", 10), ("c", 11)]),
        ],
    );
    assert!(tree.check_struct_names().is_ok());
}

#[test]
fn tree_without_a_repeated_name_passes() {
    let tree = resolved(
        11,
        vec![
            struct_row(10, "ST_B", &[("v", INT)]),
            struct_row(11, "ST_A", &[("b", 10), ("w", INT)]),
        ],
    );
    assert!(tree.check_struct_names().is_ok());
}
