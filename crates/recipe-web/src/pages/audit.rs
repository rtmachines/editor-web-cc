//! The audit pages: the list, the entry, and the security events (`api.md` section 2.9, ADR-0051).
//!
//! The module solves one problem: the history page, the audit list, and the entry page show the
//! same rows and read the same filter values. `rows` holds the row view, `filter` the parsers, and
//! `choices` the selects of the filter form.

use axum::http::Method;
use axum::routing::get;
use recipe_app::auth::Operation;

use crate::routes::{RouteEntry, operation_row};

mod choices;
mod entry;
#[cfg(test)]
mod entry_tests;
pub(crate) mod filter;
mod list;
#[cfg(test)]
mod list_tests;
pub(crate) mod rows;
#[cfg(test)]
pub(crate) mod test_world;

/// Return the route rows of the audit pages.
pub(crate) fn rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/audit",
            Operation::ListAuditEntries,
            get(list::list),
        ),
        operation_row(
            Method::GET,
            "/audit/{id}",
            Operation::GetAuditEntry,
            get(entry::entry),
        ),
    ]
}
