//! `POST /machines/{code}/units`: add a unit (`api.md` section 2.2, design D6, D7, and D9).
//!
//! A valid HTMX request gets the whole tree with the new unit as the focus unit. The answer also
//! updates `#toast`, `#error-panel`, and `#unit-form` out of band. A request without HTMX gets
//! `303` to the machine page. A rejected HTMX request puts its error into `#error-panel`. A request
//! with bad fields also gets the form with the sent texts out of band (ADR-0041 (e)). A domain
//! error of `AddUnit` gets the same form with only its line, for example `E-PLT-004` under the
//! segment.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::UnitFormPurpose;
use recipe_app::error::AppError;
use recipe_app::plant::{
    AddUnit, AddUnitInput, GetUnitForm, GetUnitTree, UnitForm, UnitFormInput, UnitTreeInput,
};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::{MachineCode, PlantError, UnitPath};
use recipe_domain::text::Text;
use serde::Deserialize;

use super::form::{FormView, Place, UnitFormFragment};
use crate::error::{FieldError, FieldReason, WebError};
use crate::pages::draft_reply::{refusal_lines, refused};
use crate::pages::machines::tree::{Scope, UnitTree};
use crate::pages::path_values::path_code;
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The `id` of the parent list of the form.
const PARENT_INPUT: &str = "parent_path";
/// The `id` of the segment input of the form.
const SEGMENT_INPUT: &str = "segment";
/// The `id` of the name input of the form.
const NAME_INPUT: &str = "unit_name";

/// The fields of the form of `POST /machines/{code}/units`.
///
/// The names are the `name` attributes of the form (design D5). `parent_path` is empty for a main
/// unit. The handler reads `segment` with the parent, and the domain reads it again.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AddFields {
    parent_path: String,
    segment: String,
    name: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

impl AddFields {
    /// Read the parent: `None` for a main unit.
    ///
    /// # Errors
    ///
    /// Returns the line of the error of `UnitPath` under the parent list, for example `E-PLT-017`.
    fn parent(&self) -> Result<Option<UnitPath>, FieldError> {
        if self.parent_path.is_empty() {
            return Ok(None);
        }
        UnitPath::parse(&self.parent_path)
            .map(Some)
            .map_err(|error| FieldError::plant(&error, Some(PARENT_INPUT.to_owned())))
    }

    /// Read the segment as `AddUnit` reads it: one segment, and at most 5 levels with `parent`.
    ///
    /// A bad parent has its own line, so the caller then passes `None` and the segment counts
    /// alone.
    ///
    /// # Errors
    ///
    /// Returns the line of the error of `UnitPath` under the segment input: `E-PLT-017` with the
    /// joined text, or `E-PLT-002`.
    fn segment(&self, parent: Option<&UnitPath>) -> Result<(), FieldError> {
        let joined = match parent {
            Some(parent) => format!("{parent}.{}", self.segment),
            None => format!("U{}", self.segment),
        };
        let single =
            UnitPath::parse(&format!("U{}", self.segment)).is_ok_and(|path| path.depth() == 1);
        let checked = if single {
            UnitPath::parse(&joined).map(|_path| ())
        } else {
            Err(PlantError::InvalidUnitPath { text: joined })
        };
        checked.map_err(|error| FieldError::plant(&error, Some(SEGMENT_INPUT.to_owned())))
    }

    /// Turn the fields into the input of `AddUnit`. Read every field (ADR-0041 (a)).
    ///
    /// # Errors
    ///
    /// Returns one line for each bad field in the order of the form: the errors of `UnitPath` for
    /// the parent and the segment, and `E-WEB-008` for the name.
    fn check(&self, code: MachineCode) -> Result<AddUnitInput, NonEmpty<FieldError>> {
        let mut lines = Vec::new();
        let parent_path = self.parent().map_err(|line| lines.push(line)).ok();
        let segment = self
            .segment(parent_path.as_ref().and_then(Option::as_ref))
            .map_err(|line| lines.push(line))
            .ok();
        let name = Text::parse(&self.name)
            .map_err(|_reason| lines.push(FieldError::bad_request(NAME_INPUT, FieldReason::Name)))
            .ok();
        match (NonEmpty::try_from(lines), parent_path, segment, name) {
            (Ok(lines), ..) => Err(lines),
            (Err(_none), Some(parent_path), Some(()), Some(name)) => Ok(AddUnitInput {
                code,
                parent_path,
                segment: self.segment.clone(),
                name,
            }),
            // A failed field always adds its line, so this arm only keeps the match total.
            (Err(_none), ..) => Err(NonEmpty::new(FieldError::bad_request(
                "form",
                FieldReason::Form,
            ))),
        }
    }
}

/// Return the input that a refused `AddUnit` names: the parent list or the segment.
fn add_input(error: &AppError) -> Option<String> {
    let AppError::Plant(error) = error else {
        return None;
    };
    let id = match error {
        PlantError::ParentNotFound { .. } | PlantError::UnitArchived { .. } => PARENT_INPUT,
        PlantError::DuplicateUnitPath { .. }
        | PlantError::InvalidUnitPath { .. }
        | PlantError::UnitDepthExceeded { .. } => SEGMENT_INPUT,
        _other => return None,
    };
    Some(id.to_owned())
}

/// Run `GetUnitForm` for a main unit with `purpose`. The query checks that permission first.
///
/// The form of an error answer shows the sent parent and segment, so it needs only the parents.
/// Each write route runs it before its field check, so a role without the right gets
/// `403 E-WEB-002` and no form (ADR-0046 (a)).
pub(super) async fn unit_form(
    state: &WebState,
    request: &PageRequest,
    (code, purpose): (MachineCode, UnitFormPurpose),
) -> Result<UnitForm, WebError> {
    let input = UnitFormInput {
        purpose,
        code,
        parent: None,
    };
    Ok(GetUnitForm::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?)
}

/// Build the error of a form with bad fields: every line and the form with the sent texts.
///
/// The error of `GetUnitForm` wins, for example `403 E-WEB-002` for a role without the right.
async fn refuse(
    state: &WebState,
    request: &PageRequest,
    (code, fields): (MachineCode, &AddFields),
    lines: NonEmpty<FieldError>,
) -> WebError {
    let form = match unit_form(state, request, (code, UnitFormPurpose::AddUnit)).await {
        Ok(form) => form,
        Err(error) => return error,
    };
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| UnitFormFragment {
        t,
        form: FormView {
            selected: fields.parent_path.clone(),
            segment: Some(fields.segment.clone()),
            name: fields.name.clone(),
            errors,
            ..FormView::new(t, &form, request.csrf_token(), Place::Refused)
        },
    })
}

/// The answer `unit-added.html`: the tree, and three out-of-band updates.
#[derive(Template)]
#[template(path = "unit-added.html")]
struct UnitAddedFragment<'a> {
    t: Texts<'a>,
    tree: UnitTree,
    form: FormView,
    message: String,
}

/// `POST /machines/{code}/units`: add the unit and show the tree with the new unit in focus.
pub(super) async fn add(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<AddFields>, FormRejection>,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    // The form query checks the permission of `AddUnit` before the fields are read (ADR-0046).
    unit_form(&state, &request, (code, UnitFormPurpose::AddUnit)).await?;
    let Form(fields) = form?;
    let input = match fields.check(code) {
        Ok(input) => input,
        Err(lines) => return Err(refuse(&state, &request, (code, &fields), lines).await),
    };
    let parent = input.parent_path.clone();
    let result = AddUnit::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    let added = match result {
        Ok(added) => added,
        Err(error) => {
            return Err(match refusal_lines(error, add_input) {
                Ok(lines) => refuse(&state, &request, (code, &fields), lines).await,
                Err(other) => other,
            });
        }
    };
    if !request.is_htmx() {
        return see_other_path(&format!("/machines/{code}"), request.request_id(), []);
    }
    tree_answer(&state, &request, (code, parent), &added.path).await
}

/// Answer a valid HTMX request: the tree with the new unit `path` in focus and the renewed form.
async fn tree_answer(
    state: &WebState,
    request: &PageRequest,
    (code, parent): (MachineCode, Option<UnitPath>),
    path: &UnitPath,
) -> Result<Response, WebError> {
    let app = state.identity().app().clone();
    let auth = request.auth()?;
    let meta = request.meta();
    let machine = GetUnitTree::new(app.clone())
        .execute(auth.user(), &meta, UnitTreeInput { code })
        .await?;
    let next = GetUnitForm::new(app)
        .execute(
            auth.user(),
            &meta,
            UnitFormInput {
                purpose: UnitFormPurpose::AddUnit,
                code,
                parent,
            },
        )
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let tree = UnitTree::build(&machine, Scope::Live, auth.user().role(), Some(path), true);
    let fragment = UnitAddedFragment {
        t,
        tree,
        form: FormView::new(t, &next, request.csrf_token(), Place::Renewed),
        message: t.with_arg("unit-added", "path", path),
    };
    Ok(render(&fragment, request.request_id())?.into_response())
}
