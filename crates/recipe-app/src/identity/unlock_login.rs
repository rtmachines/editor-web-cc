//! The use case `UnlockLogin` (design D8 i, j, and q, `data-model.md` section 6).

use std::net::IpAddr;

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange, SecurityEvent};
use recipe_domain::identity::{Login, TypedLogin, UserId};
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;

use super::{Authenticated, IdentityContext};
use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::{UseCase, audit_context};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{AttemptRow, Isolation, Locked, NonEmpty};

/// Removes the failed attempts of a login.
///
/// The use case solves one problem: an Administrator must end the delay of a user who is locked
/// out, and the audit log must show which attempts the unlock removed. The unlock deletes every
/// attempt of the login in one transaction. The entry `LoginUnlocked` holds at most
/// `audit.unlock_rows` of them and the full count. The security event `unlock` follows the commit.
#[derive(Clone)]
pub struct UnlockLogin {
    ctx: IdentityContext,
}

impl UseCase for UnlockLogin {
    const OPERATION: Operation = Operation::UnlockLogin;
}

impl UnlockLogin {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Delete the attempts of `login` and record the unlock.
    ///
    /// `client_ip` is the canonical address of the request. The event `unlock` needs an address,
    /// as `Login` does, so the caller passes it and the case of a missing address cannot occur.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the login is unknown.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    /// A failed write of the event `unlock` also returns the storage error. The unlock stays
    /// committed then, and a repeated request deletes nothing more (ADR-0035 j).
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        login: &Login,
        client_ip: IpAddr,
    ) -> Result<(), AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        let target = self.delete_attempts(auth, meta, login).await?;
        let event = SecurityEvent::unlock(
            target,
            auth.user().user(),
            meta.request_id(),
            client_ip,
            meta.user_agent().cloned(),
        );
        self.ctx.app.security.record(event).await?;
        Ok(())
    }

    /// Delete the attempts and commit `LoginUnlocked`. Return the user that was unlocked.
    async fn delete_attempts(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        login: &Login,
    ) -> Result<UserId, AppError> {
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadCommitted).await?;
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
        let typed = TypedLogin::parse(locked.login.as_str())
            .map_err(|_reason| StorageError::invariant("a login is not a typed login"))?;
        let gone = tx
            .attempts()
            .delete_for_login(&typed, self.ctx.settings.unlock_rows)
            .await?;
        let row = before.into_value();
        let entry = AuditEntry::row(
            RowAction::LoginUnlocked,
            AuditTarget::app_user(locked.id),
            RowChange::Update {
                before: with_attempts(row.clone(), &gone.newest, gone.count)?,
                after: with_attempts(row, &[], 0)?,
            },
            audit_context(auth.user(), meta),
        );
        self.ctx.app.uow.commit(tx, NonEmpty::new(entry)).await?;
        Ok(locked.id)
    }
}

/// Add the child array `login_attempts` and the field `login_attempt_count` to a user image.
fn with_attempts(image: Value, attempts: &[AttemptRow], count: u64) -> Result<Value, StorageError> {
    let Value::Object(mut fields) = image else {
        return Err(StorageError::invariant(
            "the row image of a user is no object",
        ));
    };
    let rows = attempts
        .iter()
        .map(attempt_value)
        .collect::<Result<Vec<Value>, StorageError>>()?;
    fields.insert("login_attempts".to_owned(), Value::Array(rows));
    fields.insert("login_attempt_count".to_owned(), Value::from(count));
    Ok(Value::Object(fields))
}

/// Return the columns that the delete returns for one attempt.
///
/// The time is RFC 3339 text, like the text that `to_jsonb` writes in the rest of the image on
/// PostgreSQL.
fn attempt_value(attempt: &AttemptRow) -> Result<Value, StorageError> {
    let attempted_at = attempt
        .attempted_at
        .as_offset_date_time()
        .format(&Rfc3339)
        .map_err(|_failed| {
            StorageError::invariant("the time of an attempt has no RFC 3339 text")
        })?;
    Ok(json!({
        "id": attempt.id.get(),
        "client_ip": attempt.client_ip.to_string(),
        "attempted_at": attempted_at,
        "succeeded": attempt.succeeded,
    }))
}

#[cfg(test)]
mod tests;
