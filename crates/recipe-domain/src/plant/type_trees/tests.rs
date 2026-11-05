use std::collections::BTreeSet;

use super::super::type_tree::fixtures::{
    PLATFORM, array_row, id, machine, primitive_row, struct_row, types, wstring_row,
};
use crate::error_codes::ErrorCode;
use crate::plant::{
    MemberName, PlatformTypes, PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape,
    TypeMember, TypeName,
};

use super::*;

const INT: u128 = 1;
const ZONE_2: u128 = 20;
const ZONE_3: u128 = 21;

fn zone_row(n: u128, revision: u32, supersedes: Option<u128>) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: id(n),
        machine: Some(machine()),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Zone").unwrap(),
            revision,
            supersedes: supersedes.map(id),
            members: vec![TypeMember {
                name: MemberName::parse("temp").unwrap(),
                type_id: id(INT),
            }],
        },
    })
    .unwrap()
}

fn catalog() -> PlatformTypes {
    types(vec![
        primitive_row(INT, Primitive::Int),
        zone_row(ZONE_2, 2, None),
        zone_row(ZONE_3, 3, Some(ZONE_2)),
        array_row(30, ZONE_2, &[(1, 4)]),
        struct_row(40, "ST_Line", &[("zone", ZONE_2)]),
        struct_row(41, "ST_Fresh", &[("zone", ZONE_3)]),
        array_row(31, INT, &[(1, 3)]),
    ])
}

fn all_ids() -> Vec<crate::plant::PlcTypeId> {
    [INT, ZONE_2, ZONE_3, 30, 31, 40, 41].map(id).to_vec()
}

#[test]
fn missing_row_keeps_its_plant_error() {
    let trees = TypeTrees::resolve([id(99)], &catalog());
    let error = trees.get(id(99)).unwrap().as_ref().unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(trees.tree(id(99)).is_none());
    assert!(trees.get(id(98)).is_none());
}

#[test]
fn missing_row_is_labelled_with_its_identifier() {
    let trees = TypeTrees::resolve([id(99)], &catalog());
    assert_eq!(trees.label(id(99)), id(99).to_string());
}

#[test]
fn row_dropped_for_the_platform_keeps_its_label() {
    let rows = vec![wstring_row(10, 10), primitive_row(INT, Primitive::Int)];
    let collected = PlatformTypes::collect(machine(), PlcPlatform::S7_300, rows).unwrap();
    assert_eq!(collected.label(id(10)), Some("WSTRING(10)".to_owned()));
    assert!(collected.get(id(10)).is_none());
    let trees = TypeTrees::resolve([id(10)], &collected);
    assert!(trees.get(id(10)).unwrap().is_err());
    assert_eq!(trees.label(id(10)), "WSTRING(10)");
}

#[test]
fn kept_rows_have_labels_by_shape() {
    let collected = catalog();
    assert_eq!(collected.label(id(INT)), Some("INT".to_owned()));
    assert_eq!(collected.label(id(ZONE_2)), Some("ST_Zone@2".to_owned()));
    assert_eq!(collected.label(id(31)), Some("ARRAY[1..3]".to_owned()));
    assert_eq!(collected.label(id(99)), None);
    assert_eq!(collected.rows().count(), 7);
}

#[test]
fn trees_keep_machine_and_platform_of_the_types() {
    let collected = catalog();
    let trees = TypeTrees::resolve(all_ids(), &collected);
    assert_eq!(trees.machine(), collected.machine());
    assert_eq!(trees.platform(), collected.platform());
    assert!(trees.tree(id(INT)).is_some());
}

#[test]
fn older_struct_revision_is_outdated() {
    let trees = TypeTrees::resolve(all_ids(), &catalog());
    let outdated = trees.outdated(id(ZONE_2)).unwrap();
    assert_eq!(outdated.type_name().to_string(), "ST_Zone");
    assert_eq!(outdated.current(), 2);
    assert_eq!(outdated.newest(), 3);
}

#[test]
fn array_of_an_older_revision_is_outdated() {
    let trees = TypeTrees::resolve(all_ids(), &catalog());
    let outdated = trees.outdated(id(30)).unwrap();
    assert_eq!((outdated.current(), outdated.newest()), (2, 3));
}

#[test]
fn struct_with_a_member_of_an_older_revision_is_outdated() {
    let trees = TypeTrees::resolve(all_ids(), &catalog());
    let outdated = trees.outdated(id(40)).unwrap();
    assert_eq!(outdated.type_name().to_string(), "ST_Zone");
    assert_eq!((outdated.current(), outdated.newest()), (2, 3));
}

#[test]
fn newest_revision_primitive_and_unresolved_ids_are_not_outdated() {
    let trees = TypeTrees::resolve(all_ids().into_iter().chain([id(99)]), &catalog());
    assert!(trees.outdated(id(ZONE_3)).is_none());
    assert!(trees.outdated(id(41)).is_none());
    assert!(trees.outdated(id(INT)).is_none());
    assert!(trees.outdated(id(31)).is_none());
    assert!(trees.outdated(id(99)).is_none());
    assert!(trees.outdated(id(98)).is_none());
}

#[test]
fn duplicate_ids_resolve_once() {
    let trees = TypeTrees::resolve([id(INT), id(INT)], &catalog());
    assert!(trees.tree(id(INT)).is_some());
}
