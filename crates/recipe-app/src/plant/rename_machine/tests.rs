use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlcPlatform};
use serde_json::json;

use super::{RenameMachine, RenameMachineInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{Fixture, actor, lock_line, meta, mid, text};
use crate::memory::{Fault, JournalEntry, LockMode};
use crate::ports::Isolation;

fn input(code: &str, name: &str) -> RenameMachineInput {
    RenameMachineInput {
        code: MachineCode::parse(code).unwrap(),
        name: text(name),
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(RenameMachine::OPERATION, Operation::RenameMachine);
}

#[tokio::test]
async fn the_journal_holds_the_read_the_transaction_and_the_machine_lock() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    RenameMachine::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1", "Press 2"))
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForNoKeyUpdate, mid(1).to_string()),
        ]
    );
}

#[tokio::test]
async fn success_renames_the_machine_and_writes_one_entry_with_both_images() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let output = RenameMachine::new(fixture.context())
        .execute(&user, &request, input("M1", "Press 2"))
        .await
        .unwrap();
    assert_eq!(output.machine, mid(1));
    let machine = fixture.store.state().machine(mid(1)).unwrap();
    assert_eq!(machine.name(), &text("Press 2"));
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::MachineRenamed);
    assert_eq!(entry.target().entity_type(), "machine");
    assert_eq!(entry.target().entity_id(), mid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Machine);
    assert_eq!(entry.before().unwrap().get("name"), Some(&json!("Press")));
    assert_eq!(entry.after().unwrap().get("name"), Some(&json!("Press 2")));
    assert_eq!(entry.actor(), Some(user.user()));
    assert_eq!(entry.request_id(), request.request_id());
}

#[tokio::test]
async fn an_unknown_code_gives_web_005_and_opens_no_command_transaction() {
    let fixture = Fixture::new();
    let error = RenameMachine::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M9", "X"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound {
            entity: Missing::Machine(_)
        }
    ));
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_machine_archived_between_the_read_and_the_lock_gives_plt_010_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    // The hook runs after the read of step 2 and before `begin(ReadCommitted)` of step 4.
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived
            .archive(crate::memory::fixtures::timestamp(), None)
            .unwrap();
        store.seed_machine(&archived);
    });
    let error = RenameMachine::new(context)
        .execute(&actor(Role::Administrator), &meta(), input("M1", "Late"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(fixture.store.audit_entries().is_empty());
    let stored = fixture.store.state().machine(mid(1)).unwrap();
    assert_eq!(stored.name(), &text("Press"));
}

#[tokio::test]
async fn a_forbidden_role_with_a_failing_security_sink_gives_the_storage_error_and_changes_nothing()
{
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.sink.fail_with(Fault::Unavailable);
    let error = RenameMachine::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1", "X"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert!(matches!(error, AppError::Storage(_)));
    assert!(fixture.sink.events().is_empty());
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    let stored = fixture.store.state().machine(mid(1)).unwrap();
    assert_eq!(stored.name(), &text("Press"));
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = RenameMachine::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1", "X"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
}
