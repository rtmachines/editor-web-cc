//! Samples of `IdentityError`, one for each variant.

use recipe_domain::identity::{CredentialFailure, IdentityError, Login, PasswordReason};

use super::sample::Sample;

/// Return the name `Type::Variant` of a sample. The `match` has no wildcard, so a new variant stops the build.
fn name_of(error: &IdentityError) -> &'static str {
    match error {
        IdentityError::DuplicateLogin { .. } => "IdentityError::DuplicateLogin",
        IdentityError::LastAdministrator { .. } => "IdentityError::LastAdministrator",
        IdentityError::InvalidCredentials { .. } => "IdentityError::InvalidCredentials",
        IdentityError::UserInactive => "IdentityError::UserInactive",
        IdentityError::LoginThrottled { .. } => "IdentityError::LoginThrottled",
        IdentityError::PasswordPolicy { .. } => "IdentityError::PasswordPolicy",
        IdentityError::MustChangePassword => "IdentityError::MustChangePassword",
        IdentityError::InvalidCurrentPassword => "IdentityError::InvalidCurrentPassword",
        IdentityError::UserAlreadyActive { .. } => "IdentityError::UserAlreadyActive",
        IdentityError::UserAlreadyInactive { .. } => "IdentityError::UserAlreadyInactive",
        IdentityError::TechnicalRoleChange { .. } => "IdentityError::TechnicalRoleChange",
    }
}

fn login() -> Login {
    Login::parse("alice").unwrap()
}

/// Return one sample for each variant of `IdentityError`.
pub fn samples() -> Vec<Sample> {
    let errors = [
        IdentityError::DuplicateLogin { login: login() },
        IdentityError::LastAdministrator { login: login() },
        IdentityError::InvalidCredentials {
            cause: CredentialFailure::UnknownLogin,
        },
        IdentityError::UserInactive,
        IdentityError::LoginThrottled { retry_after: 30 },
        IdentityError::PasswordPolicy {
            reason: PasswordReason::TooShort,
        },
        IdentityError::MustChangePassword,
        IdentityError::InvalidCurrentPassword,
        IdentityError::UserAlreadyActive { login: login() },
        IdentityError::UserAlreadyInactive { login: login() },
        IdentityError::TechnicalRoleChange { login: login() },
    ];
    errors
        .iter()
        .map(|error| Sample {
            name: name_of(error),
            code: error.code(),
        })
        .collect()
}
