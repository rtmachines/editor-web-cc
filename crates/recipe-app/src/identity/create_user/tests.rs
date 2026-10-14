//! Tests of the use case `CreateUser` (task 4.3, design D8).

use recipe_domain::audit::{AuditAction, AuditTarget, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{IdentityError, Login, Role, UserId};
use uuid::Uuid;

use super::{CreateUser, CreateUserInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, StorageError, TimeoutStage};
use crate::identity::fixtures::{IdentityFixture, actor_id, assert_one_time_password, fake_hash};
use crate::memory::Fault;
use crate::ports::NewUser;

fn input(login: &str, role: Role) -> CreateUserInput {
    CreateUserInput {
        login: Login::parse(login).unwrap(),
        role,
    }
}

fn stored_user(fixture: &IdentityFixture, login: &str) -> Option<crate::ports::UserRow> {
    let login = Login::parse(login).unwrap();
    fixture
        .store
        .state()
        .users
        .values()
        .find(|row| row.login == login)
        .cloned()
}

fn seed_anna(fixture: &IdentityFixture) {
    fixture.store.seed_user(&NewUser {
        id: UserId::from(Uuid::from_u128(11)),
        login: Login::parse("anna").unwrap(),
        password_hash: fake_hash("secret-phrase-one"),
        role: Role::Operator,
        must_change_password: false,
    });
}

#[test]
fn the_operation_is_create_user() {
    assert_eq!(<CreateUser as UseCase>::OPERATION, Operation::CreateUser);
}

#[tokio::test]
async fn a_success_creates_an_active_user_with_the_hash_of_the_one_time_password() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let issued = CreateUser::new(fixture.context.clone())
        .execute(&auth, &meta, input("berta", Role::ProcessEngineer))
        .await
        .unwrap();
    assert_eq!(issued.login(), &Login::parse("berta").unwrap());
    assert_one_time_password(issued.password());
    let row = stored_user(&fixture, "berta").unwrap();
    assert_eq!(row.role, Role::ProcessEngineer);
    assert!(row.is_active);
    assert!(row.must_change_password);
    assert_eq!(row.session_epoch, 0);
    assert_eq!(
        row.password_hash,
        fake_hash(issued.password().as_str()),
        "the stored hash is the hash of the returned password"
    );
    assert_eq!(fixture.hasher.hash_calls(), 1);
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn the_id_of_a_created_user_is_a_version_4_uuid() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    CreateUser::new(fixture.context.clone())
        .execute(&auth, &meta, input("berta", Role::Operator))
        .await
        .unwrap();
    let row = stored_user(&fixture, "berta").unwrap();
    assert_eq!(row.id.as_uuid().get_version_num(), 4);
}

#[tokio::test]
async fn two_calls_give_two_different_one_time_passwords() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let create = CreateUser::new(fixture.context.clone());
    let first = create
        .execute(&auth, &meta, input("berta", Role::Operator))
        .await
        .unwrap();
    let second = create
        .execute(&auth, &meta, input("carl", Role::Operator))
        .await
        .unwrap();
    assert_ne!(first.password().as_str(), second.password().as_str());
}

#[tokio::test]
async fn the_entry_user_created_holds_the_row_without_hash_and_password() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let issued = CreateUser::new(fixture.context.clone())
        .execute(&auth, &meta, input("berta", Role::Operator))
        .await
        .unwrap();
    let row = stored_user(&fixture, "berta").unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::UserCreated);
    assert_eq!(entry.target(), &AuditTarget::app_user(row.id));
    assert_eq!(entry.actor(), Some(actor_id()));
    assert_eq!(entry.before(), None);
    let after = entry.after().unwrap();
    assert_eq!(after["login"], "berta");
    assert_eq!(after["role"], "operator");
    assert_eq!(after["must_change_password"], true);
    let shown = format!("{entries:?}");
    assert!(!shown.contains(issued.password().as_str()), "{shown}");
    assert!(!shown.contains("argon2id"), "{shown}");
    assert!(!shown.contains("password_hash"), "{shown}");
}

#[tokio::test]
async fn the_result_hides_the_password_in_debug() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let issued = CreateUser::new(fixture.context.clone())
        .execute(&auth, &meta, input("berta", Role::Operator))
        .await
        .unwrap();
    let shown = format!("{issued:?}");
    assert!(!shown.contains(issued.password().as_str()), "{shown}");
    assert!(shown.contains("[redacted]"), "{shown}");
    assert!(shown.contains("berta"), "{shown}");
}

#[tokio::test]
async fn the_permission_check_comes_first_for_an_operator_and_a_process_engineer() {
    for role in [Role::Operator, Role::ProcessEngineer] {
        let fixture = IdentityFixture::new();
        let (auth, meta) = fixture.actor(role).await;
        let journal_before = fixture.store.journal().len();
        let error = CreateUser::new(fixture.context.clone())
            .execute(&auth, &meta, input("berta", Role::Operator))
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                AppError::Forbidden {
                    operation: Operation::CreateUser
                }
            ),
            "{error:?}"
        );
        assert_eq!(error.code(), ErrorCode::WEB_002);
        let events = fixture.sink.events();
        assert_eq!(events.len(), 1, "{role:?}");
        assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
        assert_eq!(fixture.store.journal().len(), journal_before, "{role:?}");
        assert_eq!(fixture.hasher.hash_calls(), 0, "{role:?}");
        assert!(stored_user(&fixture, "berta").is_none());
    }
}

#[tokio::test]
async fn a_known_login_gives_e_idn_001_before_any_hash() {
    let fixture = IdentityFixture::new();
    seed_anna(&fixture);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let error = CreateUser::new(fixture.context.clone())
        .execute(&auth, &meta, input("anna", Role::Operator))
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::Identity(IdentityError::DuplicateLogin { login })
                if login == &Login::parse("anna").unwrap()
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_001);
    assert_eq!(
        fixture.hasher.hash_calls(),
        0,
        "the pre-check saves the hash"
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_login_that_appears_after_the_pre_check_gives_e_idn_001_from_the_insert() {
    let fixture = IdentityFixture::with_race(|store| {
        store.seed_user(&NewUser {
            id: UserId::from(Uuid::from_u128(12)),
            login: Login::parse("berta").unwrap(),
            password_hash: fake_hash("secret-phrase-one"),
            role: Role::Operator,
            must_change_password: false,
        });
    });
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let error = CreateUser::new(fixture.context.clone())
        .execute(&auth, &meta, input("berta", Role::Operator))
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::Identity(IdentityError::DuplicateLogin { login })
                if login == &Login::parse("berta").unwrap()
        ),
        "{error:?}"
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test(start_paused = true)]
async fn no_free_permit_gives_e_web_009_and_creates_nothing() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(fixture.context.gate().acquire().await.unwrap());
    }
    let error = CreateUser::new(fixture.context.clone())
        .execute(&auth, &meta, input("berta", Role::Operator))
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
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(stored_user(&fixture, "berta").is_none());
    drop(held);
}

#[tokio::test]
async fn a_failed_commit_creates_no_user_and_frees_the_permit() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.store.fail_finish(Fault::Unavailable);
    let error = CreateUser::new(fixture.context.clone())
        .execute(&auth, &meta, input("berta", Role::Operator))
        .await
        .unwrap_err();
    assert!(
        matches!(error, AppError::Storage(StorageError::Unavailable)),
        "{error:?}"
    );
    assert!(stored_user(&fixture, "berta").is_none());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}
