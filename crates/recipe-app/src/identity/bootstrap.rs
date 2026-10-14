//! The creation of the first Administrator (design D17, ADR-0035 h).

use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::identity::{IdentityError, Login, NewPassword, PasswordHash, Role, UserId};

use super::issue::random_uuid;
use super::{HashGate, serializable_once_more};
use crate::error::{APP_USER_LOGIN_UNIQUE, AppError, map_storage};
use crate::ports::{Isolation, NewUser, NonEmpty, UnitOfWork};

/// The result of [`bootstrap_administrator`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapOutcome {
    /// The call created an active Administrator with `must_change_password`.
    Created,
    /// An active Administrator exists already. The call wrote nothing.
    AdministratorExists,
}

/// Create the first Administrator when no active Administrator exists.
///
/// The function solves one problem: a new database has no user who can log in, and no user can
/// create the first one. The subcommand `create-admin` of `recipe-server` calls it.
///
/// The function has no `Operation` and no permission check, like `EnsureDerivedType`. The caller
/// has no `CurrentUser`. ADR-0013 and ADR-0035 h make `recipe-server` the only caller, and a test of
/// `recipe-web` fails when its sources name the function.
///
/// The function hashes the password under a permit first. It then runs one `SERIALIZABLE`
/// transaction, and it runs that transaction once more after a serialization failure (`40001`).
/// The transaction counts the active Administrators. A count above 0 ends the call with
/// [`BootstrapOutcome::AdministratorExists`] and no write. Else the transaction inserts the user
/// and commits `UserCreated` with no actor, no client address, and no session reference.
///
/// # Errors
///
/// Returns `Timeout` (`E-WEB-009`) when no Argon2 permit is free in time.
/// Returns `TaskFailed` (`E-WEB-011`) when the hash task fails.
/// Returns `DuplicateLogin` (`E-IDN-001`) when another user has the login.
/// Returns `AppError::Storage` with `E-DB-001` after a second serialization failure.
/// Returns `AppError::Storage` or `AppError::AuditWriteFailed` for the other database failures.
pub async fn bootstrap_administrator(
    uow: &dyn UnitOfWork,
    gate: &HashGate,
    login: Login,
    password: NewPassword,
    request_id: RequestId,
) -> Result<BootstrapOutcome, AppError> {
    let permit = gate.acquire().await?;
    let (hash, permit) = permit.hash(password).await?;
    drop(permit);
    serializable_once_more(|| create_when_none(uow, &login, &hash, request_id)).await
}

/// One run of the transaction.
async fn create_when_none(
    uow: &dyn UnitOfWork,
    login: &Login,
    hash: &PasswordHash,
    request_id: RequestId,
) -> Result<BootstrapOutcome, AppError> {
    let mut tx = uow.begin(Isolation::Serializable).await?;
    if tx.users().count_active_administrators().await? > 0 {
        return Ok(BootstrapOutcome::AdministratorExists);
    }
    let user = NewUser {
        id: UserId::from(random_uuid()?),
        login: login.clone(),
        password_hash: hash.clone(),
        role: Role::Administrator,
        must_change_password: true,
    };
    let after = tx.users().insert(&user).await.map_err(|error| {
        map_storage(error, |constraint| {
            (constraint == APP_USER_LOGIN_UNIQUE).then(|| {
                IdentityError::DuplicateLogin {
                    login: login.clone(),
                }
                .into()
            })
        })
    })?;
    let entry = AuditEntry::row(
        RowAction::UserCreated,
        AuditTarget::app_user(user.id),
        RowChange::Create {
            after: after.into_value(),
        },
        AuditContext {
            actor: None,
            request_id,
            client_ip: None,
            session_ref: None,
        },
    );
    uow.commit(tx, NonEmpty::new(entry)).await?;
    Ok(BootstrapOutcome::Created)
}

#[cfg(test)]
mod tests;
