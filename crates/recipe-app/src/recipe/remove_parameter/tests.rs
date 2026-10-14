use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use serde_json::json;

use super::{RemoveParameter, RemoveParameterInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, lock_line, meta, parameter_on, pid, pkey, reference, rid, uid, vid,
};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::DraftOutput;
use crate::recipe::test_support::{
    DRAFT_VERSION, seed_deleted, seed_speed_draft, speed_key, stored_parameters,
};

fn input(expected: i64) -> RemoveParameterInput {
    RemoveParameterInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: expected,
        key: speed_key(),
    }
}

fn assert_nothing_written(fixture: &Fixture) {
    assert!(fixture.store.audit_entries().is_empty());
    assert!(
        fixture
            .store
            .state()
            .editors_of(vid(DRAFT_VERSION))
            .is_empty()
    );
    assert_eq!(stored_parameters(fixture).len(), 1);
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(RemoveParameter::OPERATION, Operation::RemoveParameter);
}

#[tokio::test]
async fn success_deletes_the_parameter_with_its_limits_and_translations() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let other = parameter_on(PlcPlatform::TwinCat3, 2, "TEMP", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &other);
    let output = RemoveParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(1))
        .await
        .unwrap();
    assert_eq!(
        output,
        DraftOutput {
            version: vid(DRAFT_VERSION),
            row_version: 2
        }
    );
    let remaining = stored_parameters(&fixture);
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining.first().unwrap().id, pid(2));
    assert_eq!(remaining.first().unwrap().translations.len(), 1);
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
            lock_line(
                "recipe_version",
                LockMode::ForUpdate,
                vid(DRAFT_VERSION).to_string()
            ),
        ]
    );
    assert_eq!(
        fixture.store.state().editors_of(vid(DRAFT_VERSION)),
        vec![actor(Role::ProcessEngineer).user()]
    );
}

#[tokio::test]
async fn the_before_state_holds_the_limits_and_the_translation_for_pl() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let engineer = actor(Role::ProcessEngineer);
    RemoveParameter::new(fixture.context())
        .execute(&engineer, &meta(), input(1))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::ParameterRemoved);
    assert_eq!(entry.target().entity_type(), "parameter");
    assert_eq!(entry.target().entity_id(), pid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    assert_eq!(entry.after(), None);
    assert_eq!(entry.actor(), Some(engineer.user()));
    let before = entry.before().unwrap();
    assert_eq!(before.get("id"), Some(&json!(pid(1).to_string())));
    assert_eq!(before.get("tag"), Some(&json!("SPEED")));
    let translations = before
        .get("translations")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(translations.len(), 1);
    assert_eq!(
        translations.first().and_then(|row| row.get("lang")),
        Some(&json!("pl"))
    );
    let limits = before.get("limits").and_then(|v| v.as_array()).unwrap();
    assert_eq!(limits.len(), 1);
    let limit = limits.first().unwrap();
    assert_eq!(limit.get("leaf_path"), Some(&json!("")));
    assert_eq!(limit.get("min_value"), Some(&json!(10)));
    assert_eq!(limit.get("max_value"), Some(&json!(250)));
}

#[tokio::test]
async fn an_unknown_key_gives_rcp_051() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let mut request = input(1);
    request.key = pkey(99);
    let error = RemoveParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_retired_version_gives_rcp_002() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_retired(&machine, 1, "R-1");
    let error = RemoveParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_deleted_recipe_gives_rcp_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = RemoveParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
}

#[tokio::test]
async fn an_old_row_version_gives_web_001_and_keeps_the_parameter() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = RemoveParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(2))
        .await
        .unwrap_err();
    assert!(matches!(
        &error,
        AppError::ConcurrentEdit {
            expected: 2,
            actual: 1,
            ..
        }
    ));
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn an_operator_is_forbidden_and_opens_no_transaction() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = RemoveParameter::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input(1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert_nothing_written(&fixture);
}
