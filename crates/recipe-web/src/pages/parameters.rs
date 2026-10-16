//! The parameter pages: the parameter table, the parameter form, the parameter page, and the value,
//! limit, and translation routes (`api.md` section 2.6, design D7 and D12).
//!
//! The module solves one problem: each area of the recipe editor owns its route rows and its tests.
//! So the agents of the recipe pages, the version pages, and the parameter pages change disjoint
//! files. Wave 5 adds the rows of the parameter routes.

use axum::extract::Path;
use axum::extract::rejection::PathRejection;
use axum::http::Method;
use axum::routing::{get, post};
use recipe_app::auth::Operation;
use recipe_app::error::{AppError, Missing};

use super::path_values::{parameter_key, reference, version_number};
use crate::error::WebError;
use crate::routes::{RouteEntry, operation_row};
use reply::Target;

mod definition;
mod edit;
pub(crate) mod fields;
mod form_view;
pub(crate) mod grid;
mod new;
mod page;
mod prepare;
mod remove;
mod reply;
mod summary;
mod table;
mod translation_routes;
mod translations;
mod value;

/// Read the route `.../parameters/{key}`. The reference number comes first, then `{n}` and `{key}`.
///
/// A rejection of axum, for example a value that is not UTF-8, holds no value. So it gives
/// `Missing::MalformedRecipe`, as a malformed reference number does.
///
/// # Errors
///
/// Returns `404 E-WEB-005` for a bad reference number, version number, key, or a rejection.
fn path_parameter(
    path: Result<Path<(String, String, String)>, PathRejection>,
) -> Result<Target, WebError> {
    let Path((text, number, key)) = path.map_err(|_rejection| {
        WebError::App(AppError::NotFound {
            entity: Missing::MalformedRecipe,
        })
    })?;
    Ok(Target {
        reference: reference(&text)?,
        number: version_number(&number)?,
        key: Some(parameter_key(&key)?),
    })
}

#[cfg(test)]
mod fields_tests;
#[cfg(test)]
mod grid_tests;
#[cfg(test)]
mod nested_group_tests;
#[cfg(test)]
pub(crate) mod tree_kit;

/// Return the route rows of the parameter pages.
///
/// The rows are the table, the form of a new parameter, `AddParameter`, the parameter page, the
/// save of the edit form, the value, the limits, the translations, and the removal (`api.md`
/// section 2.6, ADR-0040).
pub(crate) fn rows() -> Vec<RouteEntry> {
    let mut rows = new_rows();
    rows.extend(parameter_rows());
    rows.extend(translation_rows());
    rows
}

/// Return the rows of the translations and of the removal of one parameter (ADR-0040).
fn translation_rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/recipes/{ref}/versions/{n}/parameters/{key}/translations",
            Operation::GetTranslations,
            get(translation_routes::translations),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}",
            Operation::UpsertTranslation,
            post(translation_routes::upsert),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}/delete",
            Operation::RemoveTranslation,
            post(translation_routes::remove),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/parameters/{key}/delete",
            Operation::RemoveParameter,
            post(remove::remove),
        ),
    ]
}

/// Return the rows of the table, the form of a new parameter, and `AddParameter`.
fn new_rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/recipes/{ref}/versions/{n}/parameters",
            Operation::ListParameters,
            get(table::table),
        ),
        operation_row(
            Method::GET,
            "/recipes/{ref}/versions/{n}/parameters/new",
            Operation::GetParameterForm,
            get(new::new_form),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/parameters",
            Operation::AddParameter,
            post(new::add),
        ),
    ]
}

/// Return the rows of the page, the edit form, the value, and the limits of one parameter.
fn parameter_rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/recipes/{ref}/versions/{n}/parameters/{key}",
            Operation::GetParameter,
            get(page::parameter),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/parameters/{key}/definition",
            Operation::UpdateParameterDefinition,
            post(edit::save),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/parameters/{key}/value",
            Operation::SetParameterValue,
            post(value::set_value),
        ),
        operation_row(
            Method::GET,
            "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
            Operation::GetLimits,
            get(value::limits),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
            Operation::ReplaceLimits,
            post(value::replace_limits),
        ),
    ]
}

#[cfg(test)]
mod add_error_tests;
#[cfg(test)]
mod add_tests;
#[cfg(test)]
mod conflict_tests;
#[cfg(test)]
mod deleted_tests;
#[cfg(test)]
mod edit_tests;
#[cfg(test)]
mod new_tests;
#[cfg(test)]
mod page_tests;
#[cfg(test)]
mod remove_tests;
#[cfg(test)]
pub(crate) mod route_tests;
#[cfg(test)]
mod test_kit;
#[cfg(test)]
mod translation_tests;
#[cfg(test)]
mod value_tests;
