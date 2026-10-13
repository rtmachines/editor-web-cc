use recipe_domain::audit::SecurityEventKind;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;

use super::authorize;
use crate::auth::Operation;
use crate::error::AppError;
use crate::memory::fixtures::{actor, meta};
use crate::memory::{Fault, MemorySink};

#[tokio::test]
async fn an_allowed_role_writes_no_event() {
    let sink = MemorySink::default();
    let result = authorize(
        Operation::RenameMachine,
        &actor(Role::Administrator),
        &meta(),
        &sink,
    )
    .await;
    assert!(result.is_ok());
    assert!(sink.events().is_empty());
}

#[tokio::test]
async fn operator_on_rename_machine_is_forbidden_and_leaves_one_event() {
    let sink = MemorySink::default();
    let user = actor(Role::Operator);
    let request = meta();
    let error = authorize(Operation::RenameMachine, &user, &request, &sink)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(matches!(
        error,
        AppError::Forbidden {
            operation: Operation::RenameMachine
        }
    ));
    let events = sink.events();
    assert_eq!(events.len(), 1);
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::Forbidden);
    assert_eq!(event.user(), Some(user.user()));
    assert_eq!(event.request_id(), request.request_id());
    assert_eq!(event.client_ip(), request.client_ip());
    assert_eq!(
        event.detail(),
        Some(&serde_json::json!({"operation": "RenameMachine"}))
    );
}

#[tokio::test]
async fn a_failing_sink_returns_the_storage_error_and_keeps_no_event() {
    let sink = MemorySink::default();
    sink.fail_with(Fault::Unavailable);
    let error = authorize(
        Operation::RenameMachine,
        &actor(Role::Operator),
        &meta(),
        &sink,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert!(matches!(error, AppError::Storage(_)));
    assert!(sink.events().is_empty());
}

#[tokio::test]
async fn login_passes_for_every_role() {
    let sink = MemorySink::default();
    for role in [Role::Administrator, Role::ProcessEngineer, Role::Operator] {
        authorize(Operation::Login, &actor(role), &meta(), &sink)
            .await
            .unwrap();
    }
    assert!(sink.events().is_empty());
}

#[tokio::test]
async fn process_engineer_on_retire_version_is_forbidden() {
    let sink = MemorySink::default();
    let error = authorize(
        Operation::RetireVersion,
        &actor(Role::ProcessEngineer),
        &meta(),
        &sink,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert_eq!(sink.events().len(), 1);
}

#[tokio::test]
async fn process_engineer_on_list_audit_entries_passes() {
    let sink = MemorySink::default();
    authorize(
        Operation::ListAuditEntries,
        &actor(Role::ProcessEngineer),
        &meta(),
        &sink,
    )
    .await
    .unwrap();
    assert!(sink.events().is_empty());
}
