//! `GET /machines/new` and `POST /machines`: the form and the creation of a machine
//! (`api.md` section 2.2, design D5 and D6).
//!
//! A rejected HTMX request puts its error into `#error-panel` and leaves the form as it is, so the
//! user keeps the typed values. A valid request answers `303` to the page of the new machine.

use askama::Template;
use axum::extract::rejection::FormRejection;
use axum::extract::{Form, State};
use axum::response::{IntoResponse, Response};
use recipe_app::plant::{CreateMachine, CreateMachineInput, GetMachineForm, MachineForm};
use recipe_domain::plant::{MachineCode, PlcPlatform};
use recipe_domain::text::Text;
use serde::Deserialize;

use super::platform_text;
use crate::error::{FieldReason, WebError};
use crate::layout::{Layout, NavTarget};
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

/// Build the error of a field that is not valid.
const fn invalid(field: &'static str, reason: FieldReason) -> WebError {
    WebError::BadRequest { field, reason }
}

/// Read a name field. `field` is the `id` of its input.
fn name_field(text: &str, field: &'static str) -> Result<Text, WebError> {
    Text::parse(text).map_err(|_reason| invalid(field, FieldReason::Name))
}

impl CreateFields {
    /// Turn the four fields into the input of `CreateMachine`, in the order of the form.
    ///
    /// # Errors
    ///
    /// Returns `422 E-WEB-008` for the first field that is not valid.
    fn check(&self) -> Result<CreateMachineInput, WebError> {
        let code = MachineCode::parse(&self.code)
            .map_err(|_reason| invalid("code", FieldReason::MachineCode))?;
        let name = name_field(&self.name, "machine_name")?;
        let platform = PlcPlatform::parse(&self.plc_platform)
            .map_err(|_reason| invalid("plc_platform", FieldReason::Platform))?;
        let main_unit_name = name_field(&self.main_unit_name, "main_unit_name")?;
        Ok(CreateMachineInput {
            code,
            name,
            platform,
            main_unit_name,
        })
    }
}

/// One option of the list of platforms.
struct PlatformOption {
    value: &'static str,
    text: String,
}

/// The page `machine-new.html`.
#[derive(Template)]
#[template(path = "machine-new.html")]
struct NewMachinePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    platforms: Vec<PlatformOption>,
}

impl<'a> NewMachinePage<'a> {
    /// Build the page from the platforms that the query offers.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(t: Texts<'a>, request: &PageRequest, form: &MachineForm) -> Result<Self, WebError> {
        Ok(Self {
            layout: Layout::for_session(
                t,
                request,
                t.get("machine-new-title"),
                Some(NavTarget::Machines),
            )?,
            t,
            // The empty first option is selected, so the user chooses the platform (D20, U4).
            // The platform never changes later, so the browser must not pick one by default.
            platforms: std::iter::once(PlatformOption {
                value: "",
                text: t.get("field-platform-choose"),
            })
            .chain(form.platforms.iter().map(|platform| PlatformOption {
                value: platform.as_str(),
                text: platform_text(t, *platform),
            }))
            .collect(),
        })
    }
}

/// `GET /machines/new`: show the form of a new machine. The query checks the permission first.
pub(crate) async fn new_form(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let form = GetMachineForm::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta())
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let page = NewMachinePage::new(t, &request, &form)?;
    Ok(render(&page, request.request_id())?.into_response())
}

/// `POST /machines`: create a machine with its main unit and go to the page of the machine.
pub(crate) async fn create(
    State(state): State<WebState>,
    request: PageRequest,
    form: Result<Form<CreateFields>, FormRejection>,
) -> Result<Response, WebError> {
    let Form(fields) = form?;
    let input = fields.check()?;
    let code = input.code;
    CreateMachine::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    see_other_path(&format!("/machines/{code}"), request.request_id(), [])
}
