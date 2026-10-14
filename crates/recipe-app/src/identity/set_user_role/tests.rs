//! Tests of the use case `SetUserRole` (task 4.4, design D8 and D16, ADR-0035 c).

use recipe_domain::audit::{AuditAction, AuditTarget, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{IdentityError, Login, Role, UserId};
use uuid::Uuid;

use super::{SetUserRole, SetUserRoleInput};
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

fn input(login: Login, role: Role) -> SetUserRoleInput {
    SetUserRoleInput { login, role }
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
fn the_operation_is_set_user_role() {
    assert_eq!(<SetUserRole as UseCase>::OPERATION, Operation::SetUserRole);
}

#[tokio::test]
async fn a_success_sets_the_role_and_raises_the_epoch_in_a_serializable_transaction() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap();
    let row = anna_row(&fixture);
    assert_eq!(row.role, Role::ProcessEngineer);
    assert_eq!(row.session_epoch, 1);
    assert_eq!(serializable_runs(&fixture.store), 1);
}

#[tokio::test]
async fn a_success_ends_every_session_of_the_target() {
    let fixture = fixture();
    let first = fixture.seed_anna_session(1);
    let second = fixture.seed_anna_session(2);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
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
async fn the_entry_user_role_set_holds_both_images() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::UserRoleSet);
    assert_eq!(entry.target(), &AuditTarget::app_user(anna_id()));
    assert_eq!(entry.actor(), Some(actor_id()));
    assert_eq!(entry.before().unwrap()["role"], "operator");
    assert_eq!(entry.after().unwrap()["role"], "process_engineer");
    assert!(!format!("{entries:?}").contains("password_hash"));
}

#[tokio::test]
async fn an_unknown_login_gives_e_web_005() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let nobody = Login::parse("nobody").unwrap();
    let error = SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(nobody.clone(), Role::Operator))
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
async fn the_last_active_administrator_cannot_lose_the_role() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let error = SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(actor_login(), Role::Operator))
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
    assert_eq!(actor_row(&fixture).role, Role::Administrator);
    assert_eq!(actor_row(&fixture).session_epoch, 0);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn two_active_administrators_allow_a_demotion() {
    let fixture = fixture();
    seed_second_administrator(&fixture);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(actor_login(), Role::Operator))
        .await
        .unwrap();
    assert_eq!(actor_row(&fixture).role, Role::Operator);
}

#[tokio::test]
async fn the_permission_check_comes_first_for_an_operator_and_a_process_engineer() {
    for role in [Role::Operator, Role::ProcessEngineer] {
        let fixture = fixture();
        let (auth, meta) = fixture.actor(role).await;
        let runs_before = fixture.store.journal().len();
        let error = SetUserRole::new(fixture.context.clone())
            .execute(&auth, &meta, input(anna(), Role::Administrator))
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                AppError::Forbidden {
                    operation: Operation::SetUserRole
                }
            ),
            "{error:?}"
        );
        let events = fixture.sink.events();
        assert_eq!(events.len(), 1, "{role:?}");
        assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
        assert_eq!(fixture.store.journal().len(), runs_before, "{role:?}");
        assert_eq!(anna_row(&fixture).role, Role::Operator);
    }
}

#[tokio::test]
async fn one_serialization_failure_at_the_start_gives_a_second_run_that_succeeds() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture
        .store
        .fail_next(FaultPoint::Begin, Fault::Serialization, 1);
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap();
    assert_eq!(anna_row(&fixture).role, Role::ProcessEngineer);
    assert_eq!(anna_row(&fixture).session_epoch, 1, "one change only");
}

#[tokio::test]
async fn one_serialization_failure_at_finish_gives_a_second_run_that_succeeds() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture
        .store
        .fail_next(FaultPoint::Finish, Fault::Serialization, 1);
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap();
    assert_eq!(serializable_runs(&fixture.store), 2);
    assert_eq!(anna_row(&fixture).session_epoch, 1, "one change only");
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn one_serialization_failure_in_the_audit_write_gives_a_second_run_that_succeeds() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture
        .store
        .fail_next(FaultPoint::AppendAudit, Fault::Serialization, 1);
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap();
    assert_eq!(serializable_runs(&fixture.store), 2);
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn two_serialization_failures_give_e_db_001_and_change_nothing() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture
        .store
        .fail_next(FaultPoint::Finish, Fault::Serialization, 2);
    let error = SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(serializable_runs(&fixture.store), 2);
    assert_eq!(anna_row(&fixture).role, Role::Operator);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_deadlock_gives_e_db_001_at_once_with_no_second_run() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.store.fail_finish(Fault::Deadlock);
    let error = SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(serializable_runs(&fixture.store), 1);
}
