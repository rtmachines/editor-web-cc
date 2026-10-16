//! The UDT form: `GET /machines/{code}/types/new`, the answer of `POST` with `add=1`, and the
//! out-of-band form of an error answer (`api.md` section 2.3, design D6, D8, and D9).

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::UdtFormPurpose;
use recipe_app::plant::{GetUdtForm, UdtForm, UdtFormInput};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::limits::MAX_MEMBERS;
use serde::Deserialize;

use super::form_rows::SentRow;
use super::{current_nav, query_values};
use crate::error::{FieldError, FormErrors, RowField, WebError};
use crate::layout::Layout;
use crate::pages::draft_reply::refused;
use crate::pages::path_values::{path_code, type_name};
use crate::pages::render::render;
use crate::pages::{PageRequest, Texts};
use crate::state::WebState;

/// The query of `GET /machines/{code}/types/new`.
///
/// The struct accepts only `from`. A request with `csrf_token` or a member row in the query gives
/// `422 E-WEB-008`, so no URL holds the token or the rows of a form (ADR-0037 h). The form query
/// checks the permission first, so a role without `UdtManage` gets `403` (ADR-0046 (a)).
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

/// One member row of the form.
struct MemberRow {
    /// The index of the row, from 0, as in `members[<index>].name`.
    index: usize,
    /// The number of the row in the labels, from 1.
    number: usize,
    name: String,
    type_text: String,
    /// True when the name input has `autofocus`.
    focus: bool,
    /// The `id` of the name input, the key of its error lines.
    name_id: String,
    /// The `id` of the type input, the key of its error lines.
    type_id: String,
}

/// The data of the UDT form: the action, the texts, the rows, and the error lines.
///
/// The type solves one problem: the page and the out-of-band form of an error answer show the
/// same form (ADR-0041 (e)), so one view gives both the same fields.
struct UdtFormView {
    action: String,
    csrf_token: String,
    /// True for the form of a new revision. It has no name input.
    revision: bool,
    type_name: String,
    rows: Vec<MemberRow>,
    choices: Vec<String>,
    /// True when the form holds `type.max_members` rows and cannot add one.
    at_max: bool,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    oob: bool,
    errors: FormErrors,
}

impl UdtFormView {
    /// Build the clean form from the query result `form` and the rows `shown`.
    ///
    /// The form of a new revision (`form.from`) posts to `.../revisions`. The form of a new struct
    /// posts to `/machines/{code}/types`.
    fn new(form: UdtForm, shown: Shown, csrf_token: String) -> Self {
        let code = form.machine.code();
        let action = match &form.from {
            Some((name, _revision)) => format!("/machines/{code}/types/{name}/revisions"),
            None => format!("/machines/{code}/types"),
        };
        Self {
            action,
            csrf_token,
            revision: form.from.is_some(),
            type_name: shown.type_name,
            at_max: shown.rows.len() >= MAX_MEMBERS,
            rows: member_rows(shown.rows, shown.focus_last),
            choices: form.choices,
            oob: false,
            errors: FormErrors::default(),
        }
    }
}

/// The page `udt-form.html`.
#[derive(Template)]
#[template(path = "udt-form.html")]
struct UdtFormPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    title: String,
    form: UdtFormView,
    /// True when a row takes the focus, so the heading keeps the plain form.
    row_focus: bool,
    list_href: String,
    list_text: String,
}

/// The block `form` of `udt-form.html` alone: the form that an error answer swaps out of band.
#[derive(Template)]
#[template(path = "udt-form.html", block = "form")]
struct UdtFormBlock<'a> {
    t: Texts<'a>,
    form: UdtFormView,
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
            name_id: format!("members-{index}-{}", RowField::Name.id_suffix()),
            type_id: format!("members-{index}-{}", RowField::Type.id_suffix()),
        })
        .collect()
}

/// Return the title of the form: a new struct, or a new revision of the struct `form.from`.
fn title_of(t: Texts<'_>, form: &UdtForm) -> String {
    form.from.as_ref().map_or_else(
        || t.get("udt-new-title"),
        |(name, _revision)| t.with_arg("udt-revision-title", "name", name),
    )
}

/// Answer with the UDT form `form` and the rows `shown`.
///
/// The function solves one problem: `GET .../types/new` and `POST` with `add=1` show the same
/// form, so one function builds both answers.
pub(super) fn respond(
    state: &WebState,
    request: &PageRequest,
    form: UdtForm,
    shown: Shown,
) -> Result<Response, WebError> {
    let t = Texts::new(state.bundles(), request.lang());
    let code = form.machine.code();
    let title = title_of(t, &form);
    let layout = Layout::for_session(t, request, title.clone(), current_nav())?;
    let row_focus = shown.focus_last;
    let page = UdtFormPage {
        form: UdtFormView::new(form, shown, layout.csrf_token.clone()),
        layout,
        t,
        title,
        row_focus,
        list_href: format!("/machines/{code}/types"),
        list_text: t.with_arg("types-title", "code", code),
    };
    Ok(render(&page, request.request_id())?.into_response())
}

/// Build the error of a form with bad inputs: every line and the form with the sent texts.
///
/// The form keeps the sent name and rows and shows each line under its input (ADR-0041 (e)).
pub(super) fn refused_form(
    state: &WebState,
    request: &PageRequest,
    (form, shown): (UdtForm, Shown),
    lines: NonEmpty<FieldError>,
) -> WebError {
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| UdtFormBlock {
        t,
        form: UdtFormView {
            oob: true,
            errors,
            ..UdtFormView::new(form, shown, request.csrf_token())
        },
    })
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
    let wanted =
        query_values(query).and_then(|values| values.from.as_deref().map(type_name).transpose());
    // The form query checks the permission of `DefineUdt` before a bad query value gives its answer
    // (ADR-0046 (a)). A bad query value runs the query without an origin.
    let from = wanted.as_ref().ok().and_then(Option::as_ref).cloned();
    let form = GetUdtForm::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            UdtFormInput {
                purpose: UdtFormPurpose::DefineUdt,
                code,
                from,
            },
        )
        .await?;
    wanted?;
    let shown = Shown {
        type_name: String::new(),
        rows: opening_rows(&form),
        focus_last: false,
    };
    respond(&state, &request, form, shown)
}
