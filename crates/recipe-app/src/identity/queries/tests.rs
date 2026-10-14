//! Tests of the six identity queries (task 4.5, design D9).

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::SecurityEventKind;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{Login, Role, TypedLogin, UserId};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
    GetLoginForm, GetOwnPasswordForm, GetOwnProfile, GetUser, GetUserForm, ListUsers,
    ListUsersInput,
};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::identity::fixtures::{IdentityFixture, fake_hash};
use crate::memory::JournalEntry;
use crate::ports::{Isolation, NewUser};

/// The start of the clock of a fixture, in Unix seconds.
const START_SECONDS: i64 = 1_000_000;

fn at(offset: i64) -> Timestamp {
    let seconds = START_SECONDS.checked_add(offset).unwrap();
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn ip(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, last))
}

fn seed(fixture: &IdentityFixture, number: u128, login: &str, role: Role) {
    fixture.store.seed_user(&NewUser {
        id: UserId::from(Uuid::from_u128(number)),
        login: Login::parse(login).unwrap(),
        password_hash: fake_hash("secret-phrase-one"),
        role,
        must_change_password: false,
    });
}

fn logins(users: &[crate::ports::UserSummary]) -> Vec<&str> {
    users.iter().map(|user| user.login.as_str()).collect()
}

/// Tell whether every transaction after journal line `mark` is read-only, and at least one exists.
fn only_read_only_transactions(fixture: &IdentityFixture, mark: usize) -> bool {
    let journal = fixture.store.journal();
    let begins: Vec<Isolation> = journal
        .iter()
        .skip(mark)
        .filter_map(|entry| match entry {
            JournalEntry::Begin(isolation) => Some(*isolation),
            _ => None,
        })
        .collect();
    !begins.is_empty()
        && begins
            .iter()
            .all(|isolation| *isolation == Isolation::ReadOnly)
}

fn list(q: Option<&str>, page: u32) -> ListUsersInput {
    ListUsersInput {
        q: q.map(str::to_owned),
        page,
    }
}

#[test]
fn each_query_names_its_operation() {
    assert_eq!(<ListUsers as UseCase>::OPERATION, Operation::ListUsers);
    assert_eq!(<GetUser as UseCase>::OPERATION, Operation::GetUser);
    assert_eq!(<GetUserForm as UseCase>::OPERATION, Operation::GetUserForm);
    assert_eq!(
        <GetOwnProfile as UseCase>::OPERATION,
        Operation::GetOwnProfile
    );
    assert_eq!(
        <GetLoginForm as UseCase>::OPERATION,
        Operation::GetLoginForm
    );
    assert_eq!(
        <GetOwnPasswordForm as UseCase>::OPERATION,
        Operation::GetOwnPasswordForm
    );
}

#[tokio::test]
async fn list_users_filters_by_part_of_a_login_in_byte_order() {
    let fixture = IdentityFixture::new();
    seed(&fixture, 21, "mhoffman", Role::Operator);
    seed(&fixture, 22, "jkowalski", Role::Operator);
    seed(&fixture, 23, "akowal", Role::Operator);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let mark = fixture.store.journal().len();
    let users = ListUsers::new(fixture.context.clone())
        .execute(&auth, &meta, list(Some("kowal"), 0))
        .await
        .unwrap();
    assert_eq!(logins(&users), ["akowal", "jkowalski"]);
    assert!(only_read_only_transactions(&fixture, mark));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn list_users_without_a_filter_returns_every_user_in_byte_order() {
    let fixture = IdentityFixture::new();
    seed(&fixture, 21, "mhoffman", Role::Operator);
    seed(&fixture, 22, "a-b", Role::Operator);
    seed(&fixture, 23, "a_b", Role::Operator);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let users = ListUsers::new(fixture.context.clone())
        .execute(&auth, &meta, list(None, 0))
        .await
        .unwrap();
    assert_eq!(logins(&users), ["a-b", "a_b", "actor", "mhoffman"]);
}

#[tokio::test]
async fn an_underscore_in_the_filter_is_a_plain_character() {
    let fixture = IdentityFixture::new();
    seed(&fixture, 21, "mhoffman", Role::Operator);
    seed(&fixture, 22, "j_smith", Role::Operator);
    seed(&fixture, 23, "m_k", Role::Operator);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let users = ListUsers::new(fixture.context.clone())
        .execute(&auth, &meta, list(Some("_"), 0))
        .await
        .unwrap();
    assert_eq!(logins(&users), ["j_smith", "m_k"]);
}

#[tokio::test]
async fn list_users_pages_by_the_setting_page_users() {
    let fixture = IdentityFixture::new();
    for (number, login) in [
        (21, "akowal"),
        (22, "jkowalski"),
        (23, "mhoffman"),
        (24, "zed"),
    ] {
        seed(&fixture, number, login, Role::Operator);
    }
    let mut context = fixture.context.clone();
    context.settings.page_users = 2;
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let query = ListUsers::new(context);
    let first = query.execute(&auth, &meta, list(None, 0)).await.unwrap();
    let second = query.execute(&auth, &meta, list(None, 1)).await.unwrap();
    let third = query.execute(&auth, &meta, list(None, 2)).await.unwrap();
    let beyond = query.execute(&auth, &meta, list(None, 3)).await.unwrap();
    assert_eq!(logins(&first), ["actor", "akowal"]);
    assert_eq!(logins(&second), ["jkowalski", "mhoffman"]);
    assert_eq!(logins(&third), ["zed"]);
    assert!(beyond.is_empty());
}

#[tokio::test]
async fn get_user_shows_the_failures_over_every_address_in_the_window() {
    let fixture = IdentityFixture::new();
    seed(&fixture, 22, "jkowalski", Role::ProcessEngineer);
    let typed = TypedLogin::parse("jkowalski").unwrap();
    let other = TypedLogin::parse("mhoffman").unwrap();
    for (ago, address) in [(60, 1), (120, 2), (300, 1), (600, 2)] {
        fixture
            .store
            .seed_attempt(&typed, ip(address), at(-ago), false);
    }
    fixture.store.seed_attempt(&typed, ip(1), at(-1000), false);
    fixture.store.seed_attempt(&typed, ip(1), at(-30), true);
    fixture.store.seed_attempt(&other, ip(1), at(-10), false);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let login = Login::parse("jkowalski").unwrap();
    let mark = fixture.store.journal().len();
    let detail = GetUser::new(fixture.context.clone())
        .execute(&auth, &meta, &login)
        .await
        .unwrap();
    assert_eq!(detail.user.login, login);
    assert_eq!(detail.user.role, Role::ProcessEngineer);
    assert_eq!(detail.throttle.failures, 4);
    assert_eq!(detail.throttle.last_failure, Some(at(-60)));
    assert!(only_read_only_transactions(&fixture, mark));
    let shown = format!("{detail:?}");
    assert!(!shown.contains("argon2id"), "{shown}");
    assert!(!shown.contains("password_hash"), "{shown}");
}

#[tokio::test]
async fn get_user_without_failures_has_no_last_failure() {
    let fixture = IdentityFixture::new();
    seed(&fixture, 22, "jkowalski", Role::Operator);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let detail = GetUser::new(fixture.context.clone())
        .execute(&auth, &meta, &Login::parse("jkowalski").unwrap())
        .await
        .unwrap();
    assert_eq!(detail.throttle.failures, 0);
    assert_eq!(detail.throttle.last_failure, None);
}

#[tokio::test]
async fn get_user_of_an_unknown_login_gives_e_web_005() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let nobody = Login::parse("nobody").unwrap();
    let error = GetUser::new(fixture.context.clone())
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
async fn the_user_form_lists_the_three_roles() {
    let fixture = IdentityFixture::new();
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let form = GetUserForm::new(fixture.context.clone())
        .execute(&auth, &meta)
        .await
        .unwrap();
    assert_eq!(form.roles, Role::ALL.to_vec());
}

#[tokio::test]
async fn the_permission_check_comes_first_in_every_user_manage_query() {
    for role in [Role::Operator, Role::ProcessEngineer] {
        let fixture = IdentityFixture::new();
        seed(&fixture, 22, "jkowalski", Role::Operator);
        let (auth, meta) = fixture.actor(role).await;
        let context = fixture.context.clone();
        let journal_before = fixture.store.journal().len();
        let known = Login::parse("jkowalski").unwrap();
        let unknown = Login::parse("nobody").unwrap();
        let errors = [
            ListUsers::new(context.clone())
                .execute(&auth, &meta, list(None, 0))
                .await
                .unwrap_err(),
            GetUser::new(context.clone())
                .execute(&auth, &meta, &known)
                .await
                .unwrap_err(),
            GetUser::new(context.clone())
                .execute(&auth, &meta, &unknown)
                .await
                .unwrap_err(),
            GetUserForm::new(context)
                .execute(&auth, &meta)
                .await
                .unwrap_err(),
        ];
        let operations = [
            Operation::ListUsers,
            Operation::GetUser,
            Operation::GetUser,
            Operation::GetUserForm,
        ];
        for (error, expected) in errors.iter().zip(operations) {
            assert!(
                matches!(error, AppError::Forbidden { operation } if *operation == expected),
                "{role:?} {error:?}"
            );
            assert_eq!(error.code(), ErrorCode::WEB_002);
        }
        let events = fixture.sink.events();
        assert_eq!(events.len(), 4, "{role:?}");
        assert!(
            events
                .iter()
                .all(|event| event.kind() == SecurityEventKind::Forbidden)
        );
        assert_eq!(fixture.store.journal().len(), journal_before, "{role:?}");
    }
}

#[tokio::test]
async fn the_own_profile_shows_login_role_and_the_flag_of_the_session_user() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", true);
    let token = fixture.seed_anna_session(7);
    let auth = fixture.authenticated(&token).await;
    let (_admin, meta) = fixture.actor(Role::Administrator).await;
    let profile = GetOwnProfile::new(fixture.context.clone())
        .execute(&auth, &meta)
        .await
        .unwrap();
    assert_eq!(profile.login, Login::parse("anna").unwrap());
    assert_eq!(profile.role, Role::Operator);
    assert!(profile.must_change_password);
}

#[tokio::test]
async fn the_own_password_form_is_open_to_every_authenticated_role() {
    for role in Role::ALL {
        let fixture = IdentityFixture::new();
        let (auth, meta) = fixture.actor(role).await;
        let form = GetOwnPasswordForm::new(fixture.context.clone())
            .execute(&auth, &meta)
            .await
            .unwrap();
        assert!(!form.must_change_password, "{role:?}");
        assert!(fixture.sink.events().is_empty(), "{role:?}");
    }
}

#[test]
fn the_login_form_needs_no_session_and_is_empty() {
    let form = GetLoginForm::new().execute().unwrap();
    assert_eq!(form, super::LoginForm);
}
