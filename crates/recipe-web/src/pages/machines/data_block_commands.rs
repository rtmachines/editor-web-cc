//! `POST /machines/{code}/data-block`: change the data block number of a live machine (`api.md`
//! section 2.2, ADR-0156, design D6 and D8).
//!
//! A valid HTMX request gets the new number for `#machine-db-number`, the toast, the empty error
//! panel, and the clean form out of band (ADR-0041 (h)). A request without HTMX gets `303` to the
//! machine page. The route checks the permission `MachineManage` before it reads a field, so a role
//! without the right gets `403 E-WEB-002` (ADR-0046 (a)). The use case checks it again as its first
//! statement (ADR-0013). A text that is no number gives one `E-WEB-008` line under the input. A
//! domain fault of the number (`E-PLT-024`, `E-PLT-050`, `E-PLT-051`) gives `422` with its own
//! line under the input. Both answers carry the form with the sent text out of band.

use askama::Template;
use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::{Operation, authorize};
use recipe_app::error::AppError;
use recipe_app::plant::{
    GetMachine, GetMachineInput, MachineRef, SetMachineDataBlock, SetMachineDataBlockInput,
    SetMachineDataBlockOutput,
};
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::{DataBlockNumber, MachineCode, PlantError};
use serde::Deserialize;

use super::data_block_form::DataBlockForm;
use crate::error::{FieldError, FieldReason, FormErrors, WebError};
use crate::pages::draft_reply::{refusal_lines, refused, with_form};
use crate::pages::form_fields::FieldLines;
use crate::pages::path_values::path_code;
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The `id` and the name of the input of the data block number (design D8).
const DB_NUMBER_INPUT: &str = "db_number";

/// The fields of the form of `POST /machines/{code}/data-block`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DataBlockFields {
    /// The data block number. An empty text is no value (design D5).
    db_number: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// The block `data_block_form` of `machine.html` alone: the form that an answer swaps out of band.
#[derive(Template)]
#[template(path = "machine.html", block = "data_block_form")]
struct DataBlockFormBlock<'a> {
    t: Texts<'a>,
    data_block_form: DataBlockForm,
}

/// The answer `machine-data-block-set.html`: the new number and two out-of-band updates.
#[derive(Template)]
#[template(path = "machine-data-block-set.html")]
struct DataBlockSetFragment {
    number: String,
    message: String,
}

/// Parse the sent text. An empty text is no value. A bad text is one line under the input.
fn parse_number(sent: &str) -> Result<Option<DataBlockNumber>, NonEmpty<FieldError>> {
    if sent.is_empty() {
        return Ok(None);
    }
    let mut lines = FieldLines::default();
    let parsed = lines.field(
        DataBlockNumber::parse(sent),
        DB_NUMBER_INPUT,
        FieldReason::ConnectionDataBlock,
    );
    lines.finish(parsed).map(Some)
}

/// Return the `id` of the input that a fault of the domain names, or `None` for no input.
fn target_of(error: &AppError) -> Option<String> {
    match error {
        AppError::Plant(
            PlantError::DataBlockMissing { .. }
            | PlantError::DataBlockNotOnPlatform { .. }
            | PlantError::DataBlockOutOfRange { .. },
        ) => Some(DB_NUMBER_INPUT.to_owned()),
        _other => None,
    }
}

/// Return the highest data block number that a domain refusal names, or `None` for no hint.
///
/// The function solves one problem: the refused form shows the range of the platform, and these
/// refusals already carry the platform or the maximum, so the route needs no second read.
fn max_of_refusal(error: &AppError) -> Option<u16> {
    match error {
        AppError::Plant(PlantError::DataBlockOutOfRange { max, .. }) => Some(*max),
        AppError::Plant(
            PlantError::DataBlockMissing { platform, .. }
            | PlantError::DataBlockNotOnPlatform { platform, .. },
        ) => platform.max_data_block(),
        _other => None,
    }
}

/// Read the highest data block number of the platform of the live machine `code`.
///
/// A refusal of the sent text comes before the use case, so only a read can name the platform.
///
/// # Errors
///
/// Returns the `WebError` of the read. The mapping logs it once and the answer has no form.
async fn max_of_platform(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
) -> Result<Option<u16>, WebError> {
    let input = GetMachineInput {
        machine: MachineRef::Code(code),
        archived: None,
    };
    let view = GetMachine::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    Ok(view.machine.platform().max_data_block())
}

/// Build the error with the lines and the form with the sent text and the hint `max`.
fn refused_number(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    (sent, lines): (String, NonEmpty<FieldError>),
    max: Option<u16>,
) -> WebError {
    let t = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| DataBlockFormBlock {
        t,
        data_block_form: DataBlockForm::new(code, sent, request.csrf_token())
            .with_max(max)
            .out_of_band(errors),
    })
}

/// Answer a valid HTMX change: the number, the toast, the empty error panel, and the clean form.
fn saved(
    state: &WebState,
    request: &PageRequest,
    code: MachineCode,
    (number, max): (DataBlockNumber, Option<u16>),
) -> Result<Response, WebError> {
    let t = Texts::new(state.bundles(), request.lang());
    let text = number.value().to_string();
    let fragment = DataBlockSetFragment {
        number: text.clone(),
        message: t.with_arg("machine-data-block-set", "code", code),
    };
    let form = DataBlockFormBlock {
        t,
        data_block_form: DataBlockForm::new(code, text, request.csrf_token())
            .with_max(max)
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
    (db_number, sent): (Option<DataBlockNumber>, String),
) -> Result<SetMachineDataBlockOutput, WebError> {
    let input = SetMachineDataBlockInput { code, db_number };
    let error = match SetMachineDataBlock::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await
    {
        Ok(output) => return Ok(output),
        Err(error) => error,
    };
    let max = max_of_refusal(&error);
    Err(match refusal_lines(error, target_of) {
        Ok(lines) => refused_number(state, request, code, (sent, lines), max),
        Err(other) => other,
    })
}

/// `POST /machines/{code}/data-block`: set the data block number of the live machine.
pub(super) async fn set(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<DataBlockFields>, FormRejection>,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    // The route checks the permission before the first field is read (ADR-0046 (a)).
    authorize(
        Operation::SetMachineDataBlock,
        request.auth()?.user(),
        &request.meta(),
        state.security(),
    )
    .await?;
    let Form(fields) = form?;
    let db_number = match parse_number(&fields.db_number) {
        Ok(db_number) => db_number,
        Err(lines) => {
            let max = max_of_platform(&state, &request, code).await?;
            return Err(refused_number(
                &state,
                &request,
                code,
                (fields.db_number, lines),
                max,
            ));
        }
    };
    let output = run(&state, &request, code, (db_number, fields.db_number)).await?;
    if !request.is_htmx() {
        return see_other_path(&format!("/machines/{code}"), request.request_id(), []);
    }
    // A saved request has a number: the use case refuses an empty one on every platform.
    match db_number {
        Some(number) => saved(
            &state,
            &request,
            code,
            (number, output.platform.max_data_block()),
        ),
        None => Err(WebError::Internal {
            request_id: request.request_id(),
        }),
    }
}
