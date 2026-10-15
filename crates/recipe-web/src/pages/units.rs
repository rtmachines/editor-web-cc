//! The unit pages: the unit tree, the unit form, the unit page, and the commands of a unit
//! (`api.md` section 2.2, design D6, D7, and D12).
//!
//! The module solves one problem: each area of the plant model owns its route rows and its tests,
//! so the agents of the machine pages, the unit pages, and the type pages change disjoint files.

use axum::http::Method;
use axum::routing::{get, post};
use recipe_app::auth::Operation;

use super::path_values::query_values;
use crate::routes::{RouteEntry, operation_row};

mod add;
mod commands;
mod form;
mod page;

/// Return the route rows of the unit pages.
///
/// The rows are `/machines/{code}/units/new`, `/machines/{code}/units/{path}`, and the commands to
/// add, rename, and archive a unit. The row `GET /machines/{code}/units` belongs to the machine
/// pages. A later task of the change adds the other rows.
pub(crate) fn rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/machines/{code}/units/new",
            Operation::GetUnitForm,
            get(form::new_form),
        ),
        operation_row(
            Method::POST,
            "/machines/{code}/units",
            Operation::AddUnit,
            post(add::add),
        ),
        operation_row(
            Method::GET,
            "/machines/{code}/units/{path}",
            Operation::GetUnit,
            get(page::show),
        ),
        operation_row(
            Method::POST,
            "/machines/{code}/units/{path}/rename",
            Operation::RenameUnit,
            post(commands::rename),
        ),
        operation_row(
            Method::POST,
            "/machines/{code}/units/{path}/archive",
            Operation::ArchiveUnit,
            post(commands::archive),
        ),
    ]
}

#[cfg(test)]
mod add_tests;
#[cfg(test)]
mod commands_tests;
#[cfg(test)]
mod form_tests;
#[cfg(test)]
mod page_tests;
#[cfg(test)]
pub(crate) mod route_tests;
#[cfg(test)]
mod test_world;
