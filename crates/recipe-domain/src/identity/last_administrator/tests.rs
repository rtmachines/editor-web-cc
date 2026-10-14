use super::*;
use crate::identity::{IdentityError, Login, Role};

fn target(role: Role, is_active: bool) -> UserStanding {
    UserStanding {
        login: Login::parse("jkowalski").unwrap(),
        role,
        is_active,
    }
}

fn refused() -> IdentityError {
    IdentityError::LastAdministrator {
        login: Login::parse("jkowalski").unwrap(),
    }
}

#[test]
fn the_only_active_administrator_cannot_be_demoted() {
    for role in [Role::ProcessEngineer, Role::Operator] {
        let result = last_administrator_guard(
            &target(Role::Administrator, true),
            AdminChange::SetRole(role),
            1,
        );
        assert_eq!(result, Err(refused()));
    }
}

#[test]
fn the_only_active_administrator_cannot_be_deactivated() {
    let result = last_administrator_guard(
        &target(Role::Administrator, true),
        AdminChange::Deactivate,
        1,
    );
    assert_eq!(result, Err(refused()));
}

#[test]
fn a_count_of_zero_still_refuses_the_removal() {
    let result = last_administrator_guard(
        &target(Role::Administrator, true),
        AdminChange::Deactivate,
        0,
    );
    assert_eq!(result, Err(refused()));
}

#[test]
fn with_two_active_administrators_the_change_passes() {
    let administrator = target(Role::Administrator, true);
    assert_eq!(
        last_administrator_guard(&administrator, AdminChange::SetRole(Role::Operator), 2),
        Ok(())
    );
    assert_eq!(
        last_administrator_guard(&administrator, AdminChange::Deactivate, 2),
        Ok(())
    );
}

#[test]
fn an_inactive_administrator_can_change_when_one_other_administrator_is_active() {
    let inactive = target(Role::Administrator, false);
    assert_eq!(
        last_administrator_guard(&inactive, AdminChange::SetRole(Role::Operator), 1),
        Ok(())
    );
    assert_eq!(
        last_administrator_guard(&inactive, AdminChange::Deactivate, 1),
        Ok(())
    );
}

#[test]
fn a_user_who_is_not_an_administrator_can_change() {
    for role in [Role::ProcessEngineer, Role::Operator] {
        let user = target(role, true);
        assert_eq!(
            last_administrator_guard(&user, AdminChange::Deactivate, 1),
            Ok(())
        );
        assert_eq!(
            last_administrator_guard(&user, AdminChange::SetRole(Role::Administrator), 1),
            Ok(())
        );
    }
}

#[test]
fn keeping_the_administrator_role_passes_for_the_last_administrator() {
    let result = last_administrator_guard(
        &target(Role::Administrator, true),
        AdminChange::SetRole(Role::Administrator),
        1,
    );
    assert_eq!(result, Ok(()));
}

#[test]
fn the_refusal_has_the_code_of_the_last_administrator() {
    let error = last_administrator_guard(
        &target(Role::Administrator, true),
        AdminChange::Deactivate,
        1,
    )
    .unwrap_err();
    assert_eq!(error.code(), crate::error_codes::ErrorCode::IDN_002);
}
