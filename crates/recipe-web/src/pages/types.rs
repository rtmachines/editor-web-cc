//! The type pages: the type list, the UDT form, the type tree, and the type usage
//! (`api.md` section 2.3, design D6, D8, and D12).
//!
//! The module solves one problem: each area of the plant model owns its route rows and its tests,
//! so the agents of the machine pages, the unit pages, and the type pages change disjoint files.
//! This file also holds the few helpers that two pages of the area share.

use axum::http::Method;
use axum::routing::{get, post};
use fluent::FluentArgs;
use recipe_app::auth::Operation;
use recipe_domain::plant::{MachineCode, TypeName};

use super::path_values::query_values;
use super::{PageRequest, Texts};
use crate::error::WebError;
use crate::layout::NavTarget;
use crate::routes::{RouteEntry, operation_row};

mod define;
mod form;
mod form_rows;
mod list;
mod tree;
mod usage;

/// Return the route rows of the type pages.
///
/// The rows are `/machines/{code}/types`, `/machines/{code}/types/new`,
/// `/machines/{code}/types/{name}`, the usage page of a revision, and the commands to define a
/// struct and a new revision.
pub(crate) fn rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/machines/{code}/types",
            Operation::ListTypes,
            get(list::list),
        ),
        operation_row(
            Method::GET,
            "/machines/{code}/types/new",
            Operation::GetUdtForm,
            get(form::new_form),
        ),
        operation_row(
            Method::POST,
            "/machines/{code}/types",
            Operation::DefineUdt,
            post(define::define_new),
        ),
        operation_row(
            Method::POST,
            "/machines/{code}/types/{name}/revisions",
            Operation::DefineUdt,
            post(define::define_revision),
        ),
        operation_row(
            Method::GET,
            "/machines/{code}/types/{name}",
            Operation::GetTypeTree,
            get(tree::show),
        ),
        operation_row(
            Method::GET,
            "/machines/{code}/types/{name}/usage",
            Operation::GetTypeUsage,
            get(usage::show),
        ),
    ]
}

/// Return the link of the top bar that marks every page under `/machines` as the current page.
const fn current_nav() -> Option<NavTarget> {
    Some(NavTarget::Machines)
}

/// Return true when the role of the session may define a UDT type.
///
/// The pages show a link to the form only then. The use case of the form still checks the
/// permission (ADR-0013).
fn can_define(request: &PageRequest) -> Result<bool, WebError> {
    let role = request.auth()?.user().role();
    Ok(Operation::DefineUdt.requirement().allows(role))
}

/// Return the address of the page of a struct: `/machines/{code}/types/{name}`.
fn type_href(code: MachineCode, name: &TypeName) -> String {
    format!("/machines/{code}/types/{name}")
}

/// Return the address of one revision of a struct: `/machines/{code}/types/{name}?rev={revision}`.
fn revision_href(code: MachineCode, name: &TypeName, revision: u32) -> String {
    format!("/machines/{code}/types/{name}?rev={revision}")
}

/// Return the text of the message `key` with the arguments `name` and `revision`.
///
/// The revision goes into the text as digits, so the locale adds no thousands separator.
fn named_revision(texts: Texts<'_>, key: &str, name: &TypeName, revision: u32) -> String {
    let mut args = FluentArgs::new();
    args.set("name", name.to_string());
    args.set("revision", revision.to_string());
    texts.with(key, &args)
}

#[cfg(test)]
mod define_error_tests;
#[cfg(test)]
mod define_tests;
#[cfg(test)]
mod form_rows_tests;
#[cfg(test)]
mod form_tests;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
pub(crate) mod route_tests;
#[cfg(test)]
mod test_kit;
#[cfg(test)]
mod tree_tests;
#[cfg(test)]
mod usage_kit;
#[cfg(test)]
mod usage_tests;
