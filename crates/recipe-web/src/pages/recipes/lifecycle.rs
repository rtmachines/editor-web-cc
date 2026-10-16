//! `POST /recipes/{ref}/delete` and `POST /recipes/{ref}/restore` (`api.md` section 2.4,
//! ADR-0049 (g) and (h)).
//!
//! Both forms hold only the CSRF token. `DeleteRecipe` takes no `row_version`, and the page does
//! not copy its rule: the use case answers `409 E-RCP-010` or `409 E-RCP-026`. A success answers
//! `303`. A boosted form follows it, so the browser shows the list or the recipe page.

use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::Response;
use recipe_app::recipe::{DeleteRecipe, DeleteRecipeInput, RestoreRecipe, RestoreRecipeInput};

use super::path_reference;
use crate::error::WebError;
use crate::pages::render::see_other_path;
use crate::pages::request::PageRequest;
use crate::pages::users::TokenFields;
use crate::state::WebState;

/// `POST /recipes/{ref}/delete`: delete the recipe and discard its live draft.
pub(super) async fn delete(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<TokenFields>, FormRejection>,
) -> Result<Response, WebError> {
    let reference = path_reference(path)?;
    let Form(_fields) = form?;
    DeleteRecipe::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            DeleteRecipeInput { reference },
        )
        .await?;
    see_other_path("/recipes", request.request_id(), [])
}

/// `POST /recipes/{ref}/restore`: restore the deleted recipe.
pub(super) async fn restore(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<TokenFields>, FormRejection>,
) -> Result<Response, WebError> {
    let reference = path_reference(path)?;
    let Form(_fields) = form?;
    RestoreRecipe::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            RestoreRecipeInput {
                reference: reference.clone(),
            },
        )
        .await?;
    see_other_path(&format!("/recipes/{reference}"), request.request_id(), [])
}
