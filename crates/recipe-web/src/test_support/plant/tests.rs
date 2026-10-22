//! Tests of `Fixture::seed_plant` (design D12, task 4.2).
//!
//! Each test checks one item of the data of D12: the 64 units, the five levels, both revisions,
//! the usage chain, the recipe with its draft, and the archived machine with its archived unit.

use std::collections::BTreeSet;

use recipe_app::memory::{MemoryTransaction, seed_type_id};
use recipe_app::ports::{Isolation, Transaction as _};
use recipe_domain::plant::{
    MachineCode, MemberName, PlatformTypes, PlcPlatform, PlcType, Primitive, RecordShape, TypeNode,
    TypeTree, Unit, UnitPath,
};
use recipe_domain::recipe::{ReferenceNumber, VersionState};

use super::{LINE_TAG, M7_ID, M12_ID, PlantIds};
use crate::test_support::Fixture;

/// Seed a fixture and return it with the fixed identifiers.
fn seeded() -> (Fixture, PlantIds) {
    let fixture = Fixture::new();
    let ids = fixture.seed_plant();
    (fixture, ids)
}

/// Open a read transaction on the store of `fixture`.
fn begin(fixture: &Fixture) -> MemoryTransaction {
    fixture.store.begin_memory(Isolation::ReadOnly).unwrap()
}

/// Parse a unit path.
fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

/// Return the name and the revision of a struct row, or `None` for another row.
fn struct_label(row: &PlcType) -> Option<(String, u32)> {
    match row.shape() {
        RecordShape::Struct { name, revision, .. } => Some((name.to_string(), *revision)),
        _ => None,
    }
}

/// Return the names of the members of a struct row.
fn member_names(row: &PlcType) -> Vec<String> {
    match row.shape() {
        RecordShape::Struct { members, .. } => members
            .iter()
            .map(|member| member.name.as_str().to_owned())
            .collect(),
        _ => Vec::new(),
    }
}

/// Return the members of a struct node.
fn struct_members(node: &TypeNode) -> Option<&[(MemberName, TypeNode)]> {
    match node {
        TypeNode::Struct { members, .. } => Some(members.as_slice()),
        _ => None,
    }
}

/// Return the dimensions as text and the element of an array node.
fn array_parts(node: &TypeNode) -> Option<(String, &TypeNode)> {
    match node {
        TypeNode::Array {
            dimensions,
            element,
            ..
        } => Some((dimensions.to_string(), element.as_ref())),
        _ => None,
    }
}

#[test]
fn seed_plant_returns_the_same_fixed_identifiers_on_each_call() {
    let (_first_fixture, first) = seeded();
    let (_second_fixture, second) = seeded();
    assert_eq!(first, second);
    assert_eq!(*first.m7.as_uuid(), M7_ID);
    assert_eq!(*first.m12.as_uuid(), M12_ID);
    assert_ne!(M7_ID, M12_ID);
}

#[test]
fn seed_plant_m12_identifier_is_a_hyphenated_text_for_the_matrix() {
    let text = M12_ID.to_string();
    assert_eq!(text.len(), 36);
    assert_eq!(text.matches('-').count(), 4);
}

#[test]
fn seed_plant_m7_is_a_live_s7_300_machine() {
    let (fixture, ids) = seeded();
    let machine = fixture.store.state().machine(ids.m7).unwrap();
    assert_eq!(machine.code(), MachineCode::parse("M7").unwrap());
    assert_eq!(machine.platform(), PlcPlatform::S7_300);
    assert!(machine.is_live());
}

#[test]
fn seed_plant_m7_has_64_live_units_in_five_levels() {
    let (fixture, ids) = seeded();
    let machine = fixture.store.state().machine(ids.m7).unwrap();
    let units: Vec<&Unit> = machine.units().collect();
    assert_eq!(units.len(), 64);
    assert!(units.iter().all(|unit| unit.is_live()));
    let depths: BTreeSet<u8> = units.iter().map(|unit| unit.path().depth()).collect();
    assert_eq!(depths, BTreeSet::from([1, 2, 3, 4, 5]));
    let unit_ids: BTreeSet<_> = units.iter().map(|unit| unit.id()).collect();
    assert_eq!(unit_ids.len(), 64);
}

#[test]
fn seed_plant_m7_holds_the_chain_down_to_u1_2_4_5_6_and_the_unit_u1_2() {
    let (fixture, ids) = seeded();
    let machine = fixture.store.state().machine(ids.m7).unwrap();
    for text in ["U1", "U1.2", "U1.2.4", "U1.2.4.5", "U1.2.4.5.6"] {
        let unit = machine.unit(&path(text)).unwrap();
        assert!(unit.is_live(), "{text}");
        let parent = path(text).parent();
        if let Some(parent) = parent {
            assert!(machine.unit(&parent).unwrap().is_live(), "parent of {text}");
        }
    }
    assert_eq!(machine.unit(&path("U1.2.4.5.6")).unwrap().path().depth(), 5);
}

#[test]
fn seed_plant_m7_has_children_below_the_levels_three_and_four() {
    let (fixture, ids) = seeded();
    let machine = fixture.store.state().machine(ids.m7).unwrap();
    for depth in 1..=4_u8 {
        let has_child = machine
            .units()
            .filter(|unit| unit.path().depth() == depth)
            .any(|parent| {
                machine
                    .units()
                    .any(|child| child.path().parent().as_ref() == Some(parent.path()))
            });
        assert!(has_child, "no unit of depth {depth} has a child");
    }
}

#[tokio::test]
async fn seed_plant_holds_the_primitive_types_and_the_four_types_of_m7() {
    let (fixture, ids) = seeded();
    let rows = begin(&fixture).types().type_rows(ids.m7).await.unwrap();
    let global = rows.iter().filter(|row| row.machine().is_none()).count();
    assert_eq!(global, 11);
    let own = rows
        .iter()
        .filter(|row| row.machine() == Some(ids.m7))
        .count();
    assert_eq!(own, 4);
    let int = seed_type_id(Primitive::Int, None).unwrap();
    assert!(rows.iter().any(|row| row.id() == int));
}

#[tokio::test]
async fn seed_plant_has_the_revisions_1_and_2_of_st_zone_and_the_revision_1_of_st_line() {
    let (fixture, ids) = seeded();
    let rows = begin(&fixture).types().type_rows(ids.m7).await.unwrap();
    let mut labels: Vec<(String, u32)> = rows.iter().filter_map(struct_label).collect();
    labels.sort();
    let expected = [("ST_Line", 1), ("ST_Zone", 1), ("ST_Zone", 2)];
    assert_eq!(
        labels,
        expected.map(|(name, revision)| (name.to_owned(), revision))
    );
    let row = |id| rows.iter().find(|row| row.id() == id).unwrap();
    assert_eq!(member_names(row(ids.zone_v1)), ["speed"]);
    assert_eq!(member_names(row(ids.zone_v2)), ["speed", "temp_max"]);
    let supersedes = match row(ids.zone_v2).shape() {
        RecordShape::Struct { supersedes, .. } => *supersedes,
        _ => None,
    };
    assert_eq!(supersedes, Some(ids.zone_v1));
}

#[tokio::test]
async fn seed_plant_line_holds_an_array_of_st_zone_2_down_the_usage_chain() {
    let (fixture, ids) = seeded();
    let rows = begin(&fixture).types().type_rows(ids.m7).await.unwrap();
    let types = PlatformTypes::collect(ids.m7, PlcPlatform::S7_300, rows).unwrap();
    let tree = TypeTree::resolve(ids.line_v1, &types).unwrap();
    assert_eq!(tree.root_type(), ids.line_v1);
    assert_eq!(tree.leaf_count(), 8);
    let members = struct_members(tree.root()).unwrap();
    assert_eq!(members.len(), 1);
    let (name, node) = members.first().unwrap();
    assert_eq!(name.as_str(), "zones");
    assert_eq!(node.type_id(), ids.zones_array);
    let (dimensions, element) = array_parts(node).unwrap();
    assert_eq!(dimensions, "1..4");
    assert_eq!(element.type_id(), ids.zone_v2);
    assert!(matches!(element, TypeNode::Struct { revision: 2, .. }));
}

#[tokio::test]
async fn seed_plant_recipe_r100_has_a_live_draft_that_holds_the_parameter_line() {
    let (fixture, ids) = seeded();
    let mut tx = begin(&fixture);
    let reference = ReferenceNumber::parse("R-100").unwrap();
    let recipe = tx
        .recipes()
        .recipe_by_reference(&reference)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recipe.id(), ids.recipe);
    assert_eq!(recipe.machine(), ids.m7);
    assert!(!recipe.is_deleted());
    let states: Vec<(u32, VersionState)> = recipe
        .versions()
        .iter()
        .map(|header| (header.number(), header.state()))
        .collect();
    assert_eq!(
        states,
        [
            (1, VersionState::Retired),
            (2, VersionState::Approved),
            (3, VersionState::LiveDraft)
        ]
    );
    assert_eq!(recipe.version(3).unwrap().id(), ids.draft);
    let parameters = tx.recipes().version_parameters(ids.draft).await.unwrap();
    assert_eq!(parameters.len(), 1);
    let parameter = parameters.first().unwrap();
    assert_eq!(parameter.tag.as_str(), LINE_TAG);
    assert_eq!(parameter.name_en.as_str(), "line");
    assert_eq!(parameter.plc_type, ids.line_v1);
    assert_eq!(parameter.id, ids.parameter);
    for older in recipe.versions().iter().take(2) {
        let none = tx.recipes().version_parameters(older.id()).await.unwrap();
        assert!(none.is_empty(), "version {}", older.number());
    }
}

#[tokio::test]
async fn seed_plant_parameter_line_sits_on_the_main_unit_of_m7() {
    let (fixture, ids) = seeded();
    let machine = fixture.store.state().machine(ids.m7).unwrap();
    let main = machine.unit(&path("U1")).unwrap().id();
    let mut tx = begin(&fixture);
    let parameters = tx.recipes().version_parameters(ids.draft).await.unwrap();
    assert_eq!(parameters.first().unwrap().unit, main);
    assert!(parameters.first().unwrap().s7_address.is_some());
}

#[test]
fn seed_plant_m12_is_archived_with_a_live_u1_and_an_archived_u2() {
    let (fixture, ids) = seeded();
    let machine = fixture.store.state().machine(ids.m12).unwrap();
    assert_eq!(machine.code(), MachineCode::parse("M12").unwrap());
    assert!(!machine.is_live());
    assert!(machine.archived_at().is_some());
    let u1 = machine.unit(&path("U1")).unwrap();
    let u2 = machine.unit(&path("U2")).unwrap();
    assert!(u1.is_live());
    assert!(!u2.is_live());
    assert!(u2.archived_at().is_some());
    assert_eq!(machine.units().count(), 2);
    assert_eq!(u1.id(), ids.m12_live_unit);
    assert_eq!(u2.id(), ids.m12_archived_unit);
}

#[tokio::test]
async fn seed_plant_finds_m7_by_code_and_m12_only_by_identifier() {
    let (fixture, ids) = seeded();
    let mut tx = begin(&fixture);
    let m7 = MachineCode::parse("M7").unwrap();
    let m12 = MachineCode::parse("M12").unwrap();
    assert!(tx.plant().machine_by_code(m7).await.unwrap().is_some());
    assert!(tx.plant().machine_by_code(m12).await.unwrap().is_none());
    let archived = tx.plant().machine_by_id(ids.m12).await.unwrap();
    assert!(archived.is_some_and(|machine| !machine.is_live()));
}
