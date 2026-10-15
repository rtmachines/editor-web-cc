use std::collections::BTreeSet;

use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    MachineCode, MemberName, PlantError, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive,
    RecordShape, TypeExpr, TypeMember, TypeName,
};
use serde_json::json;

use super::{DefineUdt, DefineUdtInput, DefineUdtMode};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{Fixture, actor, lock_line, meta, mid, tid, timestamp};
use crate::memory::{DERIVED_TYPE_LOCK_TABLE, Fault, JournalEntry, LockMode, seed_type_id};
use crate::ports::Isolation;

fn input(mode: DefineUdtMode, name: &str, members: &[(&str, &str)]) -> DefineUdtInput {
    DefineUdtInput {
        code: MachineCode::parse("M1").unwrap(),
        name: TypeName::parse(name).unwrap(),
        members: members
            .iter()
            .map(|(member, type_expr)| {
                (
                    MemberName::parse(member).unwrap(),
                    TypeExpr::parse(type_expr).unwrap(),
                )
            })
            .collect(),
        mode,
    }
}

fn zone_input(mode: DefineUdtMode) -> DefineUdtInput {
    input(mode, "ST_Zone", &[("setpoint", "REAL")])
}

fn fixture(platform: PlcPlatform) -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_machine_on(platform, 1, "M1");
    fixture
}

/// A stored struct of machine 1 on `TwinCat3`.
fn stored_struct(
    id: u128,
    (name, revision): (&str, u32),
    supersedes: Option<u128>,
    members: &[(&str, PlcTypeId)],
) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision,
            supersedes: supersedes.map(tid),
            members: members
                .iter()
                .map(|(member, type_id)| TypeMember {
                    name: MemberName::parse(member).unwrap(),
                    type_id: *type_id,
                })
                .collect(),
        },
    })
    .unwrap()
}

/// The name, the revision, the superseded row, and the members of a struct row.
fn struct_parts(row: &PlcType) -> (String, u32, Option<PlcTypeId>, Vec<TypeMember>) {
    match row.shape() {
        RecordShape::Struct {
            name,
            revision,
            supersedes,
            members,
        } => Some((name.to_string(), *revision, *supersedes, members.clone())),
        _ => None,
    }
    .unwrap()
}

fn real() -> PlcTypeId {
    seed_type_id(Primitive::Real, None).unwrap()
}

/// The rows of machine 1 that the committed state holds.
fn own_rows(fixture: &Fixture) -> Vec<PlcType> {
    fixture
        .store
        .state()
        .types
        .values()
        .map(|row| row.plc_type.clone())
        .filter(|row| row.machine() == Some(mid(1)))
        .collect()
}

async fn run(
    fixture: &Fixture,
    role: Role,
    input: DefineUdtInput,
) -> Result<super::DefineUdtOutput, AppError> {
    DefineUdt::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(DefineUdt::OPERATION, Operation::DefineUdt);
}

#[tokio::test]
async fn a_new_struct_gets_revision_1_and_no_supersedes() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Zone",
            &[("setpoint", "REAL"), ("label", "STRING(16)")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
    let rows = own_rows(&fixture);
    let row = rows.iter().find(|row| row.id() == output.type_id).unwrap();
    assert_eq!(row.platforms(), &BTreeSet::from([PlcPlatform::TwinCat3]));
    let (name, revision, supersedes, members) = struct_parts(row);
    assert_eq!(name.as_str(), "ST_Zone");
    assert_eq!(revision, 1);
    assert!(supersedes.is_none());
    let names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
    assert_eq!(names, vec!["setpoint", "label"]);
    assert_eq!(members.first().unwrap().type_id, real());
}

#[tokio::test]
async fn a_revision_gets_the_next_number_and_supersedes_the_last_one() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture.store.seed_type(&stored_struct(
        901,
        ("ST_Zone", 1),
        None,
        &[("setpoint", real())],
    ));
    fixture.store.seed_type(&stored_struct(
        902,
        ("ST_Zone", 2),
        Some(901),
        &[("setpoint", real())],
    ));
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        zone_input(DefineUdtMode::Revision),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 3);
    let rows = own_rows(&fixture);
    let row = rows.iter().find(|row| row.id() == output.type_id).unwrap();
    assert!(matches!(
        row.shape(),
        RecordShape::Struct { revision: 3, supersedes: Some(previous), .. } if *previous == tid(902)
    ));
}

#[tokio::test]
async fn the_journal_holds_the_machine_lock_and_then_the_sorted_advisory_keys() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Zone",
            &[("label", "STRING(16)"), ("temps", "ARRAY[1..2] OF REAL")],
        ),
    )
    .await
    .unwrap();
    let advisory = |name: &str| {
        lock_line(
            DERIVED_TYPE_LOCK_TABLE,
            LockMode::Advisory,
            format!("{}:{name}", mid(1)),
        )
    };
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForShare, mid(1).to_string()),
            advisory("ST_Zone"),
            advisory("STRING"),
            advisory("ARRAY"),
        ]
    );
}

#[tokio::test]
async fn the_entries_hold_the_created_derived_types_and_then_the_struct_with_its_members() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Zone",
            &[("setpoint", "REAL"), ("label", "STRING(16)")],
        ),
    )
    .await
    .unwrap();
    let entries = fixture.store.audit_entries();
    let actions: Vec<AuditAction> = entries.iter().map(|entry| entry.action()).collect();
    assert_eq!(
        actions,
        vec![AuditAction::DerivedTypeCreated, AuditAction::UdtDefined]
    );
    let created = entries.first().unwrap();
    let defined = entries.get(1).unwrap();
    assert_eq!(defined.target().entity_type(), "plc_type");
    assert_eq!(defined.target().entity_id(), output.type_id.as_uuid());
    assert_eq!(defined.target().aggregate_type(), AggregateType::Machine);
    assert_eq!(defined.target().aggregate_id(), mid(1).as_uuid());
    assert!(defined.before().is_none());
    let after = defined.after().unwrap();
    assert_eq!(after.get("name"), Some(&json!("ST_Zone")));
    assert_eq!(after.get("revision"), Some(&json!(1)));
    let members = after.get("members").unwrap().as_array().unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(
        members.first().unwrap().get("member_name"),
        Some(&json!("setpoint"))
    );
    assert_eq!(
        members.get(1).unwrap().get("member_name"),
        Some(&json!("label"))
    );
    assert_eq!(
        members.get(1).unwrap().get("member_type_id"),
        Some(&json!(created.target().entity_id().to_string()))
    );
}

#[tokio::test]
async fn a_new_struct_with_a_used_name_gives_plt_015_and_writes_nothing() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture.store.seed_type(&stored_struct(
        901,
        ("ST_Zone", 1),
        None,
        &[("setpoint", real())],
    ));
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Zone", &[("label", "STRING(16)")]),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_015);
    assert!(matches!(
        error,
        AppError::Plant(PlantError::DuplicateTypeName { .. })
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(own_rows(&fixture).len(), 1);
}

#[tokio::test]
async fn the_constraint_plc_type_named_unique_gives_plt_015() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture.store.fail_insert_struct(Fault::DuplicateTypeName);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        zone_input(DefineUdtMode::New),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_015);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(own_rows(&fixture).is_empty());
}

#[tokio::test]
async fn the_constraint_plc_type_member_name_unique_gives_plt_021_with_the_struct_and_member() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture.store.fail_insert_struct(Fault::DuplicateMemberName);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Zone",
            &[("speed", "INT"), ("label", "REAL")],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_021);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::DuplicateMemberName { type_name, member })
            if type_name.as_str() == "ST_Zone" && member.as_str() == "speed"
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert!(own_rows(&fixture).is_empty());
}

#[tokio::test]
async fn a_revision_of_an_unknown_name_gives_web_005() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        zone_input(DefineUdtMode::Revision),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound {
            entity: Missing::Type { .. }
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unknown_machine_gives_web_005() {
    let fixture = Fixture::new();
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        zone_input(DefineUdtMode::New),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AppError::NotFound {
            entity: Missing::Machine(_)
        }
    ));
}

#[tokio::test]
async fn a_member_type_not_on_the_platform_gives_plt_006_with_the_member() {
    let fixture = fixture(PlcPlatform::S7_300);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Zone",
            &[("setpoint", "REAL"), ("label", "WSTRING(10)")],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_006);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::MemberTypeNotOnPlatform { type_name, member, platform })
            if type_name.as_str() == "ST_Zone"
                && member.as_str() == "label"
                && platform == &PlcPlatform::S7_300
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unknown_member_struct_gives_plt_013() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Zone", &[("inner", "ST_Missing@1")]),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
}

#[tokio::test]
async fn a_member_length_out_of_range_stays_plt_013() {
    let fixture = fixture(PlcPlatform::S7_1500);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Zone", &[("label", "STRING(300)")]),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
}

#[tokio::test]
async fn a_struct_without_members_gives_plt_016() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Zone", &[]),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_016);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_struct_with_1025_members_gives_plt_018() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let names: Vec<String> = (0..1025).map(|n| format!("m{n}")).collect();
    let members: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), "INT")).collect();
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Big", &members),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_018);
}

#[tokio::test]
async fn two_members_with_one_name_give_plt_021() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Zone",
            &[("a", "INT"), ("a", "REAL")],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_021);
}

#[tokio::test]
async fn a_member_whose_struct_rows_form_a_cycle_gives_plt_005() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture
        .store
        .seed_type(&stored_struct(911, ("ST_A", 1), None, &[("b", tid(912))]));
    fixture
        .store
        .seed_type(&stored_struct(912, ("ST_B", 1), None, &[("a", tid(911))]));
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_C", &[("a", "ST_A@1")]),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_005);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_revision_with_a_member_of_an_earlier_revision_gives_plt_005_and_creates_no_type() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture
        .store
        .seed_type(&stored_struct(921, ("ST_A", 1), None, &[("v", real())]));
    let before = own_rows(&fixture).len();
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::Revision, "ST_A", &[("old", "ST_A@1")]),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_005);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::TypeCycle { type_name }) if type_name.as_str() == "ST_A"
    ));
    assert_eq!(own_rows(&fixture).len(), before);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn one_struct_on_two_member_paths_is_not_a_cycle() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture
        .store
        .seed_type(&stored_struct(931, ("ST_Inner", 1), None, &[("v", real())]));
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Outer",
            &[("a", "ST_Inner@1"), ("b", "ST_Inner@1")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}

#[tokio::test]
async fn a_member_array_above_the_leaf_limit_gives_plt_018_and_writes_nothing() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Zone",
            &[("big", "ARRAY[1..16385] OF INT")],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_018);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(own_rows(&fixture).is_empty());
}

#[tokio::test]
async fn a_machine_archived_between_the_read_and_the_lock_gives_web_005_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived.archive(timestamp(), None).unwrap();
        store.seed_machine(&archived);
    });
    let error = DefineUdt::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            zone_input(DefineUdtMode::New),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound {
            entity: Missing::Machine(_)
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert!(own_rows(&fixture).is_empty());
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let error = run(&fixture, Role::Operator, zone_input(DefineUdtMode::New))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
}
