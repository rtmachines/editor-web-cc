//! The query `GetAuditEntry` (spec "Audit entry with its states", ADR-0051 (c), (f)).

use std::net::IpAddr;

use recipe_domain::audit::{RequestId, SessionRef};
use recipe_domain::identity::Permission;
use serde_json::Value;

use super::scope::audit_scope;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing};
use crate::ports::{AuditEntryId, AuditRecord, AuditRow, Isolation};

/// The trace fields of an entry. Only a role with `AuditView` sees them (ADR-0051 (f)).
///
/// The gaps of `chain_seq` would show the entries outside the scope of `AuditViewRecipes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryTrace {
    /// The position of the entry in the hash chain.
    pub chain_seq: i64,
    /// The client address of the request.
    pub client_ip: Option<IpAddr>,
    /// The session reference of the request.
    pub session_ref: Option<SessionRef>,
}

/// The result of `GetAuditEntry`: the entry with its states.
///
/// The view holds no `entry_hash`, no `prev_hash`, and no `login_hmac`. A state never holds a
/// password hash.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditEntryView {
    /// The fields of the list row.
    pub row: AuditRow,
    /// The request that wrote the entry.
    pub request_id: RequestId,
    /// The state before the change.
    pub before: Option<Value>,
    /// The state after the change.
    pub after: Option<Value>,
    /// The trace fields, or `None` for a role without `AuditView`.
    pub trace: Option<EntryTrace>,
}

impl AuditEntryView {
    /// Build the view of `record`. The view keeps the trace fields only when `with_trace` holds.
    fn of(record: AuditRecord, with_trace: bool) -> Self {
        let trace = with_trace.then_some(EntryTrace {
            chain_seq: record.chain_seq,
            client_ip: record.client_ip,
            session_ref: record.session_ref,
        });
        Self {
            row: record.row,
            request_id: record.request_id,
            before: record.before,
            after: record.after,
            trace,
        }
    }
}

/// Shows one audit entry with its states.
///
/// The query solves one problem: an auditor must read the full states of one change. An entry
/// outside the scope of the role gives the answer of an absent entry (A20).
#[derive(Clone)]
pub struct GetAuditEntry {
    ctx: AppContext,
}

impl UseCase for GetAuditEntry {
    const OPERATION: Operation = Operation::GetAuditEntry;
}

impl GetAuditEntry {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the entry `id` with its states.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the entry does not exist or is outside the scope.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        id: AuditEntryId,
    ) -> Result<AuditEntryView, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let scope = audit_scope(user.role());
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let record = tx
            .audit_reads()
            .entry(id, scope)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::AuditEntry(id),
            })?;
        let with_trace = user.role().has(Permission::AuditView);
        Ok(AuditEntryView::of(record, with_trace))
    }
}
