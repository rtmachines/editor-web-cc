//! Tests of the use case `ActivateUser` (task 4.3, design D8, ADR-0035 a).

use recipe_domain::audit::{AuditAction, AuditTarget, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{IdentityError, Login, Role};

use super::ActivateUser;
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::identity::fixtures::{IdentityFixture, actor_id, anna_id};
use crate::ports::UserRow;

fn anna() -> Login {
    Login::parse("anna").unwrap()
}

fn fixture() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    fixture
}

fn deactivate(fixture: &IdentityFixture) {
    fixture.store.update_state(|state, now| {
        let row = state.users.get_mut(&anna_id()).unwrap();
        row.is_active = false;
        row.deactivated_at = Some(now);
        row.session_epoch = 3;
    });
}

fn anna_row(fixture: &IdentityFixture) -> UserRow {
    fixture.store.state().users.get(&anna_id()).unwrap().clone()
}

#[test]
fn the_operation_is_activate_user() {
    assert_eq!(
        <ActivateUser as UseCase>::OPERATION,
        Operation::ActivateUser
    );
}

#[tokio::test]
async fn a_success_activates_the_user_and_keeps_the_epoch() {
    let fixture = fixture();
    deactivate(&fixture);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    ActivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap();
    let row = anna_row(&fixture);
    assert!(row.is_active);
    assert_eq!(row.deactivated_at, None);
    assert_eq!(row.session_epoch, 3);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::UserActivated);
    assert_eq!(entry.target(), &AuditTarget::app_user(anna_id()));
    assert_eq!(entry.actor(), Some(actor_id()));
    assert_eq!(entry.before().unwrap()["is_active"], false);
    assert_eq!(entry.after().unwrap()["is_active"], true);
}

#[tokio::test]
async fn an_active_user_gives_e_idn_009_and_writes_nothing() {
    let fixture = fixture();
    let before = anna_row(&fixture);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let error = ActivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::Identity(IdentityError::UserAlreadyActive { login }) if login == &anna()
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_009);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(anna_row(&fixture), before);
}

#[tokio::test]
async fn an_unknown_login_gives_e_web_005() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let nobody = Login::parse("nobody").unwrap();
    let error = ActivateUser::new(fixture.context.clone())
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
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_permission_check_comes_first_for_an_operator_and_a_process_engineer() {
    for role in [Role::Operator, Role::ProcessEngineer] {
        let fixture = fixture();
        deactivate(&fixture);
        let (auth, meta) = fixture.actor(role).await;
        let journal_before = fixture.store.journal().len();
        let error = ActivateUser::new(fixture.context.clone())
            .execute(&auth, &meta, &anna())
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                AppError::Forbidden {
                    operation: Operation::ActivateUser
                }
            ),
            "{error:?}"
        );
        let events = fixture.sink.events();
        assert_eq!(events.len(), 1, "{role:?}");
        assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
        assert_eq!(fixture.store.journal().len(), journal_before, "{role:?}");
        assert!(!anna_row(&fixture).is_active);
    }
}
