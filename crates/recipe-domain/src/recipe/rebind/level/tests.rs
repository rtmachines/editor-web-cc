//! Tests of the boundaries of `same_type_shape` through `kept_members` (ADR-0146 (c) to (f)).
//!
//! Each test builds two revisions of the struct `ST_Text` on `CodesysIec` and lists the members
//! that `kept_members` keeps. The seed row of the tests follows `migrations/0004_seed_plc_types.sql`.

use std::collections::BTreeSet;

use super::{kept_members, struct_level};
use crate::plant::fixtures::{
    array_row, id, machine, on_platform, primitive_row, string_row, struct_row_revision, types_for,
    wstring_row,
};
use crate::plant::{PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape, TypeTrees};

const PLATFORM: PlcPlatform = PlcPlatform::CodesysIec;

const TEXT_1: u128 = 100;
const TEXT_2: u128 = 101;
const ZONE_2: u128 = 110;
const ZONE_3: u128 = 111;
const INT: u128 = 1;
const DINT: u128 = 2;
const UINT: u128 = 3;
const STRING_80_ROW: u128 = 4;
const WSTRING_80_ROW: u128 = 5;
const ARRAY_STRING_80: u128 = 6;
const ARRAY_SEED_STRING: u128 = 7;
const ARRAY_SEED_STRING_3: u128 = 8;
const ARRAY_INT: u128 = 9;
const ARRAY_DINT: u128 = 10;

/// The id of the seed row `STRING` on `CodesysIec`.
const SEED_STRING: u128 = 0x5eed_0000_0000_4000_8000_0000_0000_001e;

/// The seed row `STRING` of `CodesysIec`: the default length 80 and 82 bytes.
fn seed_string() -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: id(SEED_STRING),
        machine: None,
        platforms: BTreeSet::from([PlcPlatform::CodesysIec, PlcPlatform::TwinCat3]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes: 82,
            string_length: Some(80),
        },
    })
    .unwrap()
}

/// The seed rows that every test uses.
fn base_rows() -> Vec<PlcType> {
    vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(DINT, Primitive::Dint),
        primitive_row(UINT, Primitive::Uint),
        seed_string(),
    ]
}

/// The names that `kept_members` keeps for the revisions `ST_Text@1` and `ST_Text@2`.
///
/// `rows` hold the member types besides the base rows. Each member list holds the name and the
/// row of the type of the member.
fn kept(
    rows: Vec<PlcType>,
    old_members: &[(&str, u128)],
    new_members: &[(&str, u128)],
) -> Vec<String> {
    let mut all = base_rows();
    all.extend(on_platform(PLATFORM, rows));
    all.extend(on_platform(
        PLATFORM,
        vec![
            struct_row_revision(TEXT_1, "ST_Text", 1, old_members),
            struct_row_revision(TEXT_2, "ST_Text", 2, new_members),
        ],
    ));
    let trees = TypeTrees::resolve(
        [TEXT_1, TEXT_2].map(id),
        &types_for(machine(), PLATFORM, all),
    );
    let old_tree = trees.tree(id(TEXT_1)).unwrap();
    let new_tree = trees.tree(id(TEXT_2)).unwrap();
    let source = struct_level(old_tree).unwrap();
    let target = struct_level(new_tree).unwrap();
    kept_members(&source, &target)
        .into_iter()
        .map(|name| name.as_str().to_owned())
        .collect()
}

/// The names of the members that a case keeps, or the empty list for a drop.
fn kept_one(rows: Vec<PlcType>, old_row: u128, new_row: u128) -> Vec<String> {
    kept(rows, &[("m", old_row)], &[("m", new_row)])
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|&name| name.to_owned()).collect()
}

fn string_80() -> PlcType {
    string_row(STRING_80_ROW, 80)
}

#[test]
fn kept_members_keeps_an_int_member_that_both_revisions_name_with_one_row() {
    assert_eq!(kept_one(Vec::new(), INT, INT), names(&["m"]));
}

#[test]
fn kept_members_keeps_a_struct_member_that_both_revisions_name_with_one_row() {
    let rows = vec![struct_row_revision(ZONE_2, "ST_Zone", 2, &[("temp", INT)])];

    assert_eq!(kept_one(rows, ZONE_2, ZONE_2), names(&["m"]));
}

#[test]
fn kept_members_keeps_string_80_of_the_machine_and_the_seed_string_on_codesys() {
    assert_eq!(
        kept_one(vec![string_80()], STRING_80_ROW, SEED_STRING),
        names(&["m"])
    );
}

#[test]
fn kept_members_keeps_the_seed_string_and_string_80_of_the_machine_on_codesys() {
    assert_eq!(
        kept_one(vec![string_80()], SEED_STRING, STRING_80_ROW),
        names(&["m"])
    );
}

#[test]
fn kept_members_keeps_an_array_of_string_80_and_an_array_of_the_seed_string_on_codesys() {
    let rows = vec![
        string_80(),
        array_row(ARRAY_STRING_80, STRING_80_ROW, &[(1, 2)]),
        array_row(ARRAY_SEED_STRING, SEED_STRING, &[(1, 2)]),
    ];

    assert_eq!(
        kept_one(rows, ARRAY_STRING_80, ARRAY_SEED_STRING),
        names(&["m"])
    );
}

#[test]
fn kept_members_drops_a_member_that_changes_from_int_to_dint() {
    assert_eq!(kept_one(Vec::new(), INT, DINT), Vec::<String>::new());
}

#[test]
fn kept_members_drops_a_member_that_changes_from_int_to_uint() {
    assert_eq!(kept_one(Vec::new(), INT, UINT), Vec::<String>::new());
}

#[test]
fn kept_members_drops_a_member_that_changes_from_string_80_to_wstring_80() {
    let rows = vec![string_80(), wstring_row(WSTRING_80_ROW, 80)];

    assert_eq!(
        kept_one(rows, STRING_80_ROW, WSTRING_80_ROW),
        Vec::<String>::new()
    );
}

#[test]
fn kept_members_drops_an_array_member_whose_dimensions_change() {
    let rows = vec![
        array_row(ARRAY_SEED_STRING, SEED_STRING, &[(1, 2)]),
        array_row(ARRAY_SEED_STRING_3, SEED_STRING, &[(1, 3)]),
    ];

    assert_eq!(
        kept_one(rows, ARRAY_SEED_STRING, ARRAY_SEED_STRING_3),
        Vec::<String>::new()
    );
}

#[test]
fn kept_members_drops_an_array_member_whose_element_changes_from_int_to_dint() {
    let rows = vec![
        array_row(ARRAY_INT, INT, &[(1, 2)]),
        array_row(ARRAY_DINT, DINT, &[(1, 2)]),
    ];

    assert_eq!(kept_one(rows, ARRAY_INT, ARRAY_DINT), Vec::<String>::new());
}

#[test]
fn kept_members_drops_a_member_that_changes_from_int_to_an_array_of_int() {
    let rows = vec![array_row(ARRAY_INT, INT, &[(1, 1)])];

    assert_eq!(kept_one(rows, INT, ARRAY_INT), Vec::<String>::new());
}

#[test]
fn kept_members_drops_a_struct_member_whose_revision_changes_with_equal_members() {
    let rows = vec![
        struct_row_revision(ZONE_2, "ST_Zone", 2, &[("temp", INT)]),
        struct_row_revision(ZONE_3, "ST_Zone", 3, &[("temp", INT)]),
    ];

    assert_eq!(kept_one(rows, ZONE_2, ZONE_3), Vec::<String>::new());
}

#[test]
fn kept_members_drops_a_member_that_only_the_source_has() {
    assert_eq!(
        kept(
            Vec::new(),
            &[("keep", INT), ("gone", INT)],
            &[("keep", INT)]
        ),
        names(&["keep"])
    );
}

#[test]
fn kept_members_does_not_list_a_member_that_only_the_target_has() {
    assert_eq!(
        kept(
            Vec::new(),
            &[("keep", INT)],
            &[("keep", INT), ("fresh", INT)]
        ),
        names(&["keep"])
    );
}
