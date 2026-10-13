use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::VersionState;
use serde_json::Value;

use super::{DiscardDraft, DiscardDraftInput, RetireVersion, RetireVersionInput, VersionOutput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{Fixture, actor, field, lock_line, meta, reference, rid, vid};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::test_support::seed_deleted;

fn discard(reference_text: &str, number: u32, expected: i64) -> DiscardDraftInput {
    DiscardDraftInput {
        reference: reference(reference_text),
        number,
        expected_row_version: expected,
    }
}

fn retire(reference_text: &str, number: u32, expected: i64) -> RetireVersionInput {
    RetireVersionInput {
        reference: reference(reference_text),
        number,
        expected_row_version: expected,
    }
}

fn locked_journal(version: u128) -> Vec<JournalEntry> {
    vec![
        JournalEntry::Begin(Isolation::ReadOnly),
        JournalEntry::Begin(Isolation::ReadCommitted),
        lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
        lock_line(
            "recipe_version",
            LockMode::ForUpdate,
            vid(version).to_string(),
        ),
    ]
}

#[test]
fn the_use_cases_declare_their_operations() {
    assert_eq!(DiscardDraft::OPERATION, Operation::DiscardDraft);
    assert_eq!(RetireVersion::OPERATION, Operation::RetireVersion);
}

#[tokio::test]
async fn discard_succeeds_with_one_entry_and_the_lock_journal() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let output = DiscardDraft::new(fixture.context())
        .execute(&user, &request, discard("R-1", 1, 1))
        .await
        .unwrap();
    assert_eq!(output, VersionOutput { version: vid(101) });
    assert_eq!(fixture.store.journal(), locked_journal(101));
    let state = fixture.store.state();
    let stored = state.recipe(rid(1)).unwrap();
    assert_eq!(stored.version(1).unwrap().state(), VersionState::Discarded);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::DraftDiscarded);
    assert_eq!(entry.target().entity_type(), "recipe_version");
    assert_eq!(entry.target().entity_id(), vid(101).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    assert_eq!(field(entry.before().unwrap(), "deleted_at"), Value::Null);
    assert_ne!(field(entry.after().unwrap(), "deleted_at"), Value::Null);
    assert_eq!(entry.actor(), Some(user.user()));
    assert_eq!(entry.request_id(), request.request_id());
}

#[tokio::test]
async fn discard_with_an_old_row_version_gives_web_001_with_both_versions() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = DiscardDraft::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), discard("R-1", 1, 5))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    assert!(matches!(
        &error,
        AppError::ConcurrentEdit {
            number: 1,
            expected: 5,
            actual: 1,
            ..
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
    let state = fixture.store.state();
    let stored = state.recipe(rid(1)).unwrap();
    assert_eq!(stored.version(1).unwrap().state(), VersionState::LiveDraft);
}

#[tokio::test]
async fn discard_of_an_approved_version_gives_rcp_007() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let error = DiscardDraft::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), discard("R-1", 1, 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_007);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn discard_in_a_deleted_recipe_gives_rcp_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = DiscardDraft::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), discard("R-1", 1, 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn discard_of_an_unknown_number_or_reference_gives_web_005() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let use_case = DiscardDraft::new(fixture.context());
    let user = actor(Role::ProcessEngineer);
    let number = use_case
        .execute(&user, &meta(), discard("R-1", 9, 1))
        .await
        .unwrap_err();
    assert_eq!(number.code(), ErrorCode::WEB_005);
    assert!(matches!(
        &number,
        AppError::NotFound {
            entity: Missing::Version { number: 9, .. }
        }
    ));
    let unknown = use_case
        .execute(&user, &meta(), discard("R-9", 1, 1))
        .await
        .unwrap_err();
    assert!(matches!(
        &unknown,
        AppError::NotFound {
            entity: Missing::Recipe(_)
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_operator_cannot_discard() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = DiscardDraft::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), discard("R-1", 1, 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
}

#[tokio::test]
async fn retire_succeeds_with_one_entry_and_the_lock_journal() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let user = actor(Role::Administrator);
    let output = RetireVersion::new(fixture.context())
        .execute(&user, &meta(), retire("R-1", 1, 1))
        .await
        .unwrap();
    assert_eq!(output, VersionOutput { version: vid(101) });
    assert_eq!(fixture.store.journal(), locked_journal(101));
    let state = fixture.store.state();
    let stored = state.recipe(rid(1)).unwrap();
    assert_eq!(stored.version(1).unwrap().state(), VersionState::Retired);
    assert_eq!(stored.version(2).unwrap().state(), VersionState::LiveDraft);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::VersionRetired);
    assert_eq!(entry.target().entity_type(), "recipe_version");
    assert_eq!(entry.target().entity_id(), vid(101).as_uuid());
    assert_eq!(field(entry.before().unwrap(), "retired_at"), Value::Null);
    assert_ne!(field(entry.after().unwrap(), "retired_at"), Value::Null);
    assert_eq!(entry.actor(), Some(user.user()));
}

#[tokio::test]
async fn retire_with_an_old_row_version_gives_web_001_with_both_versions() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let error = RetireVersion::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), retire("R-1", 1, 3))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    assert!(matches!(
        &error,
        AppError::ConcurrentEdit {
            number: 1,
            expected: 3,
            actual: 1,
            ..
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn retire_of_a_draft_gives_rcp_009() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let error = RetireVersion::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), retire("R-1", 2, 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_009);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn retire_in_a_deleted_recipe_gives_rcp_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = RetireVersion::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), retire("R-1", 1, 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn retire_of_an_unknown_number_gives_web_005() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let error = RetireVersion::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), retire("R-1", 9, 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_process_engineer_cannot_retire() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let error = RetireVersion::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), retire("R-1", 1, 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    let state = fixture.store.state();
    let stored = state.recipe(rid(1)).unwrap();
    assert_eq!(stored.version(1).unwrap().state(), VersionState::Approved);
}
