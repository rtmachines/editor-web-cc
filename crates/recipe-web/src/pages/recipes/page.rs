//! `GET /recipes/{ref}`: the recipe page with its version table (`api.md` section 2.4, design D7).
//!
//! The page shows the name, the machine, and one page of versions. A live recipe shows the rename
//! form to a role with `RecipeEdit` and the link New version to a role with `VersionCreate`. A
//! deleted recipe shows the state Deleted and no form (ADR-0043 (e)).

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::Operation;
use recipe_app::recipe::{GetRecipe, GetRecipeInput, RecipeView};
use recipe_domain::recipe::ReferenceNumber;
use serde::Deserialize;

use super::path_reference;
use super::rename::RenameForm;
use crate::error::{FormErrors, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::{Page, page as read_page, query_values};
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::pages::versions::table::VersionTable;
use crate::state::WebState;

/// The query string of the recipe page and of the version list. A key other than `page` gives
/// `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PageQuery {
    page: Option<String>,
}

impl PageQuery {
    /// Read the page of the version table.
    ///
    /// # Errors
    ///
    /// Returns `422 E-WEB-008` when the page is not a whole number from 1.
    pub(crate) fn page(query: Result<Query<Self>, QueryRejection>) -> Result<Page, WebError> {
        read_page(query_values(query)?.page.as_deref())
    }
}

/// The page `recipe.html`.
///
/// The type solves one problem: one template shows a live recipe with its forms and a deleted
/// recipe without them. The flags are true only for a live recipe and a role that holds the
/// permission of the command (`Operation::requirement`).
#[derive(Template)]
#[template(path = "recipe.html")]
struct RecipePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    name: String,
    machine: String,
    machine_href: String,
    deleted: bool,
    form: RenameForm,
    new_version_href: Option<String>,
    versions: VersionTable,
}

impl<'a> RecipePage<'a> {
    /// Build the page from the recipe, its machine, and one page of versions.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(
        t: Texts<'a>,
        request: &PageRequest,
        view: &RecipeView,
        page: Page,
    ) -> Result<Self, WebError> {
        let role = request.auth()?.user().role();
        let recipe = &view.recipe;
        let reference = recipe.reference();
        let live = !recipe.is_deleted();
        let allows = |operation: Operation| live && operation.requirement().allows(role);
        let title = t.with_arg("recipe-title", "reference", reference);
        let layout = Layout::for_session(t, request, title, Some(NavTarget::Recipes))?;
        let machine = &view.machine;
        let machine_href = if machine.is_live() {
            format!("/machines/{}", machine.code())
        } else {
            format!("/machines/by-id/{}", machine.id())
        };
        Ok(Self {
            t,
            name: recipe.name().to_string(),
            machine: machine.code().to_string(),
            machine_href,
            deleted: !live,
            form: RenameForm::new(
                reference,
                recipe.name().to_string(),
                layout.csrf_token.clone(),
                FormErrors::default(),
            )
            .shown(allows(Operation::RenameRecipe)),
            new_version_href: allows(Operation::CreateVersion)
                .then(|| format!("/recipes/{reference}/versions/new")),
            versions: VersionTable::new(t, reference, &view.versions, page),
            layout,
        })
    }
}

/// Run `GetRecipe` for the page `page` of the version table and render the recipe page.
///
/// The version list `GET /recipes/{ref}/versions` without `HX-Request` answers this page too.
///
/// # Errors
///
/// Returns the error of `GetRecipe`, for example `404 E-WEB-005` or `409 E-RCP-012`.
pub(crate) async fn show(
    state: &WebState,
    request: &PageRequest,
    reference: ReferenceNumber,
    page: Page,
) -> Result<Response, WebError> {
    let input = GetRecipeInput {
        reference,
        page: page.index,
    };
    let view = GetRecipe::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let recipe_page = RecipePage::new(t, request, &view, page)?;
    Ok(render(&recipe_page, request.request_id())?.into_response())
}

/// `GET /recipes/{ref}`: show the recipe with one page of its versions.
pub(crate) async fn recipe(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<PageQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let reference = path_reference(path)?;
    let page = PageQuery::page(query)?;
    show(&state, &request, reference, page).await
}
