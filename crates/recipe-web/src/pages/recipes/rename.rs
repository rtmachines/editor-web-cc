//! `POST /recipes/{ref}/rename` (`api.md` section 2.4, design D7, ADR-0041 (e) and (h)).
//!
//! A valid HTMX rename gets the new name for `#recipe-name`, the toast, an empty `#error-panel`,
//! and the clean form out of band. A rename without HTMX gets `303` to the recipe page. A bad name
//! gets `422 E-WEB-008` with the form and the line under its input. A domain error of
//! `RenameRecipe` gets the form with only its line. The form query `GetRecipeForm` checks the
//! permission before the field check (ADR-0013).

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::RecipeFormPurpose;
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::{RenameRecipe, RenameRecipeInput};
use recipe_domain::recipe::ReferenceNumber;
use recipe_domain::text::Text;
use serde::Deserialize;

use super::fields::FieldLines;
use super::form::recipe_form;
use super::path_reference;
use crate::error::{FieldError, FieldReason, FormErrors, WebError};
use crate::pages::draft_reply::{refusal_lines, refused, with_form};
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The fields of the form of `POST /recipes/{ref}/rename`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NameFields {
    name: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// The data of the rename form. The recipe page and each answer of the rename show it.
pub(crate) struct RenameForm {
    /// The address of the route, for example `/recipes/R-100/rename`.
    pub(crate) action: String,
    /// The name in the input: the current name, the saved name, or the sent text.
    pub(crate) name: String,
    /// The CSRF token of the session.
    pub(crate) csrf_token: String,
    /// True when the page or the answer shows the form. The recipe page sets it by permission.
    pub(crate) shown: bool,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    pub(crate) oob: bool,
    /// The error lines under the input.
    pub(crate) errors: FormErrors,
}

impl RenameForm {
    /// Build the shown form of the page, which an answer does not swap.
    pub(crate) fn new(
        reference: &ReferenceNumber,
        name: String,
        csrf_token: String,
        errors: FormErrors,
    ) -> Self {
        Self {
            action: format!("/recipes/{reference}/rename"),
            name,
            csrf_token,
            shown: true,
            oob: false,
            errors,
        }
    }

    /// Return the form with `shown` set by the permission of the role.
    #[must_use]
    pub(crate) fn shown(self, shown: bool) -> Self {
        Self { shown, ..self }
    }

    /// Return the form that an answer swaps out of band.
    fn out_of_band(self) -> Self {
        Self { oob: true, ..self }
    }
}

/// The block `rename_form` of `recipe.html` alone: the form that an answer swaps out of band.
#[derive(Template)]
#[template(path = "recipe.html", block = "rename_form")]
struct RenameFormBlock<'a> {
    t: Texts<'a>,
    form: RenameForm,
}

/// The answer `recipe-renamed.html`: the new name and two out-of-band updates.
#[derive(Template)]
#[template(path = "recipe-renamed.html")]
struct RecipeRenamedFragment {
    name: String,
    message: String,
}

/// Build the error with the line of the name and the form with the sent text.
fn refused_name(
    state: &WebState,
    request: &PageRequest,
    reference: &ReferenceNumber,
    (sent, lines): (String, NonEmpty<FieldError>),
) -> WebError {
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| RenameFormBlock {
        t,
        form: RenameForm::new(reference, sent, request.csrf_token(), errors).out_of_band(),
    })
}

/// Answer a valid HTMX rename: the name, the toast, the empty error panel, and the clean form.
fn renamed(
    state: &WebState,
    request: &PageRequest,
    reference: &ReferenceNumber,
    name: &Text,
) -> Result<Response, WebError> {
    let t = Texts::new(state.bundles(), request.lang());
    let fragment = RecipeRenamedFragment {
        name: name.to_string(),
        message: t.with_arg("recipe-renamed", "reference", reference),
    };
    let form = RenameFormBlock {
        t,
        form: RenameForm::new(
            reference,
            name.to_string(),
            request.csrf_token(),
            FormErrors::default(),
        )
        .out_of_band(),
    };
    let body = render(&fragment, request.request_id())?;
    Ok(with_form(body, &form, request.request_id())?.into_response())
}

/// `POST /recipes/{ref}/rename`: rename the live recipe.
pub(crate) async fn rename(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<NameFields>, FormRejection>,
) -> Result<Response, WebError> {
    let reference = path_reference(path)?;
    // The form query checks `RecipeEdit` before the field check (ADR-0046 (a)).
    recipe_form(&state, &request, RecipeFormPurpose::RenameRecipe).await?;
    let Form(fields) = form?;
    let mut lines = FieldLines::default();
    let parsed = lines.field(Text::parse(&fields.name), "recipe_name", FieldReason::Name);
    let name = match lines.finish(parsed) {
        Ok(name) => name,
        Err(lines) => {
            return Err(refused_name(
                &state,
                &request,
                &reference,
                (fields.name, lines),
            ));
        }
    };
    let input = RenameRecipeInput {
        reference: reference.clone(),
        name: name.clone(),
    };
    let result = RenameRecipe::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    if let Err(error) = result {
        // No error of `RenameRecipe` names the name input, so its line stands in the summary.
        return Err(match refusal_lines(error, |_refusal| None) {
            Ok(lines) => refused_name(&state, &request, &reference, (fields.name, lines)),
            Err(other) => other,
        });
    }
    if !request.is_htmx() {
        return see_other_path(&format!("/recipes/{reference}"), request.request_id(), []);
    }
    renamed(&state, &request, &reference, &name)
}
