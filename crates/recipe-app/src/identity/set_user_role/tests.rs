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
use crate::identity::fixtures::{
    IdentityFixture, actor_id, anna_id, fake_hash, journal_since, read_committed_runs,
    serializable_runs,
};
use crate::memory::{Fault, FaultPoint, JournalEntry, Lock, LockMode};
use crate::ports::{Isolation, NewUser, UserRow};

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
async fn a_success_sets_the_role_and_raises_the_epoch_in_one_read_committed_transaction() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let runs_before = read_committed_runs(&fixture.store);
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap();
    let row = anna_row(&fixture);
    assert_eq!(row.role, Role::ProcessEngineer);
    assert_eq!(row.session_epoch, 1);
    assert_eq!(read_committed_runs(&fixture.store), runs_before + 1);
    assert_eq!(serializable_runs(&fixture.store), 0);
}

#[tokio::test]
async fn the_guard_lock_is_the_first_port_call_after_the_permission_check() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let start = fixture.store.journal().len();
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap();
    let journal = journal_since(&fixture.store, start);
    let guard = Lock {
        table: "advisory_class_4",
        mode: LockMode::Advisory,
        key: "0".to_owned(),
    };
    let user = Lock {
        table: "app_user",
        mode: LockMode::ForNoKeyUpdate,
        key: anna_id().to_string(),
    };
    assert_eq!(
        journal,
        vec![
            JournalEntry::Begin(Isolation::ReadCommitted),
            JournalEntry::Lock(guard),
            JournalEntry::Lock(user),
        ]
    );
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
async fn a_serialization_failure_returns_its_error_and_runs_no_second_time() {
    for point in [
        FaultPoint::Begin,
        FaultPoint::AppendAudit,
        FaultPoint::Finish,
    ] {
        let fixture = fixture();
        let (auth, meta) = fixture.actor(Role::Administrator).await;
        fixture.store.fail_next(point, Fault::Serialization, 1);
        let runs_before = read_committed_runs(&fixture.store);
        let error = SetUserRole::new(fixture.context.clone())
            .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
            .await
            .unwrap_err();
        // The audit insert reports its own failure as `E-AUD-001`. The other points give `E-DB-001`.
        let expected = if point == FaultPoint::AppendAudit {
            ErrorCode::AUD_001
        } else {
            ErrorCode::DB_001
        };
        assert_eq!(error.code(), expected, "{point:?}");
        let opened = read_committed_runs(&fixture.store) - runs_before;
        // A failed `Begin` opens no transaction. Any other point shows one run and no second.
        assert_eq!(opened, usize::from(point != FaultPoint::Begin), "{point:?}");
        assert_eq!(serializable_runs(&fixture.store), 0, "{point:?}");
        assert_eq!(anna_row(&fixture).role, Role::Operator, "{point:?}");
        assert_eq!(anna_row(&fixture).session_epoch, 0, "{point:?}");
        assert!(fixture.store.audit_entries().is_empty(), "{point:?}");
    }
}

#[tokio::test]
async fn a_deadlock_gives_e_db_001_at_once_with_no_second_run() {
    let fixture = fixture();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.store.fail_finish(Fault::Deadlock);
    let runs_before = read_committed_runs(&fixture.store);
    let error = SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(read_committed_runs(&fixture.store), runs_before + 1);
}

#[tokio::test]
async fn a_lock_timeout_at_the_guard_lock_gives_e_db_001_and_writes_nothing() {
    let fixture = fixture();
    let session = fixture.seed_anna_session(1);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture
        .store
        .fail_next(FaultPoint::LockAdministratorGuard, Fault::LockTimeout, 1);
    let start = fixture.store.journal().len();
    let error = SetUserRole::new(fixture.context.clone())
        .execute(&auth, &meta, input(anna(), Role::ProcessEngineer))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(
        journal_since(&fixture.store, start),
        vec![JournalEntry::Begin(Isolation::ReadCommitted)],
        "one Begin and no lock"
    );
    assert_eq!(anna_row(&fixture).role, Role::Operator);
    assert_eq!(anna_row(&fixture).session_epoch, 0);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(
        authenticate(&fixture.context, &session)
            .await
            .unwrap()
            .is_some()
    );
}
