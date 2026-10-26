//! Tests of the audit coverage of every use case (`data-model.md` sections 6 and 10).
//!
//! Each use case runs its success path once. The audit log then holds the actions of its row
//! of section 6, in order. Each entry names the table and the aggregate root of section 10.
//! Each entry also names the acting user, the request, and a state.

mod identity;
mod plant;
mod recipe;

use recipe_domain::audit::{AggregateType, AuditAction, AuditEntry};

use crate::auth::{CurrentUser, RequestMeta};
use crate::memory::fixtures::Fixture;

/// One expected audit entry: the action, the table of the changed row, and the aggregate root.
type Expected = (AuditAction, &'static str, AggregateType);

/// Return the entries of the log after the first `skip` entries.
///
/// A test that writes after a seeding command, such as an approval, skips the entries before it.
pub(super) fn entries_after(fixture: &Fixture, skip: usize) -> Vec<AuditEntry> {
    fixture
        .store
        .audit_entries()
        .into_iter()
        .skip(skip)
        .collect()
}

/// Assert the actions of `entries` in order, and the actor, the request, and the state of each.
pub(super) fn assert_trail(
    entries: &[AuditEntry],
    user: &CurrentUser,
    request: &RequestMeta,
    name: &str,
    expected: &[Expected],
) {
    let actual: Vec<Expected> = entries
        .iter()
        .map(|entry| {
            (
                entry.action(),
                entry.target().entity_type(),
                entry.target().aggregate_type(),
            )
        })
        .collect();
    assert_eq!(actual, expected, "{name}: audit actions");
    for entry in entries {
        assert_eq!(entry.actor(), Some(user.user()), "{name}: actor");
        assert_eq!(
            entry.request_id(),
            request.request_id(),
            "{name}: request id"
        );
        assert!(
            entry.before().is_some() || entry.after().is_some(),
            "{name}: state"
        );
    }
}
