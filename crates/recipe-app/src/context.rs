//! Shared items of every use case (design D7).
//!
//! Every use case holds an [`AppContext`], declares its operation through [`UseCase`],
//! and builds its audit entries with [`audit_context`].

use std::sync::Arc;

use recipe_domain::audit::AuditContext;

use crate::auth::{CurrentUser, Operation, RequestMeta};
use crate::blocking::{AppSettings, DiffGate};
use crate::ports::{Clock, SecurityEventSink, UnitOfWork};

/// The ports and settings that every use case needs.
///
/// The type solves one problem: a use case must not know which adapter runs it.
/// `recipe-server` wires the `sqlx` adapters, and the tests wire the in-memory adapters.
/// A clone shares the adapters.
#[derive(Clone)]
///
/// The fields are private to the crate. A holder outside the crate cannot open a transaction,
/// write, and commit without a use case.
pub struct AppContext {
    /// The opener of transactions and the only commit path.
    pub(crate) uow: Arc<dyn UnitOfWork>,
    /// The sink of security events, such as the `forbidden` event.
    pub(crate) security: Arc<dyn SecurityEventSink>,
    /// The source of the current instant.
    pub(crate) clock: Arc<dyn Clock>,
    /// The settings of `reliability.md` section 7.11.
    pub(crate) settings: AppSettings,
    /// The gate of the parallel diffs (`diff.permits`). Every clone shares it (ADR-0050 (g)).
    pub(crate) diff_gate: DiffGate,
}

impl AppContext {
    /// Build the context from the adapters and the settings.
    ///
    /// `recipe-server` calls it once at startup with the `sqlx` adapters. The context builds the
    /// gate of `diff.permits` from the settings (ADR-0050 (g)).
    #[must_use]
    pub fn new(
        uow: Arc<dyn UnitOfWork>,
        security: Arc<dyn SecurityEventSink>,
        clock: Arc<dyn Clock>,
        settings: AppSettings,
    ) -> Self {
        Self {
            uow,
            security,
            clock,
            diff_gate: DiffGate::new(settings.diff.permits),
            settings,
        }
    }

    /// Return the sink of security events that the context holds.
    ///
    /// The accessor solves one problem: `recipe-web` records the event `csrf_rejected` and must
    /// use the same sink as the use cases. A second argument could name another sink.
    #[must_use]
    pub fn security(&self) -> Arc<dyn SecurityEventSink> {
        Arc::clone(&self.security)
    }
}

/// A use case with a fixed operation in the catalog.
///
/// The trait solves one problem: the permission check of a use case must name an operation
/// of the catalog, so a use case without a catalog row does not compile (design D7).
pub trait UseCase {
    /// The operation that `authorize` checks first.
    const OPERATION: Operation;
}

/// Build the audit context of an entry from the user and the request data.
///
/// The function solves one problem: every use case copies the actor, the request identifier,
/// the client address, and the session reference in the same way.
#[must_use]
pub fn audit_context(user: &CurrentUser, meta: &RequestMeta) -> AuditContext {
    AuditContext {
        actor: Some(user.user()),
        request_id: meta.request_id(),
        client_ip: meta.client_ip(),
        session_ref: meta.session_ref().cloned(),
    }
}

#[cfg(test)]
mod tests;
