//! `POST /machines/{code}/string-encoding`: change the `STRING` encoding of a live machine that has
//! no recipe version (`api.md` section 2.2, ADR-0202).
//!
//! A valid HTMX request gets the new encoding for `#machine-string-encoding`, the toast, the empty
//! error panel, and the clean form out of band (ADR-0041 (h)). A request without HTMX gets `303` to
//! the machine page. The route checks the permission `MachineManage` before it reads a field, so a
//! role without the right gets `403 E-WEB-002` (ADR-0046 (a)). The use case checks it again as its
//! first statement (ADR-0013). A text that is no encoding gives one `E-WEB-008` line under the
//! select. A domain fault (`E-PLT-053`, `E-PLT-054`, `E-PLT-010`) gives its own status with one line.
//! Every answer with a line carries the form with the sent text out of band.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use fluent::FluentArgs;
use recipe_app::auth::{Operation, authorize};
use recipe_app::error::AppError;
use recipe_app::plant::{
    SetMachineStringEncoding, SetMachineStringEncodingInput, SetMachineStringEncodingOutput,
};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::{MachineCode, PlantError, StringEncoding};
use serde::Deserialize;

use super::string_encoding_form::StringEncodingForm;
use crate::error::{FieldError, FieldReason, FormErrors, WebError};
use crate::pages::draft_reply::{refusal_lines, refused, with_form};
use crate::pages::form_fields::FieldLines;
use crate::pages::path_values::path_code;
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The `id` and the name of the select of the encoding.
const ENCODING_INPUT: &str = "string_encoding";

/// The fields of the form of `POST /machines/{code}/string-encoding`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StringEncodingFields {
    /// The encoding name, `Latin1` or `Utf8`.
    string_encoding: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// The block `string_encoding_form` of `machine.html` alone: the form that an answer swaps out of band.
#[derive(Template)]
#[template(path = "machine.html", block = "string_encoding_form")]
struct StringEncodingFormBlock<'a> {
    t: Texts<'a>,
    string_encoding_form: StringEncodingForm,
}

/// The answer `machine-string-encoding-set.html`: the new encoding and two out-of-band updates.
#[derive(Template)]
#[template(path = "machine-string-encoding-set.html")]
struct StringEncodingSetFragment {
    encoding: String,
    message: String,
}

/// Parse the sent text. A bad text is one line under the select.
fn parse_encoding(sent: &str) -> Result<StringEncoding, NonEmpty<FieldError>> {
    let mut lines = FieldLines::default();
    let parsed = lines.field(
        StringEncoding::parse(sent),
        ENCODING_INPUT,
        FieldReason::Choice,
    );
    lines.finish(parsed)
}

/// Return the `id` of the select that a fault of the domain names, or `None` for no input.
fn target_of(error: &AppError) -> Option<String> {
    match error {
        AppError::Plant(PlantError::StringEncodingNotOnPlatform { .. }) => {
            Some(ENCODING_INPUT.to_owned())
        }
        _other => None,
    }
}

/// Build the error with the lines and the form with the sent text.
fn refused_encoding(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    (sent, lines): (String, NonEmpty<FieldError>),
) -> WebError {
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| StringEncodingFormBlock {
        t,
        string_encoding_form: StringEncodingForm::new(code, sent, request.csrf_token())
            .out_of_band(errors),
    })
}

/// Answer a valid HTMX change: the encoding, the toast, the empty error panel, and the clean form.
fn saved(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    output: SetMachineStringEncodingOutput,
) -> Result<Response, WebError> {
    let t = Texts::new(state.bundles(), request.lang());
    let text = output.string_encoding.to_string();
    let mut args = FluentArgs::new();
    args.set("code", code.to_string());
    args.set("encoding", text.clone());
    let fragment = StringEncodingSetFragment {
        encoding: text.clone(),
        message: t.with("machine-string-encoding-set", &args),
    };
    let form = StringEncodingFormBlock {
        t,
        string_encoding_form: StringEncodingForm::new(code, text, request.csrf_token())
            .out_of_band(FormErrors::default()),
    };
    let body = render(&fragment, request.request_id())?;
    Ok(with_form(body, &form, request.request_id())?.into_response())
}

/// Run the use case. A domain refusal becomes the error with the line and the form of the sent text.
async fn run(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    (string_encoding, sent): (StringEncoding, String),
) -> Result<SetMachineStringEncodingOutput, WebError> {
    let input = SetMachineStringEncodingInput {
        code,
        string_encoding,
    };
    let error = match SetMachineStringEncoding::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await
    {
        Ok(output) => return Ok(output),
        Err(error) => error,
    };
    Err(match refusal_lines(error, target_of) {
        Ok(lines) => refused_encoding(state, request, code, (sent, lines)),
        Err(other) => other,
    })
}

/// `POST /machines/{code}/string-encoding`: set the encoding of the live machine.
pub(super) async fn set(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<StringEncodingFields>, FormRejection>,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    // The route checks the permission before the first field is read (ADR-0046 (a)).
    authorize(
        Operation::SetMachineStringEncoding,
        request.auth()?.user(),
        &request.meta(),
        state.security(),
    )
    .await?;
    let Form(fields) = form?;
    let encoding = match parse_encoding(&fields.string_encoding) {
        Ok(encoding) => encoding,
        Err(lines) => {
            return Err(refused_encoding(
                &state,
                &request,
                code,
                (fields.string_encoding, lines),
            ));
        }
    };
    let output = run(&state, &request, code, (encoding, fields.string_encoding)).await?;
    if !request.is_htmx() {
        return see_other_path(&format!("/machines/{code}"), request.request_id(), []);
    }
    saved(&state, &request, code, output)
}
