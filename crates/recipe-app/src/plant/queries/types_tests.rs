//! Tests of `ListTypes` and `GetTypeTree` on `MemoryStore`.
//!
//! The specs are "Type list of a machine" and "Type tree of a revision" (design D3).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    MachineCode, PlantError, PlcPlatform, PlcTypeId, Primitive, TypeName, TypeNode,
};

use super::TypeRef;
use super::types::{GetTypeTree, ListTypes, TypeList, TypeListRow, TypeView, TypesInput};
use super::world::{archived, array_row, machine_of, struct_row};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{Fixture, actor, meta, tid};
use crate::memory::{JournalEntry, seed_type_id};
use crate::ports::Isolation;

/// The number of the machine `M12`.
const MACHINE: u128 = 12;
/// The platform of the machine `M12`.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;
/// The struct `ST_Zone@1`.
const ZONE_1: u128 = 901;
/// The struct `ST_Zone@2`. It supersedes `ZONE_1`.
const ZONE_2: u128 = 902;
/// The type `ARRAY[1..4] OF ST_Zone@2`.
const ZONES: u128 = 903;
/// The struct `ST_Line@1` with the member `zones` of the type `ZONES`.
const LINE: u128 = 904;
/// The struct `ST_Axis@1` with two members.
const AXIS_1: u128 = 910;

fn code(value: &str) -> MachineCode {
    MachineCode::parse(value).unwrap()
}

fn name(value: &str) -> TypeName {
    TypeName::parse(value).unwrap()
}

fn int() -> PlcTypeId {
    seed_type_id(Primitive::Int, None).unwrap()
}

const fn owner() -> (u128, PlcPlatform) {
    (MACHINE, PLATFORM)
}

/// The machine `M12` with `ST_Zone` in the revisions 1 and 2 and `ST_Axis` in the revision 1.
fn zones_world() -> Fixture {
    let fixture = Fixture::new();
    let machine = machine_of(PLATFORM, MACHINE, "M12", "Press");
    fixture.store.seed_machine(&machine);
    let temp = [("temp", int())];
    for row in [
        struct_row(owner(), ZONE_1, ("ST_Zone", 1), &temp, None),
        struct_row(owner(), ZONE_2, ("ST_Zone", 2), &temp, Some(ZONE_1)),
        struct_row(
            owner(),
            AXIS_1,
            ("ST_Axis", 1),
            &[("pos", int()), ("speed", int())],
            None,
        ),
    ] {
        fixture.store.seed_type(&row);
    }
    fixture
}

/// The world of `zones_world` and the struct `ST_Line@1` that holds an array of `ST_Zone@2`.
fn nested_world() -> Fixture {
    let fixture = zones_world();
    fixture.store.seed_type(&array_row(owner(), ZONES, ZONE_2));
    let line = struct_row(
        owner(),
        LINE,
        ("ST_Line", 1),
        &[("zones", tid(ZONES))],
        None,
    );
    fixture.store.seed_type(&line);
    fixture
}

async fn list(fixture: &Fixture, machine_code: &str) -> Result<TypeList, AppError> {
    let input = TypesInput {
        code: code(machine_code),
    };
    ListTypes::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input)
        .await
}

async fn tree(
    fixture: &Fixture,
    struct_name: &str,
    revision: Option<u32>,
) -> Result<TypeView, AppError> {
    let input = TypeRef {
        code: code("M12"),
        name: name(struct_name),
        revision,
    };
    GetTypeTree::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input)
        .await
}

fn row(id: u128, struct_name: &str, revision: u32, members: usize, latest: bool) -> TypeListRow {
    TypeListRow {
        id: tid(id),
        name: name(struct_name),
        revision,
        members,
        latest,
    }
}

fn labels(list: &TypeList) -> Vec<String> {
    list.rows
        .iter()
        .map(|row| format!("{}@{}", row.name, row.revision))
        .collect()
}

#[test]
fn the_queries_declare_their_operations() {
    assert_eq!(ListTypes::OPERATION, Operation::ListTypes);
    assert_eq!(GetTypeTree::OPERATION, Operation::GetTypeTree);
}

#[tokio::test]
async fn two_revisions_of_one_struct_come_with_the_latest_first_and_marked() {
    let fixture = zones_world();

    let result = list(&fixture, "M12").await.unwrap();

    let expected = [
        row(AXIS_1, "ST_Axis", 1, 2, true),
        row(ZONE_2, "ST_Zone", 2, 1, true),
        row(ZONE_1, "ST_Zone", 1, 1, false),
    ];
    assert_eq!(result.rows, expected);
    assert_eq!(result.machine.code(), code("M12"));
}

#[tokio::test]
async fn the_list_keeps_only_the_struct_rows_of_the_machine() {
    let fixture = nested_world();
    let other = machine_of(PLATFORM, 13, "M13", "Other");
    fixture.store.seed_machine(&other);
    let foreign = struct_row((13, PLATFORM), 930, ("ST_Other", 1), &[("x", int())], None);
    fixture.store.seed_type(&foreign);

    let result = list(&fixture, "M12").await.unwrap();

    let expected = ["ST_Axis@1", "ST_Line@1", "ST_Zone@2", "ST_Zone@1"];
    assert_eq!(labels(&result), expected);
}

#[tokio::test]
async fn the_names_come_in_byte_order() {
    let fixture = zones_world();
    let lower = struct_row(owner(), 920, ("ST_axis", 1), &[("x", int())], None);
    fixture.store.seed_type(&lower);

    let result = list(&fixture, "M12").await.unwrap();

    let expected = ["ST_Axis@1", "ST_Zone@2", "ST_Zone@1", "ST_axis@1"];
    assert_eq!(labels(&result), expected);
}

#[tokio::test]
async fn a_machine_without_a_struct_has_an_empty_list() {
    let fixture = Fixture::new();
    fixture
        .store
        .seed_machine(&machine_of(PLATFORM, MACHINE, "M12", "Press"));

    let result = list(&fixture, "M12").await.unwrap();

    assert!(result.rows.is_empty(), "{:?}", result.rows);
}

#[tokio::test]
async fn an_unknown_machine_code_is_not_found_for_the_list() {
    let fixture = zones_world();

    let error = list(&fixture, "M99").await.unwrap_err();

    let expected = Missing::Machine(code("M99"));
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn the_code_of_an_archived_machine_is_not_found_for_the_list() {
    let fixture = Fixture::new();
    let old = archived(machine_of(PLATFORM, MACHINE, "M12", "Old press"));
    fixture.store.seed_machine(&old);

    let error = list(&fixture, "M12").await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
}

#[tokio::test]
async fn the_list_opens_one_read_only_transaction_and_writes_nothing() {
    let fixture = zones_world();

    list(&fixture, "M12").await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_earlier_revision_gives_its_tree_and_lists_the_revisions_from_the_latest() {
    let fixture = zones_world();

    let view = tree(&fixture, "ST_Zone", Some(1)).await.unwrap();

    assert_eq!(view.id, tid(ZONE_1));
    assert_eq!((view.name.as_str(), view.revision), ("ST_Zone", 1));
    assert_eq!(view.revisions, [2, 1]);
    assert_eq!(view.supersedes, None);
    assert_eq!(view.tree.root_type(), tid(ZONE_1));
    assert_eq!(view.tree.leaf_count(), 1);
    assert_eq!(view.machine.code(), code("M12"));
}

#[tokio::test]
async fn no_revision_gives_the_latest_revision_with_the_revision_that_it_supersedes() {
    let fixture = zones_world();

    let view = tree(&fixture, "ST_Zone", None).await.unwrap();

    assert_eq!(view.id, tid(ZONE_2));
    assert_eq!(view.revision, 2);
    assert_eq!(view.revisions, [2, 1]);
    assert_eq!(view.supersedes, Some(1));
}

#[tokio::test]
async fn a_struct_with_one_revision_supersedes_nothing() {
    let fixture = zones_world();

    let view = tree(&fixture, "ST_Axis", None).await.unwrap();

    assert_eq!(view.revisions, [1]);
    assert_eq!(view.supersedes, None);
    assert_eq!(view.tree.leaf_count(), 2);
}

#[tokio::test]
async fn the_tree_resolves_a_struct_that_holds_an_array_of_a_struct() {
    let fixture = nested_world();

    let view = tree(&fixture, "ST_Line", Some(1)).await.unwrap();

    assert_eq!(view.tree.leaf_count(), 4);
    let root = view.tree.root();
    assert!(
        matches!(
            root,
            TypeNode::Struct { members, .. }
                if matches!(
                    members.first(),
                    Some((member, TypeNode::Array { .. })) if member.as_str() == "zones"
                )
        ),
        "{root:?}"
    );
}

#[tokio::test]
async fn the_tree_gives_the_text_of_each_member_type_as_type_text_writes_it() {
    let fixture = nested_world();
    let string = seed_type_id(Primitive::String, Some(80)).unwrap();
    let label = struct_row(
        owner(),
        950,
        ("ST_Label", 1),
        &[("label", string), ("zones", tid(ZONES))],
        None,
    );
    fixture.store.seed_type(&label);

    let view = tree(&fixture, "ST_Label", None).await.unwrap();

    assert_eq!(
        view.member_types.get(&string).map(String::as_str),
        Some("STRING")
    );
    let zones = view.member_types.get(&tid(ZONES)).map(String::as_str);
    assert_eq!(zones, Some("ARRAY[1..4] OF ST_Zone@2"));
    let zone = view.member_types.get(&tid(ZONE_2)).map(String::as_str);
    assert_eq!(zone, Some("ST_Zone@2"));
    assert_eq!(
        view.member_types.get(&int()).map(String::as_str),
        Some("INT")
    );
}

#[tokio::test]
async fn an_unknown_revision_is_not_found_with_the_revision() {
    let fixture = zones_world();

    let error = tree(&fixture, "ST_Zone", Some(9)).await.unwrap_err();

    let expected = Missing::TypeRevision {
        code: code("M12"),
        name: name("ST_Zone"),
        revision: 9,
    };
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_unknown_struct_is_not_found_with_the_name() {
    let fixture = zones_world();

    let error = tree(&fixture, "ST_Missing", Some(1)).await.unwrap_err();

    let expected = Missing::Type {
        code: code("M12"),
        name: name("ST_Missing"),
    };
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_struct_of_another_machine_is_not_found() {
    let fixture = zones_world();
    let other = machine_of(PLATFORM, 13, "M13", "Other");
    fixture.store.seed_machine(&other);
    let foreign = struct_row((13, PLATFORM), 930, ("ST_Other", 1), &[("x", int())], None);
    fixture.store.seed_type(&foreign);

    let error = tree(&fixture, "ST_Other", None).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
}

#[tokio::test]
async fn an_unknown_machine_is_not_found_for_the_tree() {
    let fixture = zones_world();
    let input = TypeRef {
        code: code("M99"),
        name: name("ST_Zone"),
        revision: None,
    };

    let error = GetTypeTree::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input)
        .await
        .unwrap_err();

    let expected = Missing::Machine(code("M99"));
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn two_structs_that_contain_each_other_give_the_cycle_error() {
    let fixture = zones_world();
    let left = struct_row(owner(), 940, ("ST_Left", 1), &[("right", tid(941))], None);
    let right = struct_row(owner(), 941, ("ST_Right", 1), &[("left", tid(940))], None);
    fixture.store.seed_type(&left);
    fixture.store.seed_type(&right);

    let error = tree(&fixture, "ST_Left", None).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_005);
    assert!(
        matches!(&error, AppError::Plant(PlantError::TypeCycle { .. })),
        "{error:?}"
    );
}

#[tokio::test]
async fn the_tree_query_opens_one_read_only_transaction_and_writes_nothing() {
    let fixture = nested_world();

    tree(&fixture, "ST_Line", None).await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}
