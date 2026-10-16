//! The commands of the UDT form: `POST /machines/{code}/types` and
//! `POST /machines/{code}/types/{name}/revisions` (`api.md` section 2.3, design D6 and D8).
//!
//! A body with `add=1` runs the query `GetUdtForm` and defines no type. It answers `200` with the
//! form, the sent rows, and one more empty row. Any other body runs the command `DefineUdt`.
//!
//! Before the command, the handler reads the name and every member row. A body with bad inputs
//! gets one line for each of them at once (ADR-0041 (a)). An HTMX answer also swaps the form out
//! of band with the sent texts and a line under each bad input (ADR-0041 (e)). A domain error of
//! `DefineUdt` gets the same form with only its line, for example `E-PLT-021` under a member row.

use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::http::HeaderValue;
use axum::http::header::HeaderName;
use axum::response::Response;
use recipe_app::auth::UdtFormPurpose;
use recipe_app::error::AppError;
use recipe_app::plant::{
    DefineUdt, DefineUdtInput, DefineUdtMode, GetUdtForm, UdtForm, UdtFormInput,
};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::limits::MAX_MEMBERS;
use recipe_domain::plant::{MachineCode, MemberName, PlantError, TypeExpr, TypeName};

use super::form::{Shown, refused_form, respond};
use super::form_rows::{SentForm, SentRow, read};
use super::{revision_href, type_href};
use crate::error::{FieldError, FieldReason, RowField, WebError};
use crate::pages::PageRequest;
use crate::pages::draft_reply::refusal_lines;
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
/// reason `TypeName`. Each line names the field `type_name`.
///
/// # Errors
///
/// Returns the line `E-WEB-008` of the field `type_name`.
fn new_name(text: Option<&str>) -> Result<TypeName, FieldError> {
    let bad = |reason| FieldError::bad_request("type_name", reason);
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
/// error of the domain (`E-PLT-013`) under the type input, because the type expression has its
/// own code. The row checks both inputs, so it can give two lines (ADR-0041 (a)).
///
/// # Errors
///
/// Returns the line of each bad input of the row.
fn member_input(
    row: u16,
    sent: &SentRow,
) -> Result<Option<(MemberName, TypeExpr)>, Vec<FieldError>> {
    let bad = |field, reason| vec![FieldError::bad_row(row, field, reason)];
    match (sent.name.is_empty(), sent.type_text.is_empty()) {
        (true, true) => return Ok(None),
        (true, false) => return Err(bad(RowField::Name, FieldReason::MemberRows)),
        (false, true) => return Err(bad(RowField::Type, FieldReason::MemberRows)),
        (false, false) => {}
    }
    let name = MemberName::parse(&sent.name)
        .map_err(|_reason| FieldError::bad_row(row, RowField::Name, FieldReason::MemberName));
    let type_input = format!("members-{row}-{}", RowField::Type.id_suffix());
    let type_expr = TypeExpr::parse(&sent.type_text)
        .map_err(|error| FieldError::plant(&error, Some(type_input)));
    match (name, type_expr) {
        (Ok(name), Ok(type_expr)) => Ok(Some((name, type_expr))),
        (name, type_expr) => Err(name.err().into_iter().chain(type_expr.err()).collect()),
    }
}

/// Read every member row and skip the empty ones.
///
/// The number of a row counts every row of the form, also the empty ones, so an error names the
/// row that the user sees.
///
/// # Errors
///
/// Returns the lines of every bad row, in the order of the rows.
fn member_inputs(rows: &[SentRow]) -> Result<Vec<(MemberName, TypeExpr)>, Vec<FieldError>> {
    let mut members = Vec::with_capacity(rows.len());
    let mut lines = Vec::new();
    for (index, sent) in rows.iter().enumerate() {
        let Ok(row) = u16::try_from(index) else {
            lines.push(FieldError::bad_request("members", FieldReason::MemberCount));
            break;
        };
        match member_input(row, sent) {
            Ok(member) => members.extend(member),
            Err(row_lines) => lines.extend(row_lines),
        }
    }
    if lines.is_empty() {
        Ok(members)
    } else {
        Err(lines)
    }
}

/// Read the name and every member row of the command (ADR-0041 (a)).
///
/// # Errors
///
/// Returns the line of the name first, then the lines of the rows.
fn define_input(
    code: MachineCode,
    target: &Target,
    rows: &[SentRow],
    name: Option<&str>,
) -> Result<DefineUdtInput, NonEmpty<FieldError>> {
    let mut lines = Vec::new();
    let named = match target {
        Target::New => new_name(name)
            .map(|name| (name, DefineUdtMode::New))
            .map_err(|line| lines.push(line))
            .ok(),
        Target::Revision(name) => Some((name.clone(), DefineUdtMode::Revision)),
    };
    let members = member_inputs(rows)
        .map_err(|row_lines| lines.extend(row_lines))
        .ok();
    match (NonEmpty::try_from(lines), named, members) {
        (Err(_none), Some((name, mode)), Some(members)) => Ok(DefineUdtInput {
            code,
            name,
            members,
            mode,
        }),
        (Ok(lines), _named, _members) => Err(lines),
        // A failed part always adds its line, so this arm only keeps the match total.
        (Err(_none), _named, _members) => Err(NonEmpty::new(FieldError::bad_request(
            "form",
            FieldReason::Form,
        ))),
    }
}

/// Return the `id` of a member input: the name or the type input of the row `row`.
fn row_input(row: usize, field: RowField) -> String {
    format!("members-{row}-{}", field.id_suffix())
}

/// Return the row with the member name `member`, after `skip` earlier rows with that name.
fn member_row(rows: &[SentRow], member: &MemberName, skip: usize) -> Option<usize> {
    rows.iter()
        .enumerate()
        .filter(|(_row, sent)| MemberName::parse(&sent.name).is_ok_and(|name| name == *member))
        .nth(skip)
        .map(|(row, _sent)| row)
}

/// Return the input that a refused `DefineUdt` names (ADR-0041 (e)).
///
/// `E-PLT-015` names the name input of a new struct. `E-PLT-021` names the second row with the
/// member name. `E-PLT-006` and `E-PLT-013` name the type input of the first matching row. Any
/// other error names no input.
fn refused_input(error: &AppError, target: &Target, rows: &[SentRow]) -> Option<String> {
    let AppError::Plant(error) = error else {
        return None;
    };
    match error {
        PlantError::DuplicateTypeName { .. } => {
            matches!(target, Target::New).then(|| "type_name".to_owned())
        }
        PlantError::DuplicateMemberName { member, .. } => {
            member_row(rows, member, 1).map(|row| row_input(row, RowField::Name))
        }
        PlantError::MemberTypeNotOnPlatform { member, .. } => {
            member_row(rows, member, 0).map(|row| row_input(row, RowField::Type))
        }
        PlantError::DerivedTypeInvalid { type_expr, .. } => rows
            .iter()
            .position(|sent| {
                TypeExpr::parse(&sent.type_text)
                    .is_ok_and(|parsed| parsed.to_string() == *type_expr)
            })
            .map(|row| row_input(row, RowField::Type)),
        _other => None,
    }
}

/// Run the query `GetUdtForm` for the form of `target` with the purpose `DefineUdt`.
///
/// The query checks the permission of `DefineUdt` first. It gives the proposals and checks that
/// the machine and the struct exist.
async fn udt_form(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    target: &Target,
) -> Result<UdtForm, WebError> {
    let from = match target {
        Target::New => None,
        Target::Revision(name) => Some(name.clone()),
    };
    Ok(GetUdtForm::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            UdtFormInput {
                purpose: UdtFormPurpose::DefineUdt,
                code,
                from,
            },
        )
        .await?)
}

/// Build the error of a body with bad inputs: every line and the form with the sent texts.
///
/// The error of `GetUdtForm` wins, for example `403 E-WEB-002` for a role without the right.
async fn refuse(
    state: &WebState,
    request: &PageRequest,
    (code, target, sent): (MachineCode, &Target, SentForm),
    lines: NonEmpty<FieldError>,
) -> WebError {
    let form = match udt_form(state, request, code, target).await {
        Ok(form) => form,
        Err(error) => return error,
    };
    let shown = Shown {
        type_name: sent.name.unwrap_or_default(),
        rows: sent.rows,
        focus_last: false,
    };
    refused_form(state, request, (form, shown), lines)
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
    let form = udt_form(state, request, code, target).await?;
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
    let input = match define_input(code, &target, &sent.rows, sent.name.as_deref()) {
        Ok(input) => input,
        Err(lines) => return Err(refuse(state, request, (code, &target, sent), lines).await),
    };
    let (name, mode) = (input.name.clone(), input.mode);
    let result = DefineUdt::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    let defined = match result {
        Ok(defined) => defined,
        Err(error) => {
            let named = |refusal: &AppError| refused_input(refusal, &target, &sent.rows);
            return Err(match refusal_lines(error, named) {
                Ok(lines) => refuse(state, request, (code, &target, sent), lines).await,
                Err(other) => other,
            });
        }
    };
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
    // The form query checks the permission of `DefineUdt` before the fields are read (ADR-0046).
    udt_form(&state, &request, code, &Target::New).await?;
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
    let target = Target::Revision(name);
    // The form query checks the permission of `DefineUdt` before the fields are read (ADR-0046).
    udt_form(&state, &request, code, &target).await?;
    let Form(pairs) = form?;
    run(&state, &request, code, (target, &pairs)).await
}
