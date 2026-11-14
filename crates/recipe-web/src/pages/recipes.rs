//! The recipe pages: the list, the form, the recipe page, and the rename (`api.md` section 2.4,
//! design D7 and D12, ADR-0043).
//!
//! The module solves one problem: each area of the recipe editor owns its route rows and its
//! tests. The module also holds the parts that the recipe pages and the version pages share. These
//! are the state badge and the path value `{ref}`.

use axum::extract::Path;
use axum::extract::rejection::PathRejection;
use axum::http::Method;
use axum::routing::{get, post};
use recipe_app::auth::Operation;
use recipe_app::error::{AppError, Missing};
use recipe_domain::recipe::{ReferenceNumber, VersionState};

use super::path_values::reference;
use crate::error::WebError;
use crate::routes::{RouteEntry, operation_row};

mod form;
mod lifecycle;
mod list;
pub(crate) mod page;
mod rename;

/// Return the route rows of the recipe pages.
///
/// The rows are `/recipes`, `/recipes/new`, `/recipes/{ref}`, and the commands to create, rename,
/// delete, and restore a recipe.
pub(crate) fn rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/recipes",
            Operation::ListRecipes,
            get(list::list),
        ),
        operation_row(
            Method::GET,
            "/recipes/new",
            Operation::GetRecipeForm,
            get(form::new_form),
        ),
        operation_row(
            Method::POST,
            "/recipes",
            Operation::CreateRecipe,
            post(form::create),
        ),
        operation_row(
            Method::GET,
            "/recipes/{ref}",
            Operation::GetRecipe,
            get(page::recipe),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/rename",
            Operation::RenameRecipe,
            post(rename::rename),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/delete",
            Operation::DeleteRecipe,
            post(lifecycle::delete),
        ),
        operation_row(
            Method::POST,
            "/recipes/{ref}/restore",
            Operation::RestoreRecipe,
            post(lifecycle::restore),
        ),
    ]
}

/// Return the kind of the badge of a version state and the key of its word (`design-system.md` 9).
///
/// A discarded draft is read-only like a retired version, so it shows the tokens and the icon of
/// `Retired` with the word of `status-discarded`.
pub(crate) const fn state_badge(state: VersionState) -> (&'static str, &'static str) {
    match state {
        VersionState::LiveDraft => ("draft", "status-draft"),
        VersionState::Discarded => ("retired", "status-discarded"),
        VersionState::Approved => ("approved", "status-approved"),
        VersionState::Retired => ("retired", "status-retired"),
    }
}

/// Build the `404 E-WEB-005` error of a path value that is malformed.
const fn malformed(entity: Missing) -> WebError {
    WebError::App(AppError::NotFound { entity })
}

/// Read the route `/recipes/{ref}`.
///
/// # Errors
///
/// Returns `404 E-WEB-005` with `Missing::MalformedRecipe` when the rejection or the text is bad.
pub(crate) fn path_reference(
    path: Result<Path<String>, PathRejection>,
) -> Result<ReferenceNumber, WebError> {
    let Path(text) = path.map_err(|_rejection| malformed(Missing::MalformedRecipe))?;
    reference(&text)
}

#[cfg(test)]
mod form_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod page_tests;
#[cfg(test)]
mod post_error_tests;
#[cfg(test)]
pub(crate) mod route_tests;
#[cfg(test)]
pub(crate) mod test_world;
#[cfg(test)]
mod tests;
