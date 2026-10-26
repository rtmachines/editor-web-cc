//! The transaction that `SetUserRole` and `DeactivateUser` share (design D8, ADR-0096).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::identity::{
    AdminChange, IdentityError, Login, UserStanding, last_administrator_guard,
};

use super::{Authenticated, IdentityContext};
use crate::auth::RequestMeta;
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
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) when the login is unknown.
/// Returns `UserAlreadyInactive` (`E-IDN-010`) for a deactivation of an inactive user.
/// Returns `LastAdministrator` (`E-IDN-002`) when the change removes the last one.
/// Returns `AppError::Storage` with `E-DB-001` after a serialization failure, a deadlock, or a
/// lock timeout at the guard lock. Returns the other storage errors unchanged.
pub(super) async fn run_admin_change(
    ctx: &IdentityContext,
    auth: &Authenticated,
    meta: &RequestMeta,
    login: &Login,
    change: AdminChange,
    action: RowAction,
) -> Result<(), AppError> {
    let mut tx = ctx.app.uow.begin(Isolation::ReadCommitted).await?;
    tx.users().lock_administrator_guard().await?;
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
