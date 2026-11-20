use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlcPlatform, StringEncoding};
use serde_json::json;

use super::{SetMachineStringEncoding, SetMachineStringEncodingInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{
    Fixture, actor, lock_line, meta, mid, scenarios, timestamp, utf8_machine,
};
use crate::memory::{Fault, JournalEntry, LockMode};
use crate::ports::Isolation;

fn input(code: &str, string_encoding: StringEncoding) -> SetMachineStringEncodingInput {
    SetMachineStringEncodingInput {
        code: MachineCode::parse(code).unwrap(),
        string_encoding,
    }
}

fn stored_encoding(fixture: &Fixture) -> StringEncoding {
    fixture
        .store
        .state()
        .machine(mid(1))
        .unwrap()
        .string_encoding()
}

fn read_only_journal() -> Vec<JournalEntry> {
    vec![JournalEntry::Begin(Isolation::ReadOnly)]
}

#[test]
fn adr_0202_set_machine_string_encoding_declares_its_operation() {
    assert_eq!(
        SetMachineStringEncoding::OPERATION,
        Operation::SetMachineStringEncoding
    );
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_forbids_an_operator_with_one_event_and_no_transaction()
 {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(stored_encoding(&fixture), StringEncoding::Latin1);
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_checks_the_permission_before_the_platform() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    let error = SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(fixture.store.journal().is_empty());
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_with_a_failing_security_sink_changes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.sink.fail_with(Fault::Unavailable);
    let error = SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert!(fixture.store.journal().is_empty());
    assert_eq!(stored_encoding(&fixture), StringEncoding::Latin1);
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_changes_latin1_to_utf8_and_writes_one_entry() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let output = SetMachineStringEncoding::new(fixture.context())
        .execute(&user, &request, input("M1", StringEncoding::Utf8))
        .await
        .unwrap();
    assert_eq!(output.machine, mid(1));
    assert_eq!(output.string_encoding, StringEncoding::Utf8);
    assert_eq!(stored_encoding(&fixture), StringEncoding::Utf8);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::MachineStringEncodingSet);
    assert_eq!(entry.target().entity_type(), "machine");
    assert_eq!(entry.target().entity_id(), mid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Machine);
    assert_eq!(
        entry.before().unwrap().get("string_encoding"),
        Some(&json!("Latin1"))
    );
    assert_eq!(
        entry.after().unwrap().get("string_encoding"),
        Some(&json!("Utf8"))
    );
    assert_eq!(entry.actor(), Some(user.user()));
    assert_eq!(entry.request_id(), request.request_id());
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_changes_utf8_back_to_latin1() {
    let fixture = Fixture::new();
    fixture.store.seed_machine(&utf8_machine(1, "M1"));
    SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", StringEncoding::Latin1),
        )
        .await
        .unwrap();
    assert_eq!(stored_encoding(&fixture), StringEncoding::Latin1);
    let entries = fixture.store.audit_entries();
    let entry = entries.first().unwrap();
    assert_eq!(
        entry.before().unwrap().get("string_encoding"),
        Some(&json!("Utf8"))
    );
    assert_eq!(
        entry.after().unwrap().get("string_encoding"),
        Some(&json!("Latin1"))
    );
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_with_the_current_value_still_writes_one_entry() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::CodesysIec, 1, "M1");
    SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", StringEncoding::Latin1),
        )
        .await
        .unwrap();
    assert_eq!(stored_encoding(&fixture), StringEncoding::Latin1);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::MachineStringEncodingSet);
    assert_eq!(entry.before(), entry.after());
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_utf8_on_an_s7_machine_gives_plt_053_before_the_transaction()
 {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let fixture = Fixture::new();
        fixture.seed_machine_on(platform, 1, "M1");
        let error = SetMachineStringEncoding::new(fixture.context())
            .execute(
                &actor(Role::ProcessEngineer),
                &meta(),
                input("M1", StringEncoding::Utf8),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::PLT_053, "{platform:?}");
        assert_eq!(fixture.store.journal(), read_only_journal());
        assert!(fixture.store.audit_entries().is_empty());
        assert_eq!(stored_encoding(&fixture), StringEncoding::Latin1);
    }
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_latin1_on_an_s7_machine_is_accepted() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", StringEncoding::Latin1),
        )
        .await
        .unwrap();
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_unknown_code_gives_web_005_and_no_command_transaction()
 {
    let fixture = Fixture::new();
    let error = SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M9", StringEncoding::Utf8),
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
    assert_eq!(fixture.store.journal(), read_only_journal());
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_with_a_draft_gives_plt_054_and_keeps_the_encoding() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_054);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(stored_encoding(&fixture), StringEncoding::Latin1);
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_with_a_deleted_recipe_gives_plt_054() {
    let fixture = scenarios::deleted();
    let error = SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_054);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(stored_encoding(&fixture), StringEncoding::Latin1);
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_with_an_approved_version_gives_plt_054() {
    let fixture = scenarios::approved_and_draft();
    let error = SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_054);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_ignores_the_versions_of_another_machine() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let other = fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.seed_draft(&other, 2, "R-2");
    SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
        .await
        .unwrap();
    assert_eq!(stored_encoding(&fixture), StringEncoding::Utf8);
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_archived_under_the_lock_gives_plt_010_and_writes_nothing()
 {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived.archive(timestamp(), None).unwrap();
        store.seed_machine(&archived);
    });
    let error = SetMachineStringEncoding::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(stored_encoding(&fixture), StringEncoding::Latin1);
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_checks_the_archive_before_the_version() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived.archive(timestamp(), None).unwrap();
        store.seed_machine(&archived);
    });
    let error = SetMachineStringEncoding::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
}

#[tokio::test]
async fn adr_0202_set_machine_string_encoding_journal_holds_the_read_the_transaction_and_the_lock()
{
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    SetMachineStringEncoding::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", StringEncoding::Utf8),
        )
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
