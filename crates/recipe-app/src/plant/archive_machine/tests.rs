use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlantError, PlcPlatform};
use serde_json::Value;

use super::{ArchiveMachine, ArchiveMachineInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{
    Fixture, actor, draft_recipe, lock_line, meta, mid, reference, rid, timestamp, user,
};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;

fn input(code: &str) -> ArchiveMachineInput {
    ArchiveMachineInput {
        code: MachineCode::parse(code).unwrap(),
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(ArchiveMachine::OPERATION, Operation::ArchiveMachine);
}

#[tokio::test]
async fn the_journal_holds_the_machine_lock_and_then_the_recipe_locks_in_id_order() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let other = fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.seed_retired(&machine, 3, "R-3");
    fixture.seed_retired(&machine, 1, "R-1");
    fixture.seed_retired(&machine, 2, "R-2");
    fixture.seed_retired(&other, 4, "R-4");
    ArchiveMachine::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1"))
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForNoKeyUpdate, mid(1).to_string()),
            lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
            lock_line("recipe", LockMode::ForUpdate, rid(2).to_string()),
            lock_line("recipe", LockMode::ForUpdate, rid(3).to_string()),
        ]
    );
}

#[tokio::test]
async fn success_archives_the_machine_and_writes_one_update_entry() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_retired(&machine, 1, "R-1");
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let output = ArchiveMachine::new(fixture.context())
        .execute(&user, &request, input("M1"))
        .await
        .unwrap();
    assert_eq!(output.machine, mid(1));
    assert!(!fixture.store.state().machine(mid(1)).unwrap().is_live());
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::MachineArchived);
    assert_eq!(entry.target().entity_type(), "machine");
    assert_eq!(entry.target().entity_id(), mid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Machine);
    assert_eq!(
        entry.before().unwrap().get("archived_at"),
        Some(&Value::Null)
    );
    assert_ne!(
        entry.after().unwrap().get("archived_at"),
        Some(&Value::Null)
    );
    assert_eq!(entry.actor(), Some(user.user()));
    assert_eq!(entry.request_id(), request.request_id());
}

#[tokio::test]
async fn a_live_draft_gives_plt_011_with_its_reference_and_archives_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_retired(&machine, 1, "R-1");
    fixture.seed_draft(&machine, 2, "R-2");
    let error = ArchiveMachine::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_011);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::MachineHasDraft { reference: found, .. })
            if *found == reference("R-2")
    ));
    assert!(fixture.store.state().machine(mid(1)).unwrap().is_live());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_draft_that_appears_after_the_read_is_found_under_the_lock() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    // The hook runs after the read of step 2 and before the locks of step 4.
    let context = fixture.context_racing(|store| {
        store.seed_recipe(
            &draft_recipe((5, 1), "R-9", 501),
            PlcPlatform::TwinCat3,
            user(2),
        );
    });
    let error = ArchiveMachine::new(context)
        .execute(&actor(Role::Administrator), &meta(), input("M1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_011);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::MachineHasDraft { reference: found, .. })
            if *found == reference("R-9")
    ));
    assert!(fixture.store.state().machine(mid(1)).unwrap().is_live());
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
    let error = ArchiveMachine::new(context)
        .execute(&actor(Role::Administrator), &meta(), input("M1"))
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
}

#[tokio::test]
async fn an_unknown_code_gives_web_005() {
    let fixture = Fixture::new();
    let error = ArchiveMachine::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M9"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = ArchiveMachine::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.state().machine(mid(1)).unwrap().is_live());
}
