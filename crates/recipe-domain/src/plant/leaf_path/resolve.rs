//! Tests of the resolution of a leaf path against a type tree.

use super::super::type_tree::fixtures::{
    array_row, id, primitive_row, string_row, struct_row, types,
};
use super::*;
use crate::plant::{PlcType, TypeTree};

const INT: u128 = 1;
const REAL: u128 = 2;
const BOOL: u128 = 3;

fn tree_of(mut rows: Vec<PlcType>, root: u128) -> TypeTree {
    let mut all = vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(REAL, Primitive::Real),
        primitive_row(BOOL, Primitive::Bool),
    ];
    all.append(&mut rows);
    TypeTree::resolve(id(root), &types(all)).expect("fixture tree resolves")
}

/// `ST_Zone` of `data-model.md` section 9.1.
fn st_zone_rows() -> Vec<PlcType> {
    vec![
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
    ]
}

fn st_zone() -> TypeTree {
    tree_of(st_zone_rows(), 20)
}

fn target(tree: &TypeTree, text: &str) -> Option<LeafTarget> {
    LeafPath::parse(text).expect("valid path").resolve(tree)
}

fn assert_leaf(tree: &TypeTree, text: &str, primitive: Primitive, count: u32) {
    let found = target(tree, text);
    assert_eq!(
        found.map(|leaf| (leaf.primitive(), leaf.count())),
        Some((primitive, count)),
        "{text}"
    );
}

fn assert_none(tree: &TypeTree, texts: &[&str]) {
    for text in texts {
        assert_eq!(target(tree, text), None, "{text}");
    }
}

#[test]
fn st_zone_all_ramps_names_three_int_leaves() {
    assert_leaf(&st_zone(), ".ramps[*]", Primitive::Int, 3);
}

#[test]
fn st_zone_one_ramp_names_one_leaf() {
    assert_leaf(&st_zone(), ".ramps[2]", Primitive::Int, 1);
}

#[test]
fn st_zone_member_leaves_carry_their_type() {
    let tree = st_zone();
    assert_leaf(&tree, ".temp_sp", Primitive::Real, 1);
    let label = target(&tree, ".label").expect("label leaf");
    assert_eq!(label.primitive(), Primitive::String);
    assert_eq!(label.string_length(), Some(16));
    assert_eq!(label.leaf_class(), LeafClass::String);
    assert_eq!(label.type_id(), id(11));
    assert_eq!(label.size_bytes(), 18);
}

#[test]
fn st_zone_paths_without_a_leaf_give_none() {
    assert_none(
        &st_zone(),
        &[
            ".ramps[4]",
            ".ramps[0]",
            ".ramps",
            ".missing",
            "[1]",
            "",
            ".temp_sp.x",
            ".temp_sp[1]",
            ".temp_sp[*]",
            ".ramps[1,2]",
            ".ramps[*][1]",
        ],
    );
}

#[test]
fn primitive_root_is_named_by_the_empty_path() {
    let tree = tree_of(Vec::new(), INT);
    assert_leaf(&tree, "", Primitive::Int, 1);
    assert_none(&tree, &[".a", "[1]", "[*]"]);
}

#[test]
fn two_dimensional_array_needs_one_index_per_dimension() {
    let tree = tree_of(vec![array_row(10, INT, &[(0, 1), (1, 2)])], 10);
    assert_none(&tree, &["[1]", "[0,3]", "[2,1]", "[1,2,3]", ""]);
    assert_leaf(&tree, "[1,2]", Primitive::Int, 1);
    assert_leaf(&tree, "[0,1]", Primitive::Int, 1);
    assert_leaf(&tree, "[*]", Primitive::Int, 4);
}

#[test]
fn array_of_array_multiplies_the_counts_of_each_all_segment() {
    let tree = tree_of(
        vec![array_row(10, INT, &[(1, 3)]), array_row(11, 10, &[(1, 2)])],
        11,
    );
    assert_leaf(&tree, "[*][*]", Primitive::Int, 6);
    assert_leaf(&tree, "[1][3]", Primitive::Int, 1);
    assert_leaf(&tree, "[*][2]", Primitive::Int, 2);
    assert_none(&tree, &["[*]", "[3][1]"]);
}

#[test]
fn array_of_structs_selects_a_member_of_each_element() {
    let mut rows = st_zone_rows();
    rows.push(array_row(30, 20, &[(1, 4)]));
    rows.push(struct_row(40, "ST_Line", &[("zones", 30)]));
    let tree = tree_of(rows, 40);
    assert_leaf(&tree, ".zones[*].temp_sp", Primitive::Real, 4);
    assert_leaf(&tree, ".zones[2].ramps[*]", Primitive::Int, 3);
    assert_leaf(&tree, ".zones[*].ramps[*]", Primitive::Int, 12);
    assert_none(&tree, &[".zones[5].temp_sp", ".zones[2]", ".zones[*]"]);
}
