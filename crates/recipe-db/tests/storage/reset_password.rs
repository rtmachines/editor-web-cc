//! Storage tests of `ResetPassword` for an `ExportClient` account (task 4.5, design D6).
//!
//! A technical account never has `must_change_password`.
//! The `CHECK` `app_user_export_client_password` enforces it.
//! The reset keeps the flag false, replaces the hash, and writes one `PasswordReset` entry.
//! The new password passes the Basic check and the old password fails it.

use recipe_app::identity::{BasicCredentials, ResetPassword, authenticate_basic};
use recipe_domain::identity::{Login, Password, Role, TypedLogin};

use crate::support::app::App;
use crate::support::identity::PASSWORD;

/// The login of the technical account.
const LOGIN: &str = "exporter";
/// The count of the audit rows of a `PasswordReset` entry.
const RESET_ENTRIES: &str = "SELECT count(*) FROM audit_log WHERE action = 'PasswordReset'";
/// The flag of the account.
const FLAG: &str = "SELECT must_change_password::text FROM app_user WHERE login = 'exporter'";

fn credentials(password: &str) -> BasicCredentials {
    BasicCredentials {
        login: TypedLogin::parse(LOGIN).unwrap(),
        password: Password::parse(password).unwrap(),
    }
}

#[tokio::test]
async fn reset_password_of_an_export_client_keeps_the_flag_false_and_writes_one_entry() {
    let mut app = App::create().await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    app.seed_user(LOGIN, Role::ExportClient, PASSWORD).await;
    let before = app.rows("SELECT count(*) FROM audit_log").await;

    let issued = ResetPassword::new(ctx)
        .execute(&auth, &app.meta, &Login::parse(LOGIN).unwrap())
        .await
        .unwrap();

    assert_eq!(issued.role(), Role::ExportClient);
    assert_eq!(app.text(FLAG).await, "false");
    assert_eq!(app.rows(RESET_ENTRIES).await, 1);
    assert_eq!(app.rows("SELECT count(*) FROM audit_log").await, before + 1);
    app.assert_user_is_reset(issued.password().as_str()).await;
}

impl App {
    /// Check the Basic check of the account: the new password passes and the old one fails.
    async fn assert_user_is_reset(&mut self, new_password: &str) {
        let ctx = self.identity();

        let new = authenticate_basic(&ctx, credentials(new_password), &self.meta).await;
        let old = authenticate_basic(&ctx, credentials(PASSWORD), &self.meta).await;

        let principal = new.unwrap();
        assert_eq!(principal.user.role(), Role::ExportClient);
        assert!(old.is_err(), "the old password must fail the Basic check");
    }
}
