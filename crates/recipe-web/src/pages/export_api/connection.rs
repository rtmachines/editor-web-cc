//! The connection route of the transfer client (`api.md` sections 2.7 and 6.5, ADR-0154 (j),
//! design D9).
//!
//! The route gives the PLC connection record of one machine with its plain password. It is the only
//! answer that holds the password (ADR-0154 (h)). The use case commits the audit entry first, so
//! the handler builds the body only after the commit. Every error goes to the error layer, which
//! answers JSON on these paths (ADR-0053) and never holds the password.

use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use recipe_app::error::{AppError, Missing};
use recipe_app::export::{ExportPlcConnection, ExportPlcConnectionInput, ExportedPlcConnection};
use recipe_domain::export::ExportError;
use recipe_domain::plant::PlcConnection;
use recipe_domain::recipe::ReferenceNumber;
use serde::{Deserialize, Serialize};

use super::super::path_values::{path_code, reference};
use crate::basic_auth::BasicUser;
use crate::error::WebError;
use crate::state::WebState;

/// The `schema_version` of the connection document (`api.md` section 6.5).
const SCHEMA_VERSION: &str = "1";

/// The query string of the route. The route needs only the value `recipe`.
///
/// The struct keeps the value as text, so a missing value, a malformed value, and an unknown key
/// give the same answer (`404 E-WEB-005`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConnectionQuery {
    recipe: Option<String>,
}

/// The body of the connection document, in the field order of `api.md` section 6.5.
///
/// The type solves one problem: the document must carry the password and must still never reach
/// `Debug` or a log line. It borrows the password text from the use case result, has no `Debug`,
/// and lives only inside [`write_document`].
#[derive(Serialize)]
struct Document<'a> {
    schema_version: &'static str,
    machine: MachineBody,
    recipe: RecipeBody<'a>,
    row_version: i64,
    connection: ConnectionBody<'a>,
}

/// The object `machine` of the document.
#[derive(Serialize)]
struct MachineBody {
    code: String,
    plc_platform: String,
}

/// The object `recipe` of the document.
#[derive(Serialize)]
struct RecipeBody<'a> {
    reference_number: &'a str,
}

/// The object `connection` of the document.
///
/// A field that belongs to another platform or to credentials is absent and never `null`
/// (`api.md` section 6.5).
#[derive(Serialize)]
struct ConnectionBody<'a> {
    host: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    rack: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slot: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    db_number: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comm_mode: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    login: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<&'a str>,
}

/// Build the object `connection` from the record and the opened password.
fn connection_body<'a>(
    exported: &'a ExportedPlcConnection,
    password: Option<&'a str>,
) -> ConnectionBody<'a> {
    match &exported.connection {
        PlcConnection::S7 {
            host,
            rack,
            slot,
            db_number,
        } => ConnectionBody {
            host: host.as_str(),
            rack: Some(rack.value()),
            slot: Some(slot.value()),
            db_number: Some(db_number.value()),
            port: None,
            address: None,
            comm_mode: None,
            login: None,
            password: None,
        },
        PlcConnection::Iec {
            host,
            port,
            address,
            mode,
            login,
        } => ConnectionBody {
            host: host.as_str(),
            rack: None,
            slot: None,
            db_number: None,
            port: Some(port.value()),
            address: Some(address.as_str()),
            comm_mode: Some(mode.as_str()),
            login: login.as_ref().map(|login| login.as_str()),
            password,
        },
    }
}

/// Write the connection document of `exported` as pretty JSON.
///
/// # Errors
///
/// Returns `E-EXP-004` (`ExportError::Serialize`) when `serde_json` fails.
fn write_document(exported: &ExportedPlcConnection) -> Result<Vec<u8>, ExportError> {
    let password = exported.password.as_ref().map(|password| password.expose());
    let document = Document {
        schema_version: SCHEMA_VERSION,
        machine: MachineBody {
            code: exported.code.to_string(),
            plc_platform: exported.platform.to_string(),
        },
        recipe: RecipeBody {
            reference_number: exported.recipe.as_str(),
        },
        row_version: exported.row_version,
        connection: connection_body(exported, password),
    };
    Ok(serde_json::to_vec_pretty(&document)?)
}

/// Read the query value `recipe`.
///
/// # Errors
///
/// Returns `404 E-WEB-005` when the query string is bad, the value is missing, or the value is not
/// a reference number. The answer holds no typed text.
fn recipe_value(
    query: Result<Query<ConnectionQuery>, QueryRejection>,
) -> Result<ReferenceNumber, WebError> {
    let malformed = || {
        WebError::App(AppError::NotFound {
            entity: Missing::MalformedRecipe,
        })
    };
    let Query(values) = query.map_err(|_rejection| malformed())?;
    let text = values.recipe.ok_or_else(malformed)?;
    reference(&text)
}

/// `GET /export/v1/machines/{code}/connection?recipe={ref}`: send the connection record of the
/// machine with its password.
pub(super) async fn connection(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<ConnectionQuery>, QueryRejection>,
    account: BasicUser,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let recipe = recipe_value(query)?;
    let cipher = state.credential_cipher().ok_or(WebError::Internal {
        request_id: account.meta().request_id(),
    })?;
    let exported = ExportPlcConnection::new(state.identity().app().clone(), cipher)
        .execute(
            account.user(),
            account.meta(),
            ExportPlcConnectionInput { code, recipe },
        )
        .await?;
    let body = write_document(&exported).map_err(AppError::from)?;
    let headers = [
        (CONTENT_TYPE, HeaderValue::from_static("application/json")),
        (CACHE_CONTROL, HeaderValue::from_static("no-store")),
    ];
    Ok((StatusCode::OK, headers, body).into_response())
}
