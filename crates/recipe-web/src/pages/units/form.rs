//! `GET /machines/{code}/units/new`: the form of a new unit (`api.md` section 2.2, design D6).
//!
//! An HTMX request with `HX-Target: unit-form` gets the fragment `unit-form.html`. Any other
//! request gets the page `unit-form-page.html`: the full page, or the content of `main` for an
//! HTMX request with `HX-Target: content` (ADR-0037 w). The answers of `POST /machines/{code}/units`
//! reuse [`FormView`] for the new empty form and for the form of an error answer.

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::{Operation, UnitFormPurpose};
use recipe_app::plant::{GetUnitForm, UnitForm, UnitFormInput};
use serde::Deserialize;

use super::query_values;
use crate::error::{FormErrors, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::{path_code, unit_path};
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The value of `HX-Target` that asks for the fragment: the `id` of the region of the form.
const FORM_REGION: &str = "unit-form";

/// The query string of `GET /machines/{code}/units/new`.
///
/// A key other than `parent_path` gives `422 E-WEB-008`. So the query can never hold the CSRF
/// token (ADR-0037 h). The form query checks the permission first, so a role without
/// `UnitManage` gets `403` (ADR-0046 (a)).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NewQuery {
    parent_path: Option<String>,
}

/// Where a form stands, so one template serves the fragment, the page, and the renewed form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Place {
    /// The region `#unit-form` after the link "Add a sub-unit". The form is boosted and takes the focus.
    Fragment,
    /// The region `#unit-form` after an add. The form is boosted. The focus stays on the new unit.
    Renewed,
    /// The page of the form. It has no `#unit-tree`, so the form is plain. `focus` is false for
    /// the content answer, where the heading takes the focus.
    Page {
        /// True when the first input gets `autofocus`.
        focus: bool,
    },
    /// The form of an error answer, which swaps out of band (ADR-0041 (e)). The form is boosted.
    /// It has no heading and no `autofocus`, because the focus stays on the submit button.
    Refused,
}

/// What `unit-form.html` shows.
///
/// The type solves one problem: the fragment, the page, and the answer of an add show the same
/// form. One view gives them the same fields, so they agree on the parent and the
/// proposed segment. The struct that includes the template has the fields `t` and `form`.
pub(super) struct FormView {
    /// True when the form has the literal `hx-*` attributes of design D6.
    pub(super) boosted: bool,
    /// True when the first input gets `autofocus`.
    pub(super) autofocus: bool,
    /// The address of `POST /machines/{code}/units`.
    pub(super) action: String,
    /// The text of the `h2`: the new unit under the parent, or a new main unit.
    pub(super) heading: String,
    /// The parent path, or an empty text for a main unit. It is the value of the hidden field
    /// `parent_path` and the text of the line that names the parent.
    pub(super) selected: String,
    /// The text of the segment input: the proposed segment or the sent text. `None` when all 64
    /// are in use.
    pub(super) segment: Option<String>,
    /// The text of the name input. It is empty, except in the form of an error answer.
    pub(super) name: String,
    /// The CSRF token of the session.
    pub(super) csrf_token: String,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    pub(super) oob: bool,
    /// The error lines under the inputs.
    pub(super) errors: FormErrors,
    /// True when read-only mode blocks the form, so the notice stands in its place (ADR-0113 (d)).
    pub(super) blocked: bool,
}

impl FormView {
    /// Build the view from the result of `GetUnitForm`.
    pub(super) fn new(t: Texts<'_>, form: &UnitForm, csrf_token: String, place: Place) -> Self {
        let heading = form.parent.as_ref().map_or_else(
            || t.get("unit-new-main"),
            |parent| t.with_arg("unit-new-under", "path", parent),
        );
        let (boosted, autofocus) = match place {
            Place::Fragment => (true, true),
            Place::Renewed | Place::Refused => (true, false),
            Place::Page { focus } => (false, focus),
        };
        Self {
            boosted,
            autofocus,
            action: format!("/machines/{}/units", form.machine.code()),
            heading,
            selected: form
                .parent
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            segment: form.segment.map(|segment| segment.to_string()),
            name: String::new(),
            csrf_token,
            oob: place == Place::Refused,
            errors: FormErrors::default(),
            blocked: false,
        }
    }

    /// Put the notice of read-only mode in place of the form when `blocked` is true.
    pub(super) fn blocked(self, blocked: bool) -> Self {
        Self { blocked, ..self }
    }
}

/// The fragment `unit-form.html`: the form and nothing around it.
///
/// The form of an error answer (`Place::Refused`) has no heading, so the fragment starts with the
/// form, as the out-of-band swap requires (design D9).
#[derive(Template)]
#[template(path = "unit-form.html")]
pub(super) struct UnitFormFragment<'a> {
    pub(super) t: Texts<'a>,
    pub(super) form: FormView,
}

/// The page `unit-form-page.html`: the layout, the heading, the form, and a link to the machine.
#[derive(Template)]
#[template(path = "unit-form-page.html")]
struct UnitFormPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    form: FormView,
    code: String,
    machine_href: String,
}

/// `GET /machines/{code}/units/new`: show the form of a new unit under the optional parent.
pub(super) async fn new_form(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<NewQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let wanted = query_values(query)
        .and_then(|values| values.parent_path.as_deref().map(unit_path).transpose());
    // The form query checks the permission of `AddUnit` before a bad query value gives its answer
    // (ADR-0046 (a)). A bad query value runs the query without a parent.
    let parent = wanted.as_ref().ok().and_then(Option::as_ref).cloned();
    let form = GetUnitForm::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            UnitFormInput {
                purpose: UnitFormPurpose::AddUnit,
                code,
                parent,
            },
        )
        .await?;
    wanted?;
    let t = Texts::new(state.bundles(), request.lang());
    let csrf = request.csrf_token();
    let blocked = !request.may_run(Operation::AddUnit)?;
    if request.is_htmx() && request.hx_target() == Some(FORM_REGION) {
        let fragment = UnitFormFragment {
            t,
            form: FormView::new(t, &form, csrf, Place::Fragment).blocked(blocked),
        };
        return Ok(render(&fragment, request.request_id())?.into_response());
    }
    let title = t.with_arg("unit-new-title", "code", code);
    let layout = Layout::for_session(t, &request, title, Some(NavTarget::Machines))?;
    let place = Place::Page {
        focus: !layout.is_content(),
    };
    let page = UnitFormPage {
        layout,
        t,
        form: FormView::new(t, &form, csrf, place).blocked(blocked),
        code: code.to_string(),
        machine_href: format!("/machines/{code}"),
    };
    Ok(render(&page, request.request_id())?.into_response())
}
