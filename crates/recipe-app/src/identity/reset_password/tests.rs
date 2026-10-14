//! Tests of the use case `ResetPassword` (task 4.3, design D8).

use recipe_domain::audit::{AuditAction, AuditTarget, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{Login, Role};

use super::ResetPassword;
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing, TimeoutStage};
use crate::identity::authenticate;
use crate::identity::fixtures::{IdentityFixture, actor_id, anna_id, assert_one_time_password};
use crate::ports::UserRow;

fn anna() -> Login {
    Login::parse("anna").unwrap()
}

fn fixture() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("old-secret-phrase", false);
    fixture
}

fn anna_row(fixture: &IdentityFixture) -> UserRow {
    fixture.store.state().users.get(&anna_id()).unwrap().clone()
}

#[test]
fn the_operation_is_reset_password() {
    assert_eq!(
        <ResetPassword as UseCase>::OPERATION,
        Operation::ResetPassword
    );
}

#[tokio::test]
async fn a_success_replaces_the_hash_sets_the_flag_and_raises_the_epoch() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let issued = ResetPassword::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap();
    assert_eq!(issued.login(), &anna());
    assert_one_time_password(issued.password());
    let row = anna_row(&fixture);
    assert!(row.must_change_password);
    assert_eq!(row.session_epoch, 1);
    assert_eq!(
        row.password_hash,
        crate::identity::fixtures::fake_hash(issued.password().as_str())
    );
    assert_eq!(fixture.hasher.hash_calls(), 1);
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn every_session_of_the_target_ends_and_the_old_token_finds_no_user() {
    let fixture = fixture();
    let first = fixture.seed_anna_session(1);
    let second = fixture.seed_anna_session(2);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    ResetPassword::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap();
    for token in [&first, &second] {
        let found = authenticate(&fixture.context, token).await.unwrap();
        assert!(found.is_none());
    }
    let sessions = fixture.store.state().sessions;
    assert_eq!(sessions.len(), 1, "only the session of the actor stays");
    assert!(sessions.values().all(|row| row.user_id == actor_id()));
}

#[tokio::test]
async fn the_entry_password_reset_holds_two_images_without_secrets() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let issued = ResetPassword::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::PasswordReset);
    assert_eq!(entry.target(), &AuditTarget::app_user(anna_id()));
    assert_eq!(entry.actor(), Some(actor_id()));
    assert_eq!(entry.before().unwrap()["session_epoch"], 0);
    assert_eq!(entry.after().unwrap()["session_epoch"], 1);
    assert_eq!(entry.after().unwrap()["must_change_password"], true);
    let shown = format!("{entries:?}");
    assert!(!shown.contains(issued.password().as_str()), "{shown}");
    assert!(!shown.contains("old-secret-phrase"), "{shown}");
    assert!(!shown.contains("password_hash"), "{shown}");
    let outcome = format!("{issued:?}");
    assert!(!outcome.contains(issued.password().as_str()), "{outcome}");
}

#[tokio::test]
async fn an_unknown_login_gives_e_web_005_and_no_hash() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let nobody = Login::parse("nobody").unwrap();
    let error = ResetPassword::new(fixture.context.clone())
        .execute(&auth, &meta, &nobody)
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::NotFound { entity: Missing::User(login) } if login == &nobody
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_eq!(fixture.hasher.hash_calls(), 0);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_permission_check_comes_first_for_an_operator_and_a_process_engineer() {
    for role in [Role::Operator, Role::ProcessEngineer] {
        let fixture = fixture();
        let (auth, meta) = fixture.actor(role).await;
        let journal_before = fixture.store.journal().len();
        let error = ResetPassword::new(fixture.context.clone())
            .execute(&auth, &meta, &anna())
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                AppError::Forbidden {
                    operation: Operation::ResetPassword
                }
            ),
            "{error:?}"
        );
        let events = fixture.sink.events();
        assert_eq!(events.len(), 1, "{role:?}");
        assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
        assert_eq!(fixture.store.journal().len(), journal_before, "{role:?}");
        assert_eq!(anna_row(&fixture).session_epoch, 0);
    }
}

#[tokio::test(start_paused = true)]
async fn no_free_permit_gives_e_web_009_and_changes_nothing() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(fixture.context.gate().acquire().await.unwrap());
    }
    let error = ResetPassword::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Timeout {
                stage: TimeoutStage::Argon2Permit
            }
        ),
        "{error:?}"
    );
    assert_eq!(anna_row(&fixture).session_epoch, 0);
    drop(held);
}

#[tokio::test]
async fn a_failed_commit_keeps_the_old_hash_and_the_sessions() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.store.fail_finish(crate::memory::Fault::Unavailable);
    let error = ResetPassword::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    assert_eq!(anna_row(&fixture).session_epoch, 0);
    fixture.store.clear_failures();
    assert!(
        authenticate(&fixture.context, &token)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(fixture.context.gate().available_permits(), 4);
}
