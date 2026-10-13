use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlcPlatform, UnitPath, UnitUsage};
use serde_json::json;

use super::{AddUnit, AddUnitInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing, StorageError};
use crate::memory::fixtures::{Fixture, actor, lock_line, meta, mid, text, timestamp, uid};
use crate::memory::{Fault, JournalEntry, LockMode};
use crate::ports::Isolation;

fn input(code: &str, parent: Option<&str>, segment: &str) -> AddUnitInput {
    AddUnitInput {
        code: MachineCode::parse(code).unwrap(),
        parent_path: parent.map(|path| UnitPath::parse(path).unwrap()),
        segment: segment.to_owned(),
        name: text("Child"),
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(AddUnit::OPERATION, Operation::AddUnit);
}

#[tokio::test]
async fn the_journal_holds_the_machine_lock_and_then_the_parent_lock() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    AddUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", Some("U1.2"), "2"),
        )
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForShare, mid(1).to_string()),
            lock_line("unit", LockMode::ForShare, uid(102).to_string()),
        ]
    );
}

#[tokio::test]
async fn success_under_a_parent_adds_the_unit_and_writes_one_create_entry() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let output = AddUnit::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", Some("U1.2"), "2"),
        )
        .await
        .unwrap();
    let path = UnitPath::parse("U1.2.2").unwrap();
    assert_eq!(output.path, path);
    let machine = fixture.store.state().machine(mid(1)).unwrap();
    let unit = machine.unit(&path).unwrap();
    assert_eq!(unit.id(), output.unit);
    assert_eq!(unit.name(), &text("Child"));
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::UnitAdded);
    assert_eq!(entry.target().entity_type(), "unit");
    assert_eq!(entry.target().entity_id(), output.unit.as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Machine);
    assert_eq!(entry.target().aggregate_id(), mid(1).as_uuid());
    assert!(entry.before().is_none());
    assert_eq!(entry.after().unwrap().get("path"), Some(&json!("U1.2.2")));
    assert_eq!(entry.after().unwrap().get("name"), Some(&json!("Child")));
}

#[tokio::test]
async fn a_main_unit_needs_no_parent_lock() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let output = AddUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1", None, "2"))
        .await
        .unwrap();
    assert_eq!(output.path, UnitPath::parse("U2").unwrap());
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForShare, mid(1).to_string()),
        ]
    );
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn an_unknown_code_gives_web_005() {
    let fixture = Fixture::new();
    let error = AddUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M9", None, "2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound {
            entity: Missing::Machine(_)
        }
    ));
}

#[tokio::test]
async fn a_sixth_level_gives_plt_002_and_writes_nothing() {
    let fixture = Fixture::new();
    let mut machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let level3 = UnitPath::parse("U1.2.1").unwrap();
    let level4 = machine
        .add_unit(uid(104), Some(&level3), "1", text("Four"))
        .unwrap()
        .path()
        .clone();
    machine
        .add_unit(uid(105), Some(&level4), "1", text("Five"))
        .unwrap();
    fixture.store.seed_machine(&machine);
    let error = AddUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", Some("U1.2.1.1.1"), "1"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_002);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_missing_parent_gives_plt_003_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = AddUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", Some("U1.9"), "1"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_003);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(
        fixture
            .store
            .state()
            .machine(mid(1))
            .unwrap()
            .units()
            .count(),
        3
    );
}

#[tokio::test]
async fn a_known_path_gives_plt_004_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = AddUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", Some("U1.2"), "1"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_004);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_archived_parent_gives_plt_009_and_writes_nothing() {
    let fixture = Fixture::new();
    let mut machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    machine
        .archive_unit(
            &UnitPath::parse("U1.2.1").unwrap(),
            timestamp(),
            &UnitUsage::default(),
        )
        .unwrap();
    fixture.store.seed_machine(&machine);
    let error = AddUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", Some("U1.2.1"), "1"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_009);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_machine_archived_between_the_read_and_the_lock_gives_plt_010() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived.archive(timestamp(), None).unwrap();
        store.seed_machine(&archived);
    });
    let error = AddUnit::new(context)
        .execute(&actor(Role::Administrator), &meta(), input("M1", None, "2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_malformed_segment_gives_plt_017_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = AddUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", Some("U1.2"), "x"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_017);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_constraint_unit_path_unique_gives_plt_004() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.store.fail_insert_unit(Fault::DuplicateUnitPath);
    let error = AddUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", Some("U1.2"), "2"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_004);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_constraint_unit_parent_exists_gives_plt_003() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.store.fail_insert_unit(Fault::MissingUnitParent);
    let error = AddUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", Some("U1.2"), "2"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_003);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unknown_foreign_key_gives_db_007_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture
        .store
        .fail_insert_unit(Fault::UnknownMissingReference);
    let error = AddUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", Some("U1.2"), "2"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_007);
    assert!(matches!(
        error,
        AppError::Storage(StorageError::MissingReference { .. })
    ));
    assert!(fixture.store.audit_entries().is_empty());
    let stored = fixture.store.state().machine(mid(1)).unwrap();
    assert!(stored.unit(&UnitPath::parse("U1.2.2").unwrap()).is_none());
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = AddUnit::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1", None, "2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
}
