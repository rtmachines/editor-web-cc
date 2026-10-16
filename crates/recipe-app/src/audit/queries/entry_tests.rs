//! Tests of `GetAuditEntry` (spec "Scope of the audit queries", "Audit entry with its states").

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use serde_json::json;

use super::test_world::{entry_id, login, world};
use super::{AuditEntryView, GetAuditEntry};
use crate::error::{AppError, Missing};
use crate::identity::fixtures::IdentityFixture;
use crate::identity::{CreateUser, CreateUserInput};
use crate::memory::fixtures::{Fixture, actor, meta};

/// Run `GetAuditEntry` as `role` on the entry `id`.
async fn get(fixture: &Fixture, role: Role, id: u64) -> Result<AuditEntryView, AppError> {
    GetAuditEntry::new(fixture.context())
        .execute(&actor(role), &meta(), entry_id(id))
        .await
}

/// Assert that `result` is `NotFound` (`E-WEB-005`) of the audit entry `id`.
fn assert_not_found(result: Result<AuditEntryView, AppError>, id: u64) {
    let error = result.unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(
        matches!(&error, AppError::NotFound { entity: Missing::AuditEntry(found) } if *found == entry_id(id)),
        "{error:?}"
    );
}

#[tokio::test]
async fn the_entry_that_create_user_writes_holds_the_login_and_no_password_hash() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let input = CreateUserInput {
        login: login("berta"),
        role: Role::Operator,
    };
    let issued = CreateUser::new(fixture.context.clone())
        .execute(&auth, &meta, input)
        .await
        .unwrap();
    let view = GetAuditEntry::new(fixture.context.app.clone())
        .execute(auth.user(), &meta, entry_id(1))
        .await
        .unwrap();
    let after = view.after.unwrap();
    assert_eq!(after.get("login"), Some(&json!("berta")));
    assert!(after.get("password_hash").is_none());
    let shown = format!("{after}");
    assert!(!shown.contains("argon2id"), "{shown}");
    assert!(!shown.contains(issued.password().as_str()), "{shown}");
    assert_eq!(view.before, None);
}

#[tokio::test]
async fn an_administrator_sees_the_trace_fields() {
    let fixture = world().await;
    let view = get(&fixture, Role::Administrator, 3).await.unwrap();
    let trace = view.trace.unwrap();
    assert_eq!(trace.chain_seq, 3);
    assert!(trace.client_ip.is_some());
    assert!(trace.session_ref.is_some());
    assert!(view.before.is_some() && view.after.is_some());
}

#[tokio::test]
async fn a_process_engineer_sees_a_recipe_entry_without_the_trace_fields() {
    let fixture = world().await;
    let view = get(&fixture, Role::ProcessEngineer, 3).await.unwrap();
    assert_eq!(view.row.id, entry_id(3));
    assert_eq!(view.trace, None);
    assert!(view.after.is_some());
}

#[tokio::test]
async fn a_process_engineer_gets_no_entry_of_a_machine() {
    let fixture = world().await;
    assert_not_found(get(&fixture, Role::ProcessEngineer, 1).await, 1);
}

#[tokio::test]
async fn an_entry_outside_the_scope_and_an_absent_entry_get_the_same_answer() {
    let fixture = world().await;
    assert_not_found(get(&fixture, Role::ProcessEngineer, 7).await, 7);
    assert_not_found(get(&fixture, Role::ProcessEngineer, 99).await, 99);
    assert_not_found(get(&fixture, Role::Administrator, 99).await, 99);
}

#[test]
fn the_missing_audit_entry_names_the_full_id_and_a_malformed_id_names_no_value() {
    assert_eq!(
        Missing::AuditEntry(entry_id(1234)).to_string(),
        "Audit entry 1234"
    );
    assert_eq!(Missing::MalformedAuditEntry.to_string(), "Audit entry");
    let error = AppError::NotFound {
        entity: Missing::MalformedAuditEntry,
    };
    assert_eq!(error.code(), ErrorCode::WEB_005);
}

#[tokio::test]
async fn a_process_engineer_reads_the_entry_of_a_deleted_recipe() {
    let fixture = world().await;
    assert!(get(&fixture, Role::ProcessEngineer, 6).await.is_ok());
}
