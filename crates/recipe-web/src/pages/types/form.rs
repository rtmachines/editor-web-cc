//! The UDT form: `GET /machines/{code}/types/new` and the answer of `POST` with `add=1`
//! (`api.md` section 2.3, design D6 and D8).

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::plant::{GetUdtForm, UdtForm, UdtFormInput};
use recipe_domain::plant::limits::MAX_MEMBERS;
use serde::Deserialize;

use super::form_rows::SentRow;
use super::{current_nav, query_values};
use crate::error::WebError;
use crate::layout::Layout;
use crate::pages::path_values::{path_code, type_name};
use crate::pages::render::render;
use crate::pages::{PageRequest, Texts};
use crate::state::WebState;

/// The query of `GET /machines/{code}/types/new`.
///
/// The struct accepts only `from`. A request with `csrf_token` or a member row in the query gives
/// `422 E-WEB-008`, so no URL holds the token or the rows of a form (ADR-0037 h).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FormQuery {
    /// The struct whose latest revision the form copies.
    from: Option<String>,
}

/// What the form shows besides the machine, the origin, and the proposals.
pub(super) struct Shown {
    /// The text of the name input. A form of a new revision has no name input.
    pub(super) type_name: String,
    /// The member rows in the order of the form.
    pub(super) rows: Vec<SentRow>,
    /// True when the last row is a new empty row that takes the focus.
    pub(super) focus_last: bool,
}

/// One member row of the page.
struct MemberRow {
    /// The index of the row, from 0, as in `members[<index>].name`.
    index: usize,
    /// The number of the row in the labels, from 1.
    number: usize,
    name: String,
    type_text: String,
    /// True when the name input has `autofocus`.
    focus: bool,
}

/// The page `udt-form.html`.
#[derive(Template)]
#[template(path = "udt-form.html")]
struct UdtFormPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    title: String,
    action: String,
    /// True for the form of a new revision. It has no name input.
    revision: bool,
    type_name: String,
    rows: Vec<MemberRow>,
    choices: Vec<String>,
    /// True when the form holds `type.max_members` rows and cannot add one.
    at_max: bool,
    /// True when a row takes the focus, so the heading keeps the plain form.
    row_focus: bool,
    list_href: String,
    list_text: String,
}

/// Number the rows. The row that takes the focus is the last one.
fn member_rows(rows: Vec<SentRow>, focus_last: bool) -> Vec<MemberRow> {
    let last = rows.len().saturating_sub(1);
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| MemberRow {
            index,
            number: index.saturating_add(1),
            name: row.name,
            type_text: row.type_text,
            focus: focus_last && index == last,
        })
        .collect()
}

/// Answer with the UDT form `form` and the rows `shown`.
///
/// The function solves one problem: `GET .../types/new` and `POST` with `add=1` show the same
/// form, so one function builds both answers. The form of a new revision (`form.from`) posts to
/// `.../revisions`. The form of a new struct posts to `/machines/{code}/types`.
pub(super) fn respond(
    state: &WebState,
    request: &PageRequest,
    form: UdtForm,
    shown: Shown,
) -> Result<Response, WebError> {
    let t = Texts::new(state.bundles(), request.lang());
    let code = form.machine.code();
    let (title, action) = match &form.from {
        Some((name, _revision)) => (
            t.with_arg("udt-revision-title", "name", name),
            format!("/machines/{code}/types/{name}/revisions"),
        ),
        None => (t.get("udt-new-title"), format!("/machines/{code}/types")),
    };
    let layout = Layout::for_session(t, request, title.clone(), current_nav())?;
    let at_max = shown.rows.len() >= MAX_MEMBERS;
    let page = UdtFormPage {
        layout,
        t,
        title,
        action,
        revision: form.from.is_some(),
        type_name: shown.type_name,
        rows: member_rows(shown.rows, shown.focus_last),
        choices: form.choices,
        at_max,
        row_focus: shown.focus_last,
        list_href: format!("/machines/{code}/types"),
        list_text: t.with_arg("types-title", "code", code),
    };
    Ok(render(&page, request.request_id())?.into_response())
}

/// Return the rows of a form that the user opens: the members of the copied revision, or one
/// empty row for a new struct.
fn opening_rows(form: &UdtForm) -> Vec<SentRow> {
    let members = form.members.iter().map(|(name, type_text)| SentRow {
        name: name.to_string(),
        type_text: type_text.clone(),
    });
    let empty = form.from.is_none().then(|| SentRow {
        name: String::new(),
        type_text: String::new(),
    });
    members.chain(empty).collect()
}

/// `GET /machines/{code}/types/new`: show the form of a new struct, or of a new revision.
pub(super) async fn new_form(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    query: Result<Query<FormQuery>, QueryRejection>,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let from = query_values(query)?
        .from
        .as_deref()
        .map(type_name)
        .transpose()?;
    let form = GetUdtForm::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            UdtFormInput { code, from },
        )
        .await?;
    let shown = Shown {
        type_name: String::new(),
        rows: opening_rows(&form),
        focus_last: false,
    };
    respond(&state, &request, form, shown)
}
