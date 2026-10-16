//! `GET /recipes/new` and `POST /recipes`: the form and the creation of a recipe
//! (`api.md` section 2.4, design D6 and D7, ADR-0041).
//!
//! A valid request answers `303` to version 1 of the new recipe. A request with bad fields gets
//! `422 E-WEB-008` with one line per field. An HTMX request also gets the form with the sent texts
//! out of band. A domain error of the use case gets the same form with only its line, for example
//! `404 E-PLT-019` under the machine list.

use askama::Template;
use axum::extract::rejection::FormRejection;
use axum::extract::{Form, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::RecipeFormPurpose;
use recipe_app::error::AppError;
use recipe_app::recipe::{
    CreateRecipe, CreateRecipeInput, GetRecipeForm, RecipeForm, RecipeFormInput,
};
use recipe_domain::plant::{MachineCode, PlantError};
use recipe_domain::recipe::{RecipeError, ReferenceNumber};
use recipe_domain::text::{LongText, Text};
use serde::Deserialize;

use super::Choice;
use super::fields::FieldLines;
use crate::error::{FieldError, FieldReason, FormErrors, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::draft_reply::{refusal_lines, refused};
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;
use recipe_app::ports::NonEmpty;

/// The fields of the form of `POST /recipes` (design D6).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateFields {
    machine_code: String,
    reference_number: String,
    name: String,
    change_note: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

impl CreateFields {
    /// Turn the four fields into the input of `CreateRecipe`. Read every field (ADR-0041 (a)).
    ///
    /// # Errors
    ///
    /// Returns one line `E-WEB-008` for each field that is not valid, in the order of the form.
    fn check(&self) -> Result<CreateRecipeInput, NonEmpty<FieldError>> {
        let mut lines = FieldLines::default();
        let machine_code = lines.field(
            MachineCode::parse(&self.machine_code),
            "machine_code",
            FieldReason::MachineCode,
        );
        let reference = lines.field(
            ReferenceNumber::parse(&self.reference_number),
            "reference_number",
            FieldReason::ReferenceNumber,
        );
        let name = lines.field(Text::parse(&self.name), "recipe_name", FieldReason::Name);
        let change_note = lines.field(
            LongText::parse(&self.change_note),
            "change_note",
            FieldReason::ChangeNote,
        );
        let input = (|| {
            Some(CreateRecipeInput {
                machine_code: machine_code?,
                reference: reference?,
                name: name?,
                change_note: change_note?,
            })
        })();
        lines.finish(input)
    }
}

/// The data of the form of a new recipe: the choices, the texts, and the error lines.
struct NewRecipeForm {
    csrf_token: String,
    machines: Vec<Choice>,
    machine: String,
    reference: String,
    name: String,
    note: String,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    oob: bool,
    errors: FormErrors,
}

impl NewRecipeForm {
    /// Build an empty form over the live machines. The empty first option is selected, so the
    /// user chooses the machine (design D7). Its text is `recipe-machine-choose`.
    fn new(t: Texts<'_>, form: &RecipeForm, csrf_token: String) -> Self {
        let machines = std::iter::once(Choice::new("", t.get("recipe-machine-choose")))
            .chain(form.machines.iter().map(|machine| Choice {
                value: machine.code.to_string(),
                text: format!("{} {}", machine.code, machine.name),
            }))
            .collect();
        Self {
            csrf_token,
            machines,
            machine: String::new(),
            reference: String::new(),
            name: String::new(),
            note: String::new(),
            oob: false,
            errors: FormErrors::default(),
        }
    }

    /// Return the form with the sent texts and the error lines, swapped out of band.
    fn sent(self, fields: &CreateFields, errors: FormErrors) -> Self {
        Self {
            machine: fields.machine_code.clone(),
            reference: fields.reference_number.clone(),
            name: fields.name.clone(),
            note: fields.change_note.clone(),
            oob: true,
            errors,
            ..self
        }
    }
}

/// The page `recipe-new.html`.
#[derive(Template)]
#[template(path = "recipe-new.html")]
struct NewRecipePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    form: NewRecipeForm,
}

/// The block `form` of `recipe-new.html` alone: the form that an error answer swaps out of band.
#[derive(Template)]
#[template(path = "recipe-new.html", block = "form")]
struct NewRecipeFormBlock<'a> {
    t: Texts<'a>,
    form: NewRecipeForm,
}

/// Return the input that a refused `CreateRecipe` names: the machine list or the reference number.
fn create_input(error: &AppError) -> Option<String> {
    let id = match error {
        AppError::Plant(
            PlantError::MachineNotFound { .. } | PlantError::MachineArchived { .. },
        ) => "machine_code",
        AppError::Recipe(RecipeError::DuplicateReference { .. }) => "reference_number",
        _other => return None,
    };
    Some(id.to_owned())
}

/// Run `GetRecipeForm` with `purpose`. The query checks the permission of that command first.
///
/// `CreateRecipe` needs `RecipeCreate`, and `RenameRecipe` needs `RecipeEdit`. Each write route
/// runs it before it reads its fields (ADR-0046 (a)).
pub(super) async fn recipe_form(
    state: &WebState,
    request: &PageRequest,
    purpose: RecipeFormPurpose,
) -> Result<RecipeForm, WebError> {
    Ok(GetRecipeForm::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            RecipeFormInput { purpose },
        )
        .await?)
}

/// `GET /recipes/new`: show the form of a new recipe.
pub(crate) async fn new_form(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let form = recipe_form(&state, &request, RecipeFormPurpose::CreateRecipe).await?;
    let t = Texts::new(state.bundles(), request.lang());
    let layout = Layout::for_session(
        t,
        &request,
        t.get("recipe-new-title"),
        Some(NavTarget::Recipes),
    )?;
    let page = NewRecipePage {
        form: NewRecipeForm::new(t, &form, layout.csrf_token.clone()),
        layout,
        t,
    };
    Ok(render(&page, request.request_id())?.into_response())
}

/// Build the error of a form with bad fields: every line and the form with the sent texts.
///
/// # Errors
///
/// Returns the error of `GetRecipeForm` when the role cannot open the form.
async fn refuse(
    state: &WebState,
    request: &PageRequest,
    fields: &CreateFields,
    lines: NonEmpty<FieldError>,
) -> WebError {
    let form = match recipe_form(state, request, RecipeFormPurpose::CreateRecipe).await {
        Ok(form) => form,
        Err(error) => return error,
    };
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| NewRecipeFormBlock {
        t,
        form: NewRecipeForm::new(t, &form, request.csrf_token()).sent(fields, errors),
    })
}

/// `POST /recipes`: create a recipe with an empty version 1 and go to that version.
pub(crate) async fn create(
    State(state): State<WebState>,
    request: PageRequest,
    form: Result<Form<CreateFields>, FormRejection>,
) -> Result<Response, WebError> {
    // The form query checks the permission of `CreateRecipe` before the fields are read (ADR-0046).
    recipe_form(&state, &request, RecipeFormPurpose::CreateRecipe).await?;
    let Form(fields) = form?;
    let input = match fields.check() {
        Ok(input) => input,
        Err(lines) => return Err(refuse(&state, &request, &fields, lines).await),
    };
    let reference = input.reference.clone();
    let result = CreateRecipe::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    if let Err(error) = result {
        return Err(match refusal_lines(error, create_input) {
            Ok(lines) => refuse(&state, &request, &fields, lines).await,
            Err(other) => other,
        });
    }
    see_other_path(
        &format!("/recipes/{reference}/versions/1"),
        request.request_id(),
        [],
    )
}
