//! The commands of the UDT form: `POST /machines/{code}/types` and
//! `POST /machines/{code}/types/{name}/revisions` (`api.md` section 2.3, design D6 and D8).
//!
//! A body with `add=1` runs the query `GetUdtForm` and defines no type. It answers `200` with the
//! form, the sent rows, and one more empty row. Any other body runs the command `DefineUdt`.

use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::http::HeaderValue;
use axum::http::header::HeaderName;
use axum::response::Response;
use recipe_app::plant::{DefineUdt, DefineUdtInput, DefineUdtMode, GetUdtForm, UdtFormInput};
use recipe_domain::plant::limits::MAX_MEMBERS;
use recipe_domain::plant::{MachineCode, MemberName, TypeExpr, TypeName};

use super::form::{Shown, respond};
use super::form_rows::{SentForm, SentRow, read};
use super::{revision_href, type_href};
use crate::error::{FieldReason, RowField, WebError};
use crate::pages::PageRequest;
use crate::pages::path_values::{path_code, path_code_type};
use crate::pages::render::see_other_path;
use crate::state::WebState;

/// The name that no struct can have, because the route `/types/new` would hide its page
/// (ADR-0037 y).
const RESERVED_NAME: &str = "new";

/// The header that keeps the address bar on the URL of the form.
const HX_PUSH_URL: HeaderName = HeaderName::from_static("hx-push-url");

/// What the command defines: a new struct, or the next revision of the struct that the URL names.
enum Target {
    /// `POST /machines/{code}/types`. The name comes from the body.
    New,
    /// `POST /machines/{code}/types/{name}/revisions`. The name comes from the URL.
    Revision(TypeName),
}

/// Build the error of a member row that has an input which is not valid.
const fn bad_row(row: u16, field: RowField, reason: FieldReason) -> WebError {
    WebError::BadRow { row, field, reason }
}

/// Build the error of a form that holds more member rows than `type.max_members`.
const fn member_count() -> WebError {
    WebError::BadRequest {
        field: "members",
        reason: FieldReason::MemberCount,
    }
}

/// Read the name of a new struct.
///
/// A missing name is a form error. The name `new` and a text that is not a `TypeName` give the
/// reason `TypeName`. Both errors name the field `type_name`.
fn new_name(text: Option<&str>) -> Result<TypeName, WebError> {
    let bad = |reason| WebError::BadRequest {
        field: "type_name",
        reason,
    };
    let text = text.ok_or_else(|| bad(FieldReason::Form))?;
    if text == RESERVED_NAME {
        return Err(bad(FieldReason::TypeName));
    }
    TypeName::parse(text).map_err(|_reason| bad(FieldReason::TypeName))
}

/// Read one member row. A row with an empty name and an empty type gives `None`.
///
/// A row with only one of them gives the reason `MemberRows` for the empty input. A name that is
/// not a `MemberName` gives the reason `MemberName`. A type that is not a `TypeExpr` gives the
/// error of the domain (`E-PLT-013`), because the type expression has its own code.
fn member_input(row: u16, sent: &SentRow) -> Result<Option<(MemberName, TypeExpr)>, WebError> {
    match (sent.name.is_empty(), sent.type_text.is_empty()) {
        (true, true) => return Ok(None),
        (true, false) => {
            return Err(bad_row(row, RowField::Name, FieldReason::MemberRows));
        }
        (false, true) => {
            return Err(bad_row(row, RowField::Type, FieldReason::MemberRows));
        }
        (false, false) => {}
    }
    let name = MemberName::parse(&sent.name)
        .map_err(|_reason| bad_row(row, RowField::Name, FieldReason::MemberName))?;
    let type_expr =
        TypeExpr::parse(&sent.type_text).map_err(|error| WebError::App(error.into()))?;
    Ok(Some((name, type_expr)))
}

/// Read every member row and skip the empty ones.
///
/// The number of a row counts every row of the form, also the empty ones, so an error names the
/// row that the user sees.
fn member_inputs(rows: &[SentRow]) -> Result<Vec<(MemberName, TypeExpr)>, WebError> {
    let mut members = Vec::with_capacity(rows.len());
    for (index, sent) in rows.iter().enumerate() {
        let row = u16::try_from(index).map_err(|_error| member_count())?;
        members.extend(member_input(row, sent)?);
    }
    Ok(members)
}

/// Answer a body with `add=1`: show the form with the sent rows and one empty row.
///
/// The function defines no type. It runs the query `GetUdtForm` for the proposals and for the
/// check that the machine and the struct exist. The answer sets `HX-Push-Url: false`, so the
/// address bar keeps the URL of the form.
async fn add_row(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    (target, sent): (&Target, SentForm),
) -> Result<Response, WebError> {
    let mut rows = sent.rows;
    if rows.len() >= MAX_MEMBERS {
        return Err(member_count());
    }
    rows.push(SentRow {
        name: String::new(),
        type_text: String::new(),
    });
    let from = match target {
        Target::New => None,
        Target::Revision(name) => Some(name.clone()),
    };
    let form = GetUdtForm::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            UdtFormInput { code, from },
        )
        .await?;
    let shown = Shown {
        type_name: sent.name.unwrap_or_default(),
        rows,
        focus_last: true,
    };
    let mut response = respond(state, request, form, shown)?;
    response
        .headers_mut()
        .insert(HX_PUSH_URL, HeaderValue::from_static("false"));
    Ok(response)
}

/// Run the command `DefineUdt` and redirect to the page of the new struct or revision.
async fn define(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    (target, sent): (Target, SentForm),
) -> Result<Response, WebError> {
    let (name, mode) = match target {
        Target::New => (new_name(sent.name.as_deref())?, DefineUdtMode::New),
        Target::Revision(name) => (name, DefineUdtMode::Revision),
    };
    let members = member_inputs(&sent.rows)?;
    let input = DefineUdtInput {
        code,
        name: name.clone(),
        members,
        mode,
    };
    let defined = DefineUdt::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    let location = match mode {
        DefineUdtMode::New => type_href(code, &name),
        DefineUdtMode::Revision => revision_href(code, &name, defined.revision),
    };
    see_other_path(&location, request.request_id(), [])
}

/// Read the body and run the query or the command that it asks for.
async fn run(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    (target, pairs): (Target, &[(String, String)]),
) -> Result<Response, WebError> {
    let sent = read(pairs)?;
    if sent.add {
        add_row(state, request, code, (&target, sent)).await
    } else {
        define(state, request, code, (target, sent)).await
    }
}

/// `POST /machines/{code}/types`: define a new struct, or add a member row with `add=1`.
pub(super) async fn define_new(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<Vec<(String, String)>>, FormRejection>,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let Form(pairs) = form?;
    run(&state, &request, code, (Target::New, &pairs)).await
}

/// `POST /machines/{code}/types/{name}/revisions`: define the next revision of a struct, or add a
/// member row with `add=1`.
///
/// The name comes from the URL. A field `name` in the body does not change it.
pub(super) async fn define_revision(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<Vec<(String, String)>>, FormRejection>,
) -> Result<Response, WebError> {
    let (code, name) = path_code_type(path)?;
    let Form(pairs) = form?;
    run(&state, &request, code, (Target::Revision(name), &pairs)).await
}
