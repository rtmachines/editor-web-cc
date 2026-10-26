//! The transaction that `SetUserRole` and `DeactivateUser` share (design D8, ADR-0096).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::identity::{
    AdminChange, IdentityError, Login, UserStanding, last_administrator_guard,
};

use super::{Authenticated, IdentityContext};
use crate::auth::{Operation, RequestMeta, refuse};
use crate::context::audit_context;
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Locked, NonEmpty};

/// Run an administrative change of one user in a `READ COMMITTED` transaction.
///
/// The function solves one problem: two Administrators who demote each other at the same time
/// must not leave the system without an active Administrator, and one use case body serves both
/// commands. The body takes the guard lock of the last Administrator first (ADR-0096). The lock
/// makes the commands run one after the other, so the count of active Administrators sees each
/// earlier commit. The body then locks the user, counts the active Administrators, applies the
/// last-administrator rule, writes the change, ends every session of the user, and commits one
/// audit entry. The function never runs the body again.
///
/// The permission check of the use case runs before the transaction. The wait for the guard lock
/// can last while another Administrator demotes or deactivates the actor. So the body reads the
/// row of the actor again after it holds the lock and refuses when the actor lost the permission
/// or the active flag. That read is safe without a row lock, because every command that changes
/// the role or the active flag of an Administrator holds the guard lock.
///
/// # Errors
///
/// Returns `Forbidden` (`E-WEB-002`) when the actor lost the permission or the active flag while
/// the command waited for the guard lock. The command then writes nothing.
/// Returns `NotFound` (`E-WEB-005`) when the login is unknown.
/// Returns `UserAlreadyInactive` (`E-IDN-010`) for a deactivation of an inactive user.
/// Returns `LastAdministrator` (`E-IDN-002`) when the change removes the last one.
/// Returns `AppError::Storage` with `E-DB-001` after a serialization failure, a deadlock, or a
/// lock timeout at the guard lock. Returns the other storage errors unchanged.
pub(super) async fn run_admin_change(
    ctx: &IdentityContext,
    auth: &Authenticated,
    meta: &RequestMeta,
    operation: Operation,
    login: &Login,
    change: AdminChange,
    action: RowAction,
) -> Result<(), AppError> {
    let mut tx = ctx.app.uow.begin(Isolation::ReadCommitted).await?;
    tx.users().lock_administrator_guard().await?;
    let actor = tx.users().find_by_login(auth.user().login()).await?;
    if !actor.is_some_and(|row| row.is_active && operation.requirement().allows(row.role)) {
        // Roll back first, so the guard lock is free while the sink writes the event.
        drop(tx);
        return refuse(operation, auth.user(), meta, ctx.app.security.as_ref()).await;
    }
    let Locked {
        state: locked,
        before,
    } = tx
        .users()
        .lock_by_login(login)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::User(login.clone()),
        })?;
    if matches!(change, AdminChange::Deactivate) && !locked.is_active {
        return Err(IdentityError::UserAlreadyInactive {
            login: login.clone(),
        }
        .into());
    }
    let active_administrators = tx.users().count_active_administrators().await?;
    let standing = UserStanding {
        login: login.clone(),
        role: locked.role,
        is_active: locked.is_active,
    };
    last_administrator_guard(&standing, change, active_administrators)?;
    let after = match change {
        AdminChange::SetRole(role) => tx.users().set_role(locked.id, role).await?,
        AdminChange::Deactivate => tx.users().deactivate(locked.id).await?,
    };
    tx.sessions().delete_all_of_user(locked.id).await?;
    let entry = AuditEntry::row(
        action,
        AuditTarget::app_user(locked.id),
        RowChange::Update {
            before: before.into_value(),
            after: after.into_value(),
        },
        audit_context(auth.user(), meta),
    );
    ctx.app.uow.commit(tx, NonEmpty::new(entry)).await?;
    Ok(())
}
