//! `POST /machines/{code}/units`: add a unit (`api.md` section 2.2, design D6 and D7).
//!
//! A valid HTMX request gets the whole tree with the new unit as the focus unit. The answer also
//! updates `#toast`, `#error-panel`, and `#unit-form` out of band. A request without HTMX gets
//! `303` to the machine page. A rejected HTMX request puts its error into `#error-panel`.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::plant::{
    AddUnit, AddUnitInput, GetUnitForm, GetUnitTree, UnitFormInput, UnitTreeInput,
};
use recipe_domain::plant::MachineCode;
use recipe_domain::text::Text;
use serde::Deserialize;

use super::form::{FormView, Place};
use crate::error::{FieldReason, WebError};
use crate::pages::machines::tree::{Scope, UnitTree};
use crate::pages::path_values::{path_code, unit_path};
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The fields of the form of `POST /machines/{code}/units`.
///
/// The names are the `name` attributes of the form (design D5). `parent_path` is empty for a main
/// unit. The domain parses `segment`.
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
    /// Turn the fields into the input of `AddUnit`, in the order of the form.
    ///
    /// # Errors
    ///
    /// Returns the error of `UnitPath` for a bad parent and `422 E-WEB-008` for a bad name.
    fn check(self, code: MachineCode) -> Result<AddUnitInput, WebError> {
        let parent_path = if self.parent_path.is_empty() {
            None
        } else {
            Some(unit_path(&self.parent_path)?)
        };
        let name = Text::parse(&self.name).map_err(|_reason| WebError::BadRequest {
            field: "unit_name",
            reason: FieldReason::Name,
        })?;
        Ok(AddUnitInput {
            code,
            parent_path,
            segment: self.segment,
            name,
        })
    }
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
    let Form(fields) = form?;
    let input = fields.check(code)?;
    let parent = input.parent_path.clone();
    let app = state.identity().app().clone();
    let auth = request.auth()?;
    let meta = request.meta();
    let added = AddUnit::new(app.clone())
        .execute(auth.user(), &meta, input)
        .await?;
    if !request.is_htmx() {
        return see_other_path(&format!("/machines/{code}"), request.request_id(), []);
    }
    let machine = GetUnitTree::new(app.clone())
        .execute(auth.user(), &meta, UnitTreeInput { code })
        .await?;
    let next = GetUnitForm::new(app)
        .execute(auth.user(), &meta, UnitFormInput { code, parent })
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let tree = UnitTree::build(
        &machine,
        Scope::Live,
        auth.user().role(),
        Some(&added.path),
        true,
    );
    let fragment = UnitAddedFragment {
        t,
        tree,
        form: FormView::new(t, &next, request.csrf_token(), Place::Renewed),
        message: t.with_arg("unit-added", "path", &added.path),
    };
    Ok(render(&fragment, request.request_id())?.into_response())
}
