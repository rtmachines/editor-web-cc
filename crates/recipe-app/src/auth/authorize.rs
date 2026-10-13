//! The permission check that starts every use case.

use recipe_domain::audit::SecurityEvent;

use super::{CurrentUser, Operation, RequestMeta};
use crate::error::AppError;
use crate::ports::SecurityEventSink;

/// Check that the role of `user` may run `op`.
///
/// The function solves one problem: every use case must refuse a forbidden call in the same
/// way and must leave a trace of it (ADR-0013, R28). A refusal records one `forbidden`
/// security event and returns `E-WEB-002`. When the sink fails, the function returns the
/// storage error, so a refusal is never invisible (design D18). The function logs nothing.
///
/// # Errors
///
/// Returns `AppError::Forbidden` when the role lacks the permission.
/// Returns `AppError::Storage` when the sink cannot write the event.
pub async fn authorize(
    op: Operation,
    user: &CurrentUser,
    meta: &RequestMeta,
    sink: &dyn SecurityEventSink,
) -> Result<(), AppError> {
    if op.requirement().allows(user.role()) {
        return Ok(());
    }
    let event = SecurityEvent::forbidden(
        user.user(),
        op.name(),
        meta.request_id(),
        meta.client_ip(),
        meta.user_agent().cloned(),
    );
    sink.record(event).await?;
    Err(AppError::Forbidden { operation: op })
}

#[cfg(test)]
mod tests;
