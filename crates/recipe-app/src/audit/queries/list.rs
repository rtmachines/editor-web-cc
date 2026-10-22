//! The query `ListAuditEntries` (spec "Audit list query", ADR-0051 (b) to (e)).

use recipe_domain::audit::AggregateType;

use super::scope::{admits_filter, audit_scope};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::AppError;
use crate::ports::{
    AggregateId, AuditEntryId, AuditFilter, AuditReads, AuditRow, CursorPage, EntityLabel,
    Isolation,
};

/// The parsed request of `ListAuditEntries`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditListInput {
    /// The filters of the list. Each `None` keeps every entry.
    pub filter: AuditFilter,
    /// The cursor: the list keeps the entries with a smaller id. `None` starts at the newest entry.
    pub before: Option<AuditEntryId>,
}

/// The name of the aggregate of the filter `aggregate` (ADR-0051 (d)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregateLabel {
    /// The aggregate type of the newest row of the page.
    pub aggregate_type: AggregateType,
    /// The label of the current row of the root. `Missing` when the root does not exist.
    pub label: EntityLabel,
}

/// The result of `ListAuditEntries`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditList {
    /// One page of the entries, newest first, without states.
    pub page: CursorPage<AuditRow>,
    /// The name of the aggregate of the filter `aggregate`.
    ///
    /// It is `None` without the filter and for an empty page. The page then shows the short id.
    pub aggregate: Option<AggregateLabel>,
}

/// Lists the audit entries in the scope of the role, newest first, one cursor page at a time.
///
/// The query solves one problem: the audit log grows without an end, so the list reads one page
/// below the cursor. A role with only `AuditViewRecipes` sees only the recipe entries (A20).
#[derive(Clone)]
pub struct ListAuditEntries {
    ctx: AppContext,
}

impl UseCase for ListAuditEntries {
    const OPERATION: Operation = Operation::ListAuditEntries;
}

impl ListAuditEntries {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return one page of the entries that match the filters. A row holds no state.
    ///
    /// A filter value outside the scope of the role gives the empty page without a read.
    /// With the filter `aggregate`, a page with rows also gives the label of the aggregate.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: AuditListInput,
    ) -> Result<AuditList, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let scope = audit_scope(user.role());
        if !admits_filter(scope, &input.filter) {
            return Ok(AuditList {
                page: CursorPage::empty(),
                aggregate: None,
            });
        }
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let size = self.ctx.settings.pages.audit;
        let reads = tx.audit_reads();
        let page = reads
            .entries(&input.filter, scope, input.before, size)
            .await?;
        let aggregate = match (input.filter.aggregate, page.rows.first()) {
            (Some(id), Some(newest)) => {
                Some(aggregate_label(reads, newest.aggregate_type, id).await?)
            }
            _ => None,
        };
        Ok(AuditList { page, aggregate })
    }
}

/// Read the label of the root `id` of an aggregate of `aggregate_type` (ADR-0051 (d)).
async fn aggregate_label(
    reads: &mut dyn AuditReads,
    aggregate_type: AggregateType,
    id: AggregateId,
) -> Result<AggregateLabel, AppError> {
    let label = reads.aggregate_label(aggregate_type, id).await?;
    Ok(AggregateLabel {
        aggregate_type,
        label,
    })
}
