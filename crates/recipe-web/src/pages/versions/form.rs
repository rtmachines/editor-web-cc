//! `GET /recipes/{ref}/versions/new` and `POST /recipes/{ref}/versions` (`api.md` section 2.5,
//! design D6 and D7, ADR-0043 (h), ADR-0041).
//!
//! A valid request answers `303` to the new draft. A request with bad fields, or with a base that
//! names no version, gets `422 E-WEB-008` with one line per field. An HTMX request also gets the
//! form with the sent texts out of band. A domain error of `CreateVersion` gets the same form with
//! only its line, for example `E-RCP-011`. A failed snapshot check keeps its answer
//! `422 E-RCP-020` with one line per item.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::VersionFormPurpose;
use recipe_app::error::{AppError, Missing};
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::{
    CreateVersion, CreateVersionInput, GetVersionForm, GetVersionFormInput, VersionForm,
};
use recipe_domain::recipe::ReferenceNumber;
use recipe_domain::text::LongText;
use serde::Deserialize;

use crate::error::{FieldError, FieldReason, FormErrors, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::draft_reply::{refusal_lines, refused};
use crate::pages::recipes::fields::FieldLines;
use crate::pages::recipes::{Choice, path_reference, state_badge};
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The fields of the form of `POST /recipes/{ref}/versions` (design D6).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateFields {
    /// The number of the base version. An empty or missing value asks for an empty draft.
    #[serde(default)]
    based_on: String,
    change_note: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// Read `based_on`: empty for an empty draft, else a whole number from 1 with no zero in front.
fn base_number(text: &str) -> Result<Option<u32>, FieldReason> {
    if text.is_empty() {
        return Ok(None);
    }
    let canonical = text.bytes().all(|byte| byte.is_ascii_digit()) && !text.starts_with('0');
    text.parse::<u32>()
        .ok()
        .filter(|_number| canonical)
        .map(Some)
        .ok_or(FieldReason::BasedOn)
}

impl CreateFields {
    /// Turn the fields into the input of `CreateVersion`. Read every field (ADR-0041 (a)).
    ///
    /// # Errors
    ///
    /// Returns one line `E-WEB-008` for each field that is not valid, in the order of the form.
    fn check(
        &self,
        reference: &ReferenceNumber,
    ) -> Result<CreateVersionInput, NonEmpty<FieldError>> {
        let mut lines = FieldLines::default();
        let based_on = lines.field(
            base_number(&self.based_on),
            "based_on",
            FieldReason::BasedOn,
        );
        let change_note = lines.field(
            LongText::parse(&self.change_note),
            "change_note",
            FieldReason::ChangeNote,
        );
        let input = (|| {
            Some(CreateVersionInput {
                reference: reference.clone(),
                based_on: based_on?,
                change_note: change_note?,
            })
        })();
        lines.finish(input)
    }
}

/// The data of the form of a new version: the choices, the texts, and the error lines.
struct NewVersionForm {
    action: String,
    csrf_token: String,
    choices: Vec<Choice>,
    based_on: String,
    note: String,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    oob: bool,
    errors: FormErrors,
}

impl NewVersionForm {
    /// Build the empty form. The choices list each version, the newest first, and then the empty
    /// draft. The default base is selected (ADR-0043 (h)).
    fn new(t: Texts<'_>, form: &VersionForm, csrf_token: String) -> Self {
        let choices = form
            .choices
            .iter()
            .map(|choice| {
                let (_kind, key) = state_badge(choice.state);
                Choice {
                    value: choice.number.to_string(),
                    text: format!("{} ({})", choice.number, t.get(key)),
                }
            })
            .chain(std::iter::once(Choice::new(
                "",
                t.get("version-empty-draft"),
            )))
            .collect();
        Self {
            action: format!("/recipes/{}/versions", form.reference),
            csrf_token,
            choices,
            based_on: form
                .default_based_on
                .map(|number| number.to_string())
                .unwrap_or_default(),
            note: String::new(),
            oob: false,
            errors: FormErrors::default(),
        }
    }

    /// Return the form with the sent texts and the error lines, swapped out of band.
    fn sent(self, fields: &CreateFields, errors: FormErrors) -> Self {
        Self {
            based_on: fields.based_on.clone(),
            note: fields.change_note.clone(),
            oob: true,
            errors,
            ..self
        }
    }
}

/// The page `version-new.html`.
#[derive(Template)]
#[template(path = "version-new.html")]
struct NewVersionPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    recipe_href: String,
    recipe_text: String,
    form: NewVersionForm,
}

/// The block `form` of `version-new.html` alone: the form that an error answer swaps out of band.
#[derive(Template)]
#[template(path = "version-new.html", block = "form")]
struct NewVersionFormBlock<'a> {
    t: Texts<'a>,
    form: NewVersionForm,
}

/// Run `GetVersionForm` with the purpose `CreateVersion`.
///
/// The query checks the permission of `CreateVersion` first and refuses a deleted recipe. The
/// create route runs it before it reads its fields (ADR-0046 (a)).
async fn version_form(
    state: &WebState,
    request: &PageRequest,
    reference: &ReferenceNumber,
) -> Result<VersionForm, WebError> {
    let input = GetVersionFormInput {
        purpose: VersionFormPurpose::CreateVersion,
        reference: reference.clone(),
    };
    Ok(GetVersionForm::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?)
}

/// `GET /recipes/{ref}/versions/new`: show the form of a new version.
pub(crate) async fn new_form(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let reference = path_reference(path)?;
    let form = version_form(&state, &request, &reference).await?;
    let t = Texts::new(state.bundles(), request.lang());
    let title = t.with_arg("version-new-title", "reference", &reference);
    let layout = Layout::for_session(t, &request, title, Some(NavTarget::Recipes))?;
    let page = NewVersionPage {
        form: NewVersionForm::new(t, &form, layout.csrf_token.clone()),
        recipe_href: format!("/recipes/{reference}"),
        recipe_text: t.with_arg("recipe-title", "reference", &reference),
        layout,
        t,
    };
    Ok(render(&page, request.request_id())?.into_response())
}

/// Build the error of a form with bad fields: every line and the form with the sent texts.
async fn refuse(
    state: &WebState,
    request: &PageRequest,
    (reference, fields): (&ReferenceNumber, &CreateFields),
    lines: NonEmpty<FieldError>,
) -> WebError {
    let form = match version_form(state, request, reference).await {
        Ok(form) => form,
        Err(error) => return error,
    };
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| NewVersionFormBlock {
        t,
        form: NewVersionForm::new(t, &form, request.csrf_token()).sent(fields, errors),
    })
}

/// Return true when the error says that `based_on` names no version of the recipe.
///
/// `CreateVersion` answers `NotFound` for it. `based_on` is a request field, so the page shows the
/// line `E-WEB-008` of the field (ADR-0043 (h), `api.md` section 2.5).
fn is_missing_base(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound {
            entity: Missing::Version { .. }
        }
    )
}

/// `POST /recipes/{ref}/versions`: create a draft from a base version and go to the draft.
pub(crate) async fn create(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<CreateFields>, FormRejection>,
) -> Result<Response, WebError> {
    let reference = path_reference(path)?;
    // The form query checks the permission of `CreateVersion` before the fields are read (ADR-0046).
    version_form(&state, &request, &reference).await?;
    let Form(fields) = form?;
    let target = (&reference, &fields);
    let input = match fields.check(&reference) {
        Ok(input) => input,
        Err(lines) => return Err(refuse(&state, &request, target, lines).await),
    };
    let result = CreateVersion::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    let created = match result {
        Ok(created) => created,
        Err(error) if is_missing_base(&error) => {
            let line = FieldError::bad_request("based_on", FieldReason::BasedOn);
            return Err(refuse(&state, &request, target, NonEmpty::new(line)).await);
        }
        // No other error of `CreateVersion` names an input, so its line stands in the summary.
        Err(error) => {
            return Err(match refusal_lines(error, |_refusal| None) {
                Ok(lines) => refuse(&state, &request, target, lines).await,
                Err(other) => other,
            });
        }
    };
    see_other_path(
        &format!("/recipes/{reference}/versions/{}", created.number),
        request.request_id(),
        [],
    )
}
