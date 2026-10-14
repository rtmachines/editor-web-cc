//! Atomic tests of the identity commands (task 5.7, `data-model.md` section 6).
//!
//! Each command has one midway failure test. A test trigger of the superuser makes one statement
//! of the command transaction fail, after earlier statements of that transaction ran. The failed
//! command leaves every `app_user` column, every session, and every attempt row as it was. The
//! head of the audit chain does not move.

use std::net::{IpAddr, Ipv4Addr};

use recipe_app::identity::{
    ActivateUser, ChangeOwnPassword, ChangeOwnPasswordInput, CreateUser, CreateUserInput,
    DeactivateUser, Login, LoginInput, Logout, ResetPassword, SetUserRole, SetUserRoleInput,
    UnlockLogin,
};
use recipe_domain::identity::{Login as UserLogin, Password, Role, SessionId, TypedLogin};

use crate::support::app::App;
use crate::support::fault::{FAULT, assert_audit_fault, assert_statement_fault};
use crate::support::identity::PASSWORD;

/// The address of the requests.
const CLIENT: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9));
/// A new password that meets the policy.
const NEW_PASSWORD: &str = "Replacement-password-2024";
/// The count of the attempts of a login.
const ATTEMPT_ROWS: &str = "SELECT count(*) FROM login_attempt WHERE login = 'target'";

fn login(text: &str) -> UserLogin {
    UserLogin::parse(text).unwrap()
}

#[tokio::test]
async fn login_fails_in_the_new_session_and_keeps_the_old_session() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let anna = app.seed_user("anna", Role::Operator, PASSWORD).await;
    let old = app.seed_session(anna).await;
    let state = app.identity_state().await;
    let head = app.head().await;
    app.install_probe("session", "BEFORE", "INSERT", FAULT)
        .await;
    let input = LoginInput {
        login: TypedLogin::parse("anna").unwrap(),
        password: Password::parse(PASSWORD).unwrap(),
        client_ip: CLIENT,
        old_session: Some(SessionId::of(&old)),
    };

    let error = Login::new(ctx).execute(&app.meta, input).await.unwrap_err();

    assert_statement_fault(&error);
    let after = app.identity_state().await;
    assert_eq!(after.users, state.users);
    assert_eq!(after.sessions, state.sessions);
    assert_eq!(app.rows("SELECT count(*) FROM login_attempt").await, 1);
    assert_eq!(
        app.rows("SELECT count(*) FROM login_attempt WHERE succeeded")
            .await,
        0
    );
    app.assert_chain_unchanged(&head).await;
}

#[tokio::test]
async fn logout_fails_in_the_audit_insert_and_keeps_the_session() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    let state = app.identity_state().await;
    let head = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = Logout::new(ctx)
        .execute(&auth, &app.meta)
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.identity_state().await, state);
    app.assert_chain_unchanged(&head).await;
}

#[tokio::test]
async fn create_user_fails_in_the_audit_insert_and_leaves_no_user() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    let state = app.identity_state().await;
    let head = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;
    let input = CreateUserInput {
        login: login("newcomer"),
        role: Role::Operator,
    };

    let error = CreateUser::new(ctx)
        .execute(&auth, &app.meta, input)
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.identity_state().await, state);
    app.assert_chain_unchanged(&head).await;
}

#[tokio::test]
async fn set_user_role_fails_in_the_audit_insert_and_keeps_role_and_sessions() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    let target = app.seed_user("target", Role::Operator, PASSWORD).await;
    app.seed_session(target).await;
    let state = app.identity_state().await;
    let head = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;
    let input = SetUserRoleInput {
        login: login("target"),
        role: Role::ProcessEngineer,
    };

    let error = SetUserRole::new(ctx)
        .execute(&auth, &app.meta, input)
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.identity_state().await, state);
    app.assert_chain_unchanged(&head).await;
}

#[tokio::test]
async fn deactivate_user_fails_in_the_session_delete_and_keeps_the_user_active() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    let target = app.seed_user("target", Role::Operator, PASSWORD).await;
    app.seed_session(target).await;
    let state = app.identity_state().await;
    let head = app.head().await;
    app.install_probe("session", "BEFORE", "DELETE", FAULT)
        .await;

    let error = DeactivateUser::new(ctx)
        .execute(&auth, &app.meta, &login("target"))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.identity_state().await, state);
    app.assert_chain_unchanged(&head).await;
}

#[tokio::test]
async fn activate_user_fails_in_the_audit_insert_and_keeps_the_user_inactive() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    let target = app.seed_user("target", Role::Operator, PASSWORD).await;
    app.make_inactive(target).await;
    let state = app.identity_state().await;
    let head = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = ActivateUser::new(ctx)
        .execute(&auth, &app.meta, &login("target"))
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.identity_state().await, state);
    app.assert_chain_unchanged(&head).await;
}

#[tokio::test]
async fn reset_password_fails_in_the_session_delete_and_keeps_hash_and_sessions() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    let target = app.seed_user("target", Role::Operator, PASSWORD).await;
    app.seed_session(target).await;
    let state = app.identity_state().await;
    let head = app.head().await;
    app.install_probe("session", "BEFORE", "DELETE", FAULT)
        .await;

    let error = ResetPassword::new(ctx)
        .execute(&auth, &app.meta, &login("target"))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.identity_state().await, state);
    app.assert_chain_unchanged(&head).await;
}

#[tokio::test]
async fn change_own_password_fails_in_the_new_session_and_keeps_hash_and_old_sessions() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let anna = app.seed_user("anna", Role::Operator, PASSWORD).await;
    let token = app.seed_session(anna).await;
    let auth = app.sign_in(&ctx, &token).await;
    let state = app.identity_state().await;
    let head = app.head().await;
    app.install_probe("session", "BEFORE", "INSERT", FAULT)
        .await;
    let input = ChangeOwnPasswordInput {
        client_ip: CLIENT,
        current_password: Password::parse(PASSWORD).unwrap(),
        new_password: Password::parse(NEW_PASSWORD).unwrap(),
    };

    let error = ChangeOwnPassword::new(ctx)
        .execute(&auth, &app.meta, input)
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    let after = app.identity_state().await;
    assert_eq!(after.users, state.users);
    assert_eq!(after.sessions, state.sessions);
    assert_eq!(
        app.rows("SELECT count(*) FROM login_attempt WHERE succeeded")
            .await,
        0
    );
    app.assert_chain_unchanged(&head).await;
}

#[tokio::test]
async fn unlock_login_fails_in_the_audit_insert_and_keeps_the_attempts() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    app.seed_user("target", Role::Operator, PASSWORD).await;
    app.seed_attempts("target", "10.0.0.5", 3).await;
    let state = app.identity_state().await;
    let head = app.head().await;
    assert_eq!(app.rows(ATTEMPT_ROWS).await, 3);
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = UnlockLogin::new(ctx)
        .execute(&auth, &app.meta, &login("target"), CLIENT)
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.identity_state().await, state);
    assert_eq!(app.rows(ATTEMPT_ROWS).await, 3);
    app.assert_chain_unchanged(&head).await;
}
