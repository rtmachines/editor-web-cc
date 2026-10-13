use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlcPlatform, UnitPath, UnitUsage};
use serde_json::json;

use super::{RenameUnit, RenameUnitInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{Fixture, actor, lock_line, meta, mid, text, timestamp, uid};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;

fn input(code: &str, path: &str, name: &str) -> RenameUnitInput {
    RenameUnitInput {
        code: MachineCode::parse(code).unwrap(),
        path: UnitPath::parse(path).unwrap(),
        name: text(name),
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(RenameUnit::OPERATION, Operation::RenameUnit);
}

#[tokio::test]
async fn the_journal_holds_the_machine_lock_and_then_the_unit_lock() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    RenameUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", "U1.2", "Second"),
        )
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForNoKeyUpdate, mid(1).to_string()),
            lock_line("unit", LockMode::ForUpdate, uid(102).to_string()),
        ]
    );
}

#[tokio::test]
async fn success_renames_the_unit_and_writes_one_entry_with_both_images() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let output = RenameUnit::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", "U1.2", "Second"),
        )
        .await
        .unwrap();
    assert_eq!(output.unit, uid(102));
    let machine = fixture.store.state().machine(mid(1)).unwrap();
    let unit = machine.unit(&UnitPath::parse("U1.2").unwrap()).unwrap();
    assert_eq!(unit.name(), &text("Second"));
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::UnitRenamed);
    assert_eq!(entry.target().entity_type(), "unit");
    assert_eq!(entry.target().entity_id(), uid(102).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Machine);
    assert_eq!(entry.target().aggregate_id(), mid(1).as_uuid());
    assert_eq!(entry.before().unwrap().get("name"), Some(&json!("Two")));
    assert_eq!(entry.after().unwrap().get("name"), Some(&json!("Second")));
}

#[tokio::test]
async fn an_unknown_code_gives_web_005() {
    let fixture = Fixture::new();
    let error = RenameUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M9", "U1", "X"))
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
async fn an_unknown_path_gives_web_005_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = RenameUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", "U1.9", "X"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound {
            entity: Missing::Unit { .. }
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_archived_unit_gives_plt_009_and_writes_nothing() {
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
    let error = RenameUnit::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", "U1.2.1", "X"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_009);
    assert!(fixture.store.audit_entries().is_empty());
    let stored = fixture.store.state().machine(mid(1)).unwrap();
    let unit = stored.unit(&UnitPath::parse("U1.2.1").unwrap()).unwrap();
    assert_eq!(unit.name(), &text("Sub"));
}

#[tokio::test]
async fn a_unit_archived_between_the_read_and_the_lock_gives_plt_009() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived
            .archive_unit(
                &UnitPath::parse("U1.2.1").unwrap(),
                timestamp(),
                &UnitUsage::default(),
            )
            .unwrap();
        store.seed_machine(&archived);
    });
    let error = RenameUnit::new(context)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", "U1.2.1", "Late"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_009);
    assert!(fixture.store.audit_entries().is_empty());
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
    let error = RenameUnit::new(context)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", "U1.2", "Late"),
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
    let stored = fixture.store.state().machine(mid(1)).unwrap();
    let unit = stored.unit(&UnitPath::parse("U1.2").unwrap()).unwrap();
    assert_eq!(unit.name(), &text("Two"));
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = RenameUnit::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1", "U1.2", "X"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
}
