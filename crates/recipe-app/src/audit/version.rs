//! The audit entry of a version row (design D9).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::recipe::{RecipeId, RecipeVersionId};
use serde_json::Value;

use crate::auth::{CurrentUser, RequestMeta};
use crate::context::audit_context;
use crate::ports::RowImage;

/// Build the update entry of a version row.
///
/// The function solves one problem: `ApproveVersion`, `RetireVersion`, and `DiscardDraft` write
/// the same update entry of a version row. Each caller decides how it wraps the entry.
pub(crate) fn version_entry(
    action: RowAction,
    (recipe, version): (RecipeId, RecipeVersionId),
    (before, after): (RowImage, Value),
    (user, meta): (&CurrentUser, &RequestMeta),
) -> AuditEntry {
    AuditEntry::row(
        action,
        AuditTarget::recipe_version(recipe, version),
        RowChange::Update {
            before: before.into_value(),
            after,
        },
        audit_context(user, meta),
    )
}
