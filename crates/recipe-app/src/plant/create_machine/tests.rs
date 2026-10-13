use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlcPlatform, UnitPath};
use serde_json::json;

use super::{CreateMachine, CreateMachineInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::memory::fixtures::{Fixture, actor, meta, text};
use crate::plant::short_rows::short_rows_context;

fn input(code: &str) -> CreateMachineInput {
    CreateMachineInput {
        code: MachineCode::parse(code).unwrap(),
        name: text("Press 7"),
        platform: PlcPlatform::S7_300,
        main_unit_name: text("Feeder"),
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(CreateMachine::OPERATION, Operation::CreateMachine);
}

#[tokio::test]
async fn success_stores_the_machine_with_u1_named_as_the_request() {
    let fixture = Fixture::new();
    let output = CreateMachine::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M7"))
        .await
        .unwrap();
    let machine = fixture.store.state().machine(output.machine).unwrap();
    assert_eq!(machine.code(), MachineCode::parse("M7").unwrap());
    assert_eq!(machine.name(), &text("Press 7"));
    assert_eq!(machine.platform(), PlcPlatform::S7_300);
    let unit = machine.unit(&UnitPath::parse("U1").unwrap()).unwrap();
    assert_eq!(unit.id(), output.main_unit);
    assert_eq!(unit.name(), &text("Feeder"));
}

#[tokio::test]
async fn success_writes_the_two_entries_with_after_images_in_the_machine_aggregate() {
    let fixture = Fixture::new();
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let output = CreateMachine::new(fixture.context())
        .execute(&user, &request, input("M7"))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    let actions: Vec<AuditAction> = entries.iter().map(|entry| entry.action()).collect();
    assert_eq!(
        actions,
        vec![AuditAction::MachineCreated, AuditAction::UnitAdded]
    );
    let machine_entry = entries.first().unwrap();
    assert_eq!(machine_entry.target().entity_type(), "machine");
    assert_eq!(machine_entry.target().entity_id(), output.machine.as_uuid());
    assert_eq!(machine_entry.before(), None);
    let after = machine_entry.after().unwrap();
    assert_eq!(after.get("code"), Some(&json!("M7")));
    assert_eq!(after.get("name"), Some(&json!("Press 7")));
    let unit_entry = entries.get(1).unwrap();
    assert_eq!(unit_entry.target().entity_type(), "unit");
    assert_eq!(unit_entry.target().entity_id(), output.main_unit.as_uuid());
    assert_eq!(unit_entry.before(), None);
    assert_eq!(unit_entry.after().unwrap().get("path"), Some(&json!("U1")));
    assert_eq!(
        unit_entry.after().unwrap().get("name"),
        Some(&json!("Feeder"))
    );
    for entry in &entries {
        assert_eq!(entry.target().aggregate_type(), AggregateType::Machine);
        assert_eq!(entry.target().aggregate_id(), output.machine.as_uuid());
        assert_eq!(entry.actor(), Some(user.user()));
        assert_eq!(entry.request_id(), request.request_id());
        assert_eq!(entry.client_ip(), request.client_ip());
        assert_eq!(entry.session_ref(), request.session_ref());
    }
}

#[tokio::test]
async fn a_duplicate_live_code_gives_plt_001_and_writes_no_entry() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M7");
    let machines_before = fixture.store.state().machines.len();
    let error = CreateMachine::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M7"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_001);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(fixture.store.state().machines.len(), machines_before);
}

#[tokio::test]
async fn an_adapter_that_returns_too_few_images_gives_db_008_and_commits_nothing() {
    let fixture = Fixture::new();
    let machines_before = fixture.store.state().machines.len();
    let error = CreateMachine::new(short_rows_context(&fixture))
        .execute(&actor(Role::Administrator), &meta(), input("M7"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(fixture.store.state().machines.len(), machines_before);
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    let error = CreateMachine::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M7"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
}
