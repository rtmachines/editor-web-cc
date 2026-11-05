use super::super::fixtures::{
    array_row, dimensions, id, primitive_row, struct_row, struct_row_revision, types,
};
use super::*;
use crate::error_codes::ErrorCode;
use crate::plant::{PlatformTypes, Primitive};

const INT: u128 = 1;
const BOOL: u128 = 3;
const ZONE_2: u128 = 20;
const ZONE_3: u128 = 21;
const ARRAY_4: u128 = 30;
const INNER: u128 = 31;
const OUTER: u128 = 32;
const LINE: u128 = 40;
const BIG_1: u128 = 50;
const BIG_2: u128 = 51;
const BIG_ARRAY: u128 = 52;
const DEEP_1: u128 = 60;
const DEEP_ARRAY: u128 = 62;

fn catalog() -> PlatformTypes {
    types(vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(BOOL, Primitive::Bool),
        struct_row_revision(ZONE_2, "ST_Zone", 2, &[("temp", INT)]),
        struct_row_revision(ZONE_3, "ST_Zone", 3, &[("temp", INT), ("active", BOOL)]),
        array_row(ARRAY_4, ZONE_2, &[(1, 4)]),
        array_row(INNER, ZONE_2, &[(1, 3)]),
        array_row(OUTER, INNER, &[(1, 2)]),
        struct_row(LINE, "ST_Line", &[("zone", ZONE_2)]),
        struct_row_revision(BIG_1, "ST_Big", 1, &[("a", INT)]),
        struct_row_revision(BIG_2, "ST_Big", 2, &[("a", INT), ("b", INT)]),
        array_row(BIG_ARRAY, BIG_1, &[(1, 16_384)]),
    ])
}

fn tree(n: u128) -> TypeTree {
    TypeTree::resolve(id(n), &catalog()).unwrap()
}

fn rebound(from: u128, source: u128, to: u128) -> Option<TypeTree> {
    tree(source).rebind(id(from), &tree(to)).unwrap()
}

#[test]
fn struct_root_becomes_the_target_tree() {
    let result = rebound(ZONE_2, ZONE_2, ZONE_3);

    assert_eq!(result, Some(tree(ZONE_3)));
}

#[test]
fn array_of_the_source_keeps_dimensions_and_old_array_id() {
    let result = rebound(ZONE_2, ARRAY_4, ZONE_3).unwrap();

    let target = tree(ZONE_3);
    assert_eq!(
        result.root(),
        &TypeNode::Array {
            type_id: id(ARRAY_4),
            dimensions: dimensions(&[(1, 4)]),
            element: Box::new(target.root().clone()),
        }
    );
    assert_eq!(
        result.leaf_count(),
        4_u32.checked_mul(target.leaf_count()).unwrap()
    );
    assert_eq!(result.depth(), target.depth().checked_add(1).unwrap());
    assert_eq!(result.platform(), tree(ARRAY_4).platform());
}

#[test]
fn nested_arrays_keep_both_dimensions() {
    let result = rebound(ZONE_2, OUTER, ZONE_3).unwrap();

    let target = tree(ZONE_3);
    let inner = TypeNode::Array {
        type_id: id(INNER),
        dimensions: dimensions(&[(1, 3)]),
        element: Box::new(target.root().clone()),
    };
    assert_eq!(
        result.root(),
        &TypeNode::Array {
            type_id: id(OUTER),
            dimensions: dimensions(&[(1, 2)]),
            element: Box::new(inner),
        }
    );
    assert_eq!(
        result.leaf_count(),
        6_u32.checked_mul(target.leaf_count()).unwrap()
    );
    assert_eq!(result.depth(), target.depth().checked_add(2).unwrap());
}

#[test]
fn struct_member_of_the_source_does_not_count() {
    assert_eq!(rebound(ZONE_2, LINE, ZONE_3), None);
}

#[test]
fn primitive_root_is_not_a_use() {
    assert_eq!(rebound(ZONE_2, INT, ZONE_3), None);
}

#[test]
fn other_revision_in_the_chain_is_not_a_use() {
    assert_eq!(rebound(ZONE_3, ARRAY_4, ZONE_3), None);
}

#[test]
fn non_struct_target_gives_none() {
    assert_eq!(rebound(ZONE_2, ZONE_2, INT), None);
}

#[test]
fn target_above_the_leaf_limit_gives_e_plt_018() {
    let error = tree(BIG_ARRAY).rebind(id(BIG_1), &tree(BIG_2)).unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_018);
    assert!(matches!(
        error,
        PlantError::TypeTooLarge {
            ref type_name,
            limit: TypeLimit::Leaves,
        } if type_name == "ST_Big"
    ));
}

#[test]
fn target_above_the_depth_limit_gives_e_plt_018() {
    let (source, target) = deep_trees(MAX_DEPTH);
    assert_eq!(target.depth(), MAX_DEPTH);

    let error = source.rebind(id(DEEP_1), &target).unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_018);
    assert!(matches!(
        error,
        PlantError::TypeTooLarge {
            limit: TypeLimit::Depth,
            ..
        }
    ));
}

/// An array of `ST_Deep@1` and a target `ST_Deep@2` that has `levels` JSON levels.
fn deep_trees(levels: u32) -> (TypeTree, TypeTree) {
    let mut rows = vec![
        primitive_row(INT, Primitive::Int),
        struct_row_revision(DEEP_1, "ST_Deep", 1, &[("a", INT)]),
        array_row(DEEP_ARRAY, DEEP_1, &[(1, 2)]),
    ];
    let mut previous = INT;
    for step in 0..levels {
        let n = 1_000_u128.checked_add(u128::from(step)).unwrap();
        let name = if step == levels.checked_sub(1).unwrap() {
            "ST_Deep"
        } else {
            "ST_Level"
        };
        let revision = if name == "ST_Deep" { 2 } else { 1 };
        rows.push(struct_row_revision(n, name, revision, &[("a", previous)]));
        previous = n;
    }
    let all: PlatformTypes = types(rows);
    (
        TypeTree::resolve(id(DEEP_ARRAY), &all).unwrap(),
        TypeTree::resolve(id(previous), &all).unwrap(),
    )
}
