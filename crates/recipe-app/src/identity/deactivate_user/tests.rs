//! Tests of the use case `DeactivateUser` (task 4.4, design D8 and D16, ADR-0035 a and c).

use recipe_domain::audit::{AuditAction, AuditTarget, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{IdentityError, Login, Role, UserId};
use uuid::Uuid;

use super::DeactivateUser;
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::identity::authenticate;
use crate::identity::fixtures::{IdentityFixture, actor_id, anna_id, fake_hash, serializable_runs};
use crate::memory::{Fault, FaultPoint};
use crate::ports::{NewUser, UserRow};

fn anna() -> Login {
    Login::parse("anna").unwrap()
}

fn actor_login() -> Login {
    Login::parse("actor").unwrap()
}

fn fixture() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    fixture
}

fn seed_second_administrator(fixture: &IdentityFixture) {
    fixture.store.seed_user(&NewUser {
        id: UserId::from(Uuid::from_u128(12)),
        login: Login::parse("boss").unwrap(),
        password_hash: fake_hash("secret-phrase-two"),
        role: Role::Administrator,
        must_change_password: false,
    });
}

fn anna_row(fixture: &IdentityFixture) -> UserRow {
    fixture.store.state().users.get(&anna_id()).unwrap().clone()
}

fn actor_row(fixture: &IdentityFixture) -> UserRow {
    fixture
        .store
        .state()
        .users
        .get(&actor_id())
        .unwrap()
        .clone()
}

#[test]
fn the_operation_is_deactivate_user() {
    assert_eq!(
        <DeactivateUser as UseCase>::OPERATION,
        Operation::DeactivateUser
    );
}

#[tokio::test]
async fn a_success_deactivates_the_user_and_raises_the_epoch_in_a_serializable_transaction() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap();
    let row = anna_row(&fixture);
    assert!(!row.is_active);
    assert!(row.deactivated_at.is_some());
    assert_eq!(row.session_epoch, 1);
    assert_eq!(serializable_runs(&fixture.store), 1);
}

#[tokio::test]
async fn a_success_ends_every_session_and_the_next_request_finds_no_user() {
    let fixture = fixture();
    let first = fixture.seed_anna_session(1);
    let second = fixture.seed_anna_session(2);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap();
    for token in [&first, &second] {
        assert!(
            authenticate(&fixture.context, token)
                .await
                .unwrap()
                .is_none()
        );
    }
    let sessions = fixture.store.state().sessions;
    assert!(sessions.values().all(|row| row.user_id == actor_id()));
}

#[tokio::test]
async fn the_entry_user_deactivated_holds_both_images() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::UserDeactivated);
    assert_eq!(entry.target(), &AuditTarget::app_user(anna_id()));
    assert_eq!(entry.actor(), Some(actor_id()));
    assert_eq!(entry.before().unwrap()["is_active"], true);
    assert_eq!(entry.after().unwrap()["is_active"], false);
}

#[tokio::test]
async fn an_inactive_user_gives_e_idn_010_and_writes_nothing() {
    let fixture = fixture();
    fixture.store.update_state(|state, now| {
        let row = state.users.get_mut(&anna_id()).unwrap();
        row.is_active = false;
        row.deactivated_at = Some(now);
        row.session_epoch = 4;
    });
    let before = anna_row(&fixture);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let error = DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::Identity(IdentityError::UserAlreadyInactive { login }) if login == &anna()
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_010);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(anna_row(&fixture), before);
}

#[tokio::test]
async fn an_unknown_login_gives_e_web_005() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let nobody = Login::parse("nobody").unwrap();
    let error = DeactivateUser::new(fixture.context.clone())
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
}

#[tokio::test]
async fn the_last_active_administrator_cannot_be_deactivated() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let error = DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &actor_login())
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::Identity(IdentityError::LastAdministrator { login })
                if login == &actor_login()
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_002);
    assert!(actor_row(&fixture).is_active);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn two_active_administrators_allow_a_deactivation() {
    let fixture = fixture();
    seed_second_administrator(&fixture);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &actor_login())
        .await
        .unwrap();
    assert!(!actor_row(&fixture).is_active);
}

#[tokio::test]
async fn the_permission_check_comes_first_for_an_operator_and_a_process_engineer() {
    for role in [Role::Operator, Role::ProcessEngineer] {
        let fixture = fixture();
        let (auth, meta) = fixture.actor(role).await;
        let journal_before = fixture.store.journal().len();
        let error = DeactivateUser::new(fixture.context.clone())
            .execute(&auth, &meta, &anna())
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                AppError::Forbidden {
                    operation: Operation::DeactivateUser
                }
            ),
            "{error:?}"
        );
        let events = fixture.sink.events();
        assert_eq!(events.len(), 1, "{role:?}");
        assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
        assert_eq!(fixture.store.journal().len(), journal_before, "{role:?}");
        assert!(anna_row(&fixture).is_active);
    }
}

#[tokio::test]
async fn one_serialization_failure_gives_a_second_run_that_succeeds() {
    for point in [
        FaultPoint::Begin,
        FaultPoint::AppendAudit,
        FaultPoint::Finish,
    ] {
        let fixture = fixture();
        let (auth, meta) = fixture.actor(Role::Administrator).await;
        fixture.store.fail_next(point, Fault::Serialization, 1);
        DeactivateUser::new(fixture.context.clone())
            .execute(&auth, &meta, &anna())
            .await
            .unwrap();
        assert!(!anna_row(&fixture).is_active, "{point:?}");
        assert_eq!(anna_row(&fixture).session_epoch, 1, "{point:?}");
        assert_eq!(fixture.store.audit_entries().len(), 1, "{point:?}");
    }
}

#[tokio::test]
async fn two_serialization_failures_give_e_db_001_and_change_nothing() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture
        .store
        .fail_next(FaultPoint::Finish, Fault::Serialization, 2);
    let error = DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(serializable_runs(&fixture.store), 2);
    assert!(anna_row(&fixture).is_active);
}

#[tokio::test]
async fn a_deadlock_gives_e_db_001_at_once_with_no_second_run() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.store.fail_finish(Fault::Deadlock);
    let error = DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &meta, &anna())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(serializable_runs(&fixture.store), 1);
}
