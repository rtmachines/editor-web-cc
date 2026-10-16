//! `GET /machines/new` and `POST /machines`: the form and the creation of a machine
//! (`api.md` section 2.2, design D5 and D6, ADR-0041).
//!
//! A valid request answers `303` to the page of the new machine. A request with bad fields gets
//! `422 E-WEB-008` with one line per field. An HTMX request also gets the form with the sent texts
//! out of band, with a line under each bad input. A domain error of the use case gets the same
//! form with only its own line, for example `E-PLT-001` under the code.

use askama::Template;
use axum::extract::rejection::FormRejection;
use axum::extract::{Form, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::MachineFormPurpose;
use recipe_app::error::AppError;
use recipe_app::plant::{
    CreateMachine, CreateMachineInput, GetMachineForm, MachineForm, MachineFormInput,
};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::{MachineCode, PlantError, PlcPlatform};
use recipe_domain::text::Text;
use serde::Deserialize;

use super::platform_text;
use crate::error::{FieldError, FieldReason, FormErrors, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::draft_reply::{refusal_lines, refused};
use crate::pages::recipes::fields::FieldLines;
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The fields of the form of `POST /machines`.
///
/// The names are the `name` attributes of the form (design D5). The error of a name field uses
/// the `id` of its input, for example `machine_name`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateFields {
    code: String,
    name: String,
    plc_platform: String,
    main_unit_name: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

impl CreateFields {
    /// Turn the four fields into the input of `CreateMachine`. Read every field (ADR-0041 (a)).
    ///
    /// # Errors
    ///
    /// Returns one line `E-WEB-008` for each field that is not valid, in the order of the form.
    fn check(&self) -> Result<CreateMachineInput, NonEmpty<FieldError>> {
        let mut lines = FieldLines::default();
        let code = lines.field(
            MachineCode::parse(&self.code),
            "code",
            FieldReason::MachineCode,
        );
        let name = lines.field(Text::parse(&self.name), "machine_name", FieldReason::Name);
        let platform = lines.field(
            PlcPlatform::parse(&self.plc_platform),
            "plc_platform",
            FieldReason::Platform,
        );
        let main_unit_name = lines.field(
            Text::parse(&self.main_unit_name),
            "main_unit_name",
            FieldReason::Name,
        );
        let input = (|| {
            Some(CreateMachineInput {
                code: code?,
                name: name?,
                platform: platform?,
                main_unit_name: main_unit_name?,
            })
        })();
        lines.finish(input)
    }
}

/// One option of the list of platforms.
struct PlatformOption {
    value: &'static str,
    text: String,
}

/// The data of the form of a new machine: the platforms, the texts, and the error lines.
struct NewMachineForm {
    csrf_token: String,
    platforms: Vec<PlatformOption>,
    code: String,
    name: String,
    platform: String,
    main_unit: String,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    oob: bool,
    errors: FormErrors,
}

impl NewMachineForm {
    /// Build an empty form from the platforms that the query offers.
    fn new(t: Texts<'_>, form: &MachineForm, csrf_token: String) -> Self {
        // The empty first option is selected, so the user chooses the platform (D20, U4).
        // The platform never changes later, so the browser must not pick one by default.
        let platforms = std::iter::once(PlatformOption {
            value: "",
            text: t.get("field-platform-choose"),
        })
        .chain(form.platforms.iter().map(|platform| PlatformOption {
            value: platform.as_str(),
            text: platform_text(t, *platform),
        }))
        .collect();
        Self {
            csrf_token,
            platforms,
            code: String::new(),
            name: String::new(),
            platform: String::new(),
            main_unit: String::new(),
            oob: false,
            errors: FormErrors::default(),
        }
    }

    /// Return the form with the sent texts and the error lines, swapped out of band.
    fn sent(self, fields: &CreateFields, errors: FormErrors) -> Self {
        Self {
            code: fields.code.clone(),
            name: fields.name.clone(),
            platform: fields.plc_platform.clone(),
            main_unit: fields.main_unit_name.clone(),
            oob: true,
            errors,
            ..self
        }
    }
}

/// The page `machine-new.html`.
#[derive(Template)]
#[template(path = "machine-new.html")]
struct NewMachinePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    form: NewMachineForm,
}

/// The block `form` of `machine-new.html` alone: the form that an error answer swaps out of band.
#[derive(Template)]
#[template(path = "machine-new.html", block = "form")]
struct NewMachineFormBlock<'a> {
    t: Texts<'a>,
    form: NewMachineForm,
}

/// Run `GetMachineForm` with `purpose`. The query checks the permission of that command first.
///
/// Each write route runs it before its field check, so a role without the right gets
/// `403 E-WEB-002` and no form (ADR-0046 (a)).
pub(super) async fn machine_form(
    state: &WebState,
    request: &PageRequest,
    purpose: MachineFormPurpose,
) -> Result<MachineForm, WebError> {
    Ok(GetMachineForm::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            MachineFormInput { purpose },
        )
        .await?)
}

/// `GET /machines/new`: show the form of a new machine. The query checks the permission first.
pub(crate) async fn new_form(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let form = machine_form(&state, &request, MachineFormPurpose::CreateMachine).await?;
    let t = Texts::new(state.bundles(), request.lang());
    let layout = Layout::for_session(
        t,
        &request,
        t.get("machine-new-title"),
        Some(NavTarget::Machines),
    )?;
    let page = NewMachinePage {
        form: NewMachineForm::new(t, &form, layout.csrf_token.clone()),
        layout,
        t,
    };
    Ok(render(&page, request.request_id())?.into_response())
}

/// Build the error of a form with bad fields: every line and the form with the sent texts.
///
/// The error of `GetMachineForm` wins, for example `403 E-WEB-002` for a role without the right.
async fn refuse(
    state: &WebState,
    request: &PageRequest,
    fields: &CreateFields,
    lines: NonEmpty<FieldError>,
) -> WebError {
    let form = match machine_form(state, request, MachineFormPurpose::CreateMachine).await {
        Ok(form) => form,
        Err(error) => return error,
    };
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| NewMachineFormBlock {
        t,
        form: NewMachineForm::new(t, &form, request.csrf_token()).sent(fields, errors),
    })
}

/// `POST /machines`: create a machine with its main unit and go to the page of the machine.
pub(crate) async fn create(
    State(state): State<WebState>,
    request: PageRequest,
    form: Result<Form<CreateFields>, FormRejection>,
) -> Result<Response, WebError> {
    // The form query checks the permission of `CreateMachine` before the fields are read (ADR-0046).
    machine_form(&state, &request, MachineFormPurpose::CreateMachine).await?;
    let Form(fields) = form?;
    let input = match fields.check() {
        Ok(input) => input,
        Err(lines) => return Err(refuse(&state, &request, &fields, lines).await),
    };
    let code = input.code;
    let created = CreateMachine::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    if let Err(error) = created {
        return Err(match refusal_lines(error, code_input) {
            Ok(lines) => refuse(&state, &request, &fields, lines).await,
            Err(other) => other,
        });
    }
    see_other_path(&format!("/machines/{code}"), request.request_id(), [])
}

/// Return the input that a refused `CreateMachine` names: the code of `E-PLT-001`.
fn code_input(error: &AppError) -> Option<String> {
    matches!(
        error,
        AppError::Plant(PlantError::DuplicateMachineCode { .. })
    )
    .then(|| "code".to_owned())
}
