//! The version pages: the version list, the version form, the version page, and the rebind form
//! (`api.md` section 2.5, design D7 and D12).
//!
//! The module solves one problem: each area of the recipe editor owns its route rows and its tests.
//! So the agents of the recipe pages, the version pages, and the parameter pages change disjoint
//! files. Wave 4 adds the version list, the version form, and the version page. Wave 5 adds the
//! rebind.

use axum::extract::Path;
use axum::extract::rejection::PathRejection;
use axum::http::Method;
use axum::routing::{get, post};
use recipe_app::auth::Operation;
use recipe_app::error::{AppError, Missing};
use recipe_domain::recipe::ReferenceNumber;

use super::path_values::{reference, version_number};
use crate::error::WebError;
use crate::routes::{RouteEntry, operation_row};

mod actions;
mod form;
mod list;
mod outdated;
pub(crate) mod page;
pub(crate) mod parameter_table;
mod rebind;
mod rebind_form;
pub(crate) mod table;

/// Read the route `/recipes/{ref}/versions/{n}`. The reference number comes first.
///
/// A rejection of axum, for example a value that is not UTF-8, holds no value. So it gives
/// `Missing::MalformedRecipe`, as a malformed reference number does.
///
/// # Errors
///
/// Returns `404 E-WEB-005` for a bad reference number, a bad version number, or a rejection.
pub(crate) fn path_version(
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<(ReferenceNumber, u32), WebError> {
    let Path((text, number)) = path.map_err(|_rejection| {
        WebError::App(AppError::NotFound {
            entity: Missing::MalformedRecipe,
        })
    })?;
    Ok((reference(&text)?, version_number(&number)?))
}

/// Return the route rows of the version pages.
///
/// The rows are `/recipes/{ref}/versions`, `/recipes/{ref}/versions/new`, the version page, the
/// command that creates a version, the rebind, and the rows of [`lifecycle_rows`] (`api.md`
/// section 2.5).
pub(crate) fn rows() -> Vec<RouteEntry> {
    let mut rows = vec![
        operation_row(
            Method::GET,
            "/recipes/{ref}/versions",
            Operation::ListVersions,
            get(list::list),
        ),
        operation_row(
            Method::GET,
            "/recipes/{ref}/versions/new",
            Operation::GetVersionForm,
            get(form::new_form),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions",
            Operation::CreateVersion,
            post(form::create),
        ),
        operation_row(
            Method::GET,
            "/recipes/{ref}/versions/{n}",
            Operation::GetVersion,
            get(page::version),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/rebind",
            Operation::RebindTypeRevision,
            post(rebind::rebind),
        ),
    ];
    rows.extend(lifecycle_rows());
    rows
}

/// Return the route rows of the approval preview, the approval, the retire, and the discard
/// (ADR-0049).
fn lifecycle_rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/recipes/{ref}/versions/{n}/approve",
            Operation::GetApprovalPreview,
            get(super::approval::preview::preview),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/approve",
            Operation::ApproveVersion,
            post(super::approval::approve::approve),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/retire",
            Operation::RetireVersion,
            post(actions::retire),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/discard",
            Operation::DiscardDraft,
            post(actions::discard),
        ),
    ]
}

#[cfg(test)]
mod actions_tests;
#[cfg(test)]
mod form_tests;
#[cfg(test)]
mod large_tests;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod outdated_tests;
#[cfg(test)]
mod page_tests;
#[cfg(test)]
mod parameter_table_tests;
#[cfg(test)]
mod rebind_error_tests;
#[cfg(test)]
mod rebind_form_tests;
#[cfg(test)]
mod rebind_tests;
#[cfg(test)]
mod rebind_world;
#[cfg(test)]
pub(crate) mod route_tests;
