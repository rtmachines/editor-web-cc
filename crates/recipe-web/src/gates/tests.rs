//! Tests of the gate rules of every `Access` arm (design D11 step 9, D14, Assumption A24).
//!
//! The session need of each arm lives in `session.rs`. The HTTP test of the compare route covers
//! the need of `Access::Authenticated`.

use recipe_app::auth::Operation;

use crate::routes::Access;

/// The operations that a user with `must_change_password` may still use (`api.md` section 7.4).
const OPEN_WHILE_PASSWORD_CHANGE: [Operation; 3] = [
    Operation::GetOwnPasswordForm,
    Operation::ChangeOwnPassword,
    Operation::Logout,
];

#[test]
fn gates_operation_arm_follows_the_operation_in_read_only_mode() {
    for operation in Operation::ALL {
        assert_eq!(
            Access::Operation(operation).runs_in_read_only(),
            operation.runs_in_read_only(),
            "{operation}"
        );
    }
    assert!(Access::Operation(Operation::Login).runs_in_read_only());
    assert!(Access::Operation(Operation::Logout).runs_in_read_only());
    assert!(Access::Operation(Operation::ListUsers).runs_in_read_only());
    assert!(!Access::Operation(Operation::CreateUser).runs_in_read_only());
}

#[test]
fn gates_operation_arm_opens_only_three_routes_while_the_password_must_change() {
    for operation in Operation::ALL {
        assert_eq!(
            Access::Operation(operation).open_while_password_change(),
            OPEN_WHILE_PASSWORD_CHANGE.contains(&operation),
            "{operation}"
        );
    }
}

#[test]
fn gates_authenticated_arm_runs_in_read_only_mode() {
    assert!(Access::Authenticated.runs_in_read_only());
}

#[test]
fn gates_authenticated_arm_is_closed_while_the_password_must_change() {
    assert!(!Access::Authenticated.open_while_password_change());
}

#[test]
fn gates_static_arm_runs_in_read_only_mode_and_stays_open_while_the_password_must_change() {
    assert!(Access::Static.runs_in_read_only());
    assert!(Access::Static.open_while_password_change());
}

#[test]
fn gates_lang_arm_runs_in_read_only_mode_and_stays_open_while_the_password_must_change() {
    assert!(Access::Lang.runs_in_read_only());
    assert!(Access::Lang.open_while_password_change());
}
