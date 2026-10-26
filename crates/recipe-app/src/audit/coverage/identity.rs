//! Audit coverage of the identity use cases.

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{AggregateType, AuditAction, AuditEntry, RequestId};
use recipe_domain::identity::{Login as UserLogin, Password, Role, TypedLogin};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::assert_trail;
use crate::auth::{CurrentUser, RequestMeta};
use crate::identity::fixtures::{IdentityFixture, anna_id};
use crate::identity::{
    ActivateUser, Authenticated, ChangeOwnPassword, ChangeOwnPasswordInput, CreateUser,
    CreateUserInput, DeactivateUser, Login, LoginInput, Logout, ResetPassword, SetUserRole,
    SetUserRoleInput, UnlockLogin,
};

/// The password of the fixture user `anna`.
const PASSWORD: &str = "correct-horse-battery";
/// The start of the clock of a fixture, in Unix seconds.
const START_SECONDS: i64 = 1_000_000;
/// The new password of the change of `anna`.
const NEW_PASSWORD: &str = "brand-new-secret-phrase";

/// Return the client address of the identity calls.
fn client() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))
}

/// Return the login `anna` as a domain value.
fn anna_login() -> UserLogin {
    UserLogin::parse("anna").unwrap()
}

/// Return a fixture with the active Operator `anna`, who has the password `PASSWORD`.
fn fixture_with_anna() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.seed_anna(PASSWORD, false);
    fixture
}

/// Build the request of the session that `auth` holds.
fn session_meta(auth: &Authenticated) -> RequestMeta {
    RequestMeta::new(
        RequestId::from(Uuid::from_u128(0x88)),
        Some(client()),
        None,
        Some(auth.session_ref().clone()),
    )
}

/// Assert that no image of an audit entry holds the key `password_hash`.
fn assert_no_password_hash(entries: &[AuditEntry], name: &str) {
    for entry in entries {
        for image in [entry.before(), entry.after()].into_iter().flatten() {
            assert!(
                image.get("password_hash").is_none(),
                "{name}: password_hash in an image"
            );
        }
    }
}

#[tokio::test]
async fn login_writes_user_logged_in_for_the_user_that_logs_in() {
    let fixture = fixture_with_anna();
    let request = RequestMeta::new(RequestId::from(Uuid::from_u128(0x77)), None, None, None);
    let input = LoginInput {
        login: TypedLogin::parse("anna").unwrap(),
        password: Password::parse(PASSWORD).unwrap(),
        client_ip: client(),
        old_session: None,
    };
    Login::new(fixture.context.clone())
        .execute(&request, input)
        .await
        .unwrap();
    let anna = CurrentUser::new(anna_id(), anna_login(), Role::Operator);
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        &anna,
        &request,
        "Login",
        &[(
            AuditAction::UserLoggedIn,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "Login");
}

#[tokio::test]
async fn logout_writes_user_logged_out() {
    let fixture = fixture_with_anna();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let request = session_meta(&auth);
    Logout::new(fixture.context.clone())
        .execute(&auth, &request)
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "Logout",
        &[(
            AuditAction::UserLoggedOut,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "Logout");
}

#[tokio::test]
async fn create_user_writes_user_created() {
    let fixture = IdentityFixture::new();
    let (auth, request) = fixture.actor(Role::Administrator).await;
    let input = CreateUserInput {
        login: UserLogin::parse("berta").unwrap(),
        role: Role::Operator,
    };
    CreateUser::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "CreateUser",
        &[(AuditAction::UserCreated, "app_user", AggregateType::AppUser)],
    );
    assert_no_password_hash(&entries, "CreateUser");
}

#[tokio::test]
async fn set_user_role_writes_user_role_set() {
    let fixture = fixture_with_anna();
    let (auth, request) = fixture.actor(Role::Administrator).await;
    let input = SetUserRoleInput {
        login: anna_login(),
        role: Role::ProcessEngineer,
    };
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "SetUserRole",
        &[(AuditAction::UserRoleSet, "app_user", AggregateType::AppUser)],
    );
    assert_no_password_hash(&entries, "SetUserRole");
}

#[tokio::test]
async fn deactivate_user_writes_user_deactivated() {
    let fixture = fixture_with_anna();
    let (auth, request) = fixture.actor(Role::Administrator).await;
    DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &request, &anna_login())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "DeactivateUser",
        &[(
            AuditAction::UserDeactivated,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "DeactivateUser");
}

#[tokio::test]
async fn activate_user_writes_user_activated() {
    let fixture = fixture_with_anna();
    fixture.store.update_state(|state, now| {
        let row = state.users.get_mut(&anna_id()).unwrap();
        row.is_active = false;
        row.deactivated_at = Some(now);
        row.session_epoch = 3;
    });
    let (auth, request) = fixture.actor(Role::Administrator).await;
    ActivateUser::new(fixture.context.clone())
        .execute(&auth, &request, &anna_login())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "ActivateUser",
        &[(
            AuditAction::UserActivated,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "ActivateUser");
}

#[tokio::test]
async fn reset_password_writes_password_reset() {
    let fixture = fixture_with_anna();
    let (auth, request) = fixture.actor(Role::Administrator).await;
    ResetPassword::new(fixture.context.clone())
        .execute(&auth, &request, &anna_login())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "ResetPassword",
        &[(
            AuditAction::PasswordReset,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "ResetPassword");
}

#[tokio::test]
async fn change_own_password_writes_own_password_changed() {
    let fixture = fixture_with_anna();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let request = session_meta(&auth);
    let input = ChangeOwnPasswordInput {
        client_ip: client(),
        current_password: Password::parse(PASSWORD).unwrap(),
        new_password: Password::parse(NEW_PASSWORD).unwrap(),
    };
    ChangeOwnPassword::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "ChangeOwnPassword",
        &[(
            AuditAction::OwnPasswordChanged,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "ChangeOwnPassword");
}

#[tokio::test]
async fn unlock_login_writes_login_unlocked() {
    let fixture = fixture_with_anna();
    for offset in [0_i64, 1] {
        let seconds = START_SECONDS.checked_add(offset).unwrap();
        let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap());
        fixture
            .store
            .seed_attempt(&TypedLogin::parse("anna").unwrap(), client(), at, false);
    }
    let (auth, request) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &request, &anna_login(), client())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "UnlockLogin",
        &[(
            AuditAction::LoginUnlocked,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "UnlockLogin");
    let listed = entries
        .first()
        .and_then(|entry| entry.before())
        .and_then(|image| image.get("login_attempts"))
        .and_then(|value| value.as_array());
    assert_eq!(
        listed.map(Vec::len),
        Some(2),
        "UnlockLogin: listed attempts"
    );
}
