use super::fixtures::{
    PLATFORM, array_row, id, int_row_on, machine, other_machine, primitive_row, string_row,
    struct_row, struct_row_of, types,
};
use super::*;
use crate::error_codes::ErrorCode;

const INT: u128 = 1;
const REAL: u128 = 2;
const BOOL: u128 = 3;

fn seeds() -> Vec<PlcType> {
    vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(REAL, Primitive::Real),
        primitive_row(BOOL, Primitive::Bool),
    ]
}

fn with_seeds(mut rows: Vec<PlcType>) -> PlatformTypes {
    let mut all = seeds();
    all.append(&mut rows);
    types(all)
}

fn member_names(node: &TypeNode) -> Vec<String> {
    match node {
        TypeNode::Struct { members, .. } => {
            members.iter().map(|(name, _)| name.to_string()).collect()
        }
        _ => Vec::new(),
    }
}

#[test]
fn st_zone_of_data_model_9_1_has_six_leaves_and_depth_two() {
    let rows = vec![
        array_row(10, INT, &[(1, 3)]),
        string_row(11, 16),
        struct_row(
            20,
            "ST_Zone",
            &[
                ("temp_sp", REAL),
                ("active", BOOL),
                ("ramps", 10),
                ("label", 11),
            ],
        ),
    ];
    let tree = TypeTree::resolve(id(20), &with_seeds(rows)).unwrap();
    assert_eq!(tree.leaf_count(), 6);
    assert_eq!(tree.depth(), 2);
    assert_eq!(
        member_names(tree.root()),
        ["temp_sp", "active", "ramps", "label"]
    );
}

#[test]
fn primitive_root_has_one_leaf_and_depth_zero() {
    let tree = TypeTree::resolve(id(INT), &with_seeds(Vec::new())).unwrap();
    assert_eq!(tree.leaf_count(), 1);
    assert_eq!(tree.depth(), 0);
    assert!(matches!(
        tree.root(),
        TypeNode::Leaf {
            primitive: Primitive::Int,
            size_bytes: 2,
            ..
        }
    ));
}

#[test]
fn two_dimensional_array_has_four_leaves_and_depth_two() {
    let rows = vec![array_row(10, INT, &[(0, 1), (1, 2)])];
    let tree = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap();
    assert_eq!(tree.leaf_count(), 4);
    assert_eq!(tree.depth(), 2);
}

#[test]
fn array_of_array_has_six_leaves_and_depth_two() {
    let rows = vec![array_row(10, INT, &[(1, 3)]), array_row(11, 10, &[(1, 2)])];
    let tree = TypeTree::resolve(id(11), &with_seeds(rows)).unwrap();
    assert_eq!(tree.leaf_count(), 6);
    assert_eq!(tree.depth(), 2);
}

#[test]
fn shared_struct_appears_once_per_member() {
    let rows = vec![
        struct_row(10, "ST_Inner", &[("v", INT)]),
        struct_row(11, "ST_Outer", &[("a", 10), ("b", 10)]),
    ];
    let tree = TypeTree::resolve(id(11), &with_seeds(rows)).unwrap();
    assert_eq!(tree.leaf_count(), 2);
    assert_eq!(tree.depth(), 2);
    let members = match tree.root() {
        TypeNode::Struct { members, .. } => members.as_slice(),
        _ => &[],
    };
    assert_eq!(members.len(), 2);
    assert_eq!(members.first().map(|m| &m.1), members.get(1).map(|m| &m.1));
}

#[test]
fn struct_cycle_over_two_structs_gives_type_cycle() {
    let rows = vec![
        struct_row(10, "ST_A", &[("b", 11)]),
        struct_row(11, "ST_B", &[("a", 10)]),
    ];
    let error = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_005);
    assert!(matches!(
        &error,
        PlantError::TypeCycle { type_name } if type_name.as_str() == "ST_A"
    ));
}

#[test]
fn cycle_through_an_array_names_the_first_struct_on_the_cycle() {
    let rows = vec![
        struct_row(10, "ST_A", &[("x", INT), ("arr", 11)]),
        array_row(11, 10, &[(1, 2)]),
    ];
    let error = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap_err();
    assert!(matches!(error, PlantError::TypeCycle { .. }));
}

#[test]
fn cycle_of_arrays_only_gives_row_shape() {
    let rows = vec![array_row(10, 11, &[(1, 2)]), array_row(11, 10, &[(1, 2)])];
    let error = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(matches!(
        error,
        PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::RowShape,
            ..
        }
    ));
}

#[test]
fn missing_member_id_gives_not_on_platform() {
    let rows = vec![struct_row(10, "ST_A", &[("x", INT), ("y", 99)])];
    let error = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    let missing = id(99).to_string();
    assert!(matches!(
        &error,
        PlantError::DerivedTypeInvalid { type_expr, reason: DerivedTypeReason::NotOnPlatform }
            if *type_expr == missing
    ));
}

#[test]
fn missing_root_id_gives_not_on_platform() {
    let error = TypeTree::resolve(id(99), &with_seeds(Vec::new())).unwrap_err();
    assert!(matches!(
        error,
        PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::NotOnPlatform,
            ..
        }
    ));
}

#[test]
fn row_of_another_platform_is_absent() {
    let mut rows = seeds();
    rows.push(int_row_on(50, PlcPlatform::S7_1200));
    rows.push(struct_row(10, "ST_A", &[("x", 50)]));
    let collected = types(rows);
    assert_eq!(collected.platform(), PLATFORM);
    assert!(collected.get(id(50)).is_none());
    assert!(collected.get(id(INT)).is_some());
    let error = TypeTree::resolve(id(10), &collected).unwrap_err();
    assert!(matches!(
        error,
        PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::NotOnPlatform,
            ..
        }
    ));
}

#[test]
fn struct_of_another_machine_is_absent() {
    let mut rows = seeds();
    rows.push(struct_row_of(
        other_machine(),
        50,
        "ST_Other",
        &[("x", INT)],
    ));
    rows.push(struct_row(10, "ST_A", &[("o", 50)]));
    let collected = types(rows);
    assert_eq!(collected.machine(), machine());
    assert!(collected.get(id(50)).is_none());
    let error = TypeTree::resolve(id(10), &collected).unwrap_err();
    assert!(matches!(
        error,
        PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::NotOnPlatform,
            ..
        }
    ));
}

#[test]
fn global_primitive_and_own_struct_are_kept() {
    let mut rows = seeds();
    rows.push(struct_row(10, "ST_A", &[("x", INT)]));
    let collected = types(rows);
    assert!(collected.get(id(INT)).is_some());
    assert!(collected.get(id(10)).is_some());
}

#[test]
fn duplicate_identifier_is_a_row_shape_error() {
    let mut rows = seeds();
    rows.push(primitive_row(INT, Primitive::Int));
    let error = PlatformTypes::collect(machine(), PLATFORM, rows).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(matches!(
        error,
        PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::RowShape,
            ..
        }
    ));
}
