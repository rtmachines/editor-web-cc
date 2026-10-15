//! The parsers of the path values and the query values of the plant pages (design D5).
//!
//! The module solves one problem: the machine, unit, and type pages read the same path values and
//! query values. Each bad value needs the same answer on every page. One parser for each value
//! gives that answer, so no page repeats a rule or invents a code (`api.md` section 1).
//!
//! A path value that names a resource gives `404 E-WEB-005` when it is malformed. The error holds
//! no text, so no typed value reaches a message or a log line. A unit path gives the code of
//! `UnitPath`. A query value of a `GET` form gives `422 E-WEB-008` with a fixed field name.

use axum::extract::path::ErrorKind;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query};
use recipe_app::error::{AppError, Missing};
use recipe_domain::plant::{MachineCode, MachineId, PlantError, TypeName, UnitPath};
use uuid::Uuid;

use crate::error::{FieldReason, WebError};

/// The length of a UUID in the hyphenated form. The route `/machines/by-id/{uuid}` takes only it.
const HYPHENATED_UUID_LENGTH: usize = 36;
/// The name of the path value that holds a unit path in the routes of the plant pages.
const UNIT_PATH_VALUE: &str = "path";
/// The name of the path value that holds a struct name in the routes of the type pages.
const TYPE_NAME_VALUE: &str = "name";
/// The text of a unit path that is not UTF-8. It is the replacement character of a lossy decoding.
const UNDECODABLE_TEXT: &str = "\u{FFFD}";

/// Build the `404 E-WEB-005` error for a path value that is malformed.
fn not_found(entity: Missing) -> WebError {
    WebError::App(AppError::NotFound { entity })
}

/// Read a whole number in the canonical form: ASCII digits only and no leading zero.
///
/// The function returns `None` for an empty text, a sign, a space, and a value above `u32::MAX`.
fn whole_number(text: &str) -> Option<u32> {
    let canonical = !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    if canonical {
        text.parse::<u32>().ok()
    } else {
        None
    }
}

/// Read the path value `{code}`. A malformed text gives `Missing::MalformedMachine`.
///
/// # Errors
///
/// Returns `404 E-WEB-005` when the text is not a machine code.
pub(crate) fn machine_code(text: &str) -> Result<MachineCode, WebError> {
    MachineCode::parse(text).map_err(|_reason| not_found(Missing::MalformedMachine))
}

/// Read the path value `{uuid}`. It takes the hyphenated form only.
///
/// # Errors
///
/// Returns `404 E-WEB-005` with `Missing::MalformedMachine` when the text is not such a UUID.
pub(crate) fn machine_id(text: &str) -> Result<MachineId, WebError> {
    let malformed = || not_found(Missing::MalformedMachine);
    if text.len() != HYPHENATED_UUID_LENGTH {
        return Err(malformed());
    }
    Uuid::parse_str(text)
        .map(MachineId::from)
        .map_err(|_reason| malformed())
}

/// Read the path value `{name}` or the query value `from`.
///
/// # Errors
///
/// Returns `404 E-WEB-005` with `Missing::MalformedType` when the text is not a struct name.
pub(crate) fn type_name(text: &str) -> Result<TypeName, WebError> {
    TypeName::parse(text).map_err(|_reason| not_found(Missing::MalformedType))
}

/// Read the query value `rev`: a whole number from 1 with no sign and no leading zero.
///
/// # Errors
///
/// Returns `404 E-WEB-005` with `Missing::MalformedType` for any other text.
pub(crate) fn revision(text: &str) -> Result<u32, WebError> {
    whole_number(text)
        .filter(|number| *number > 0)
        .ok_or_else(|| not_found(Missing::MalformedType))
}

/// Read the path value `{path}` or the query value `parent_path` or `archived` of a machine page.
///
/// # Errors
///
/// Returns the error of `UnitPath::parse`: `E-PLT-017` for a malformed text and `E-PLT-002` for a
/// path with more than five segments.
pub(crate) fn unit_path(text: &str) -> Result<UnitPath, WebError> {
    UnitPath::parse(text).map_err(|error| WebError::App(error.into()))
}

/// Read the query value `archived` of `/machines`: the code of the machine that was archived.
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the field `archived` when the text is not a machine code.
pub(crate) fn archived_machine(text: &str) -> Result<MachineCode, WebError> {
    MachineCode::parse(text).map_err(|_reason| WebError::BadRequest {
        field: "archived",
        reason: FieldReason::MachineCode,
    })
}

/// A page of a list. The URL counts from 1 and the use case counts from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Page {
    /// The page number of the URL. The first page is 1.
    pub(crate) number: u32,
    /// The page number of the use case. The first page is 0.
    pub(crate) index: u32,
}

/// Read the query value `page`. A missing value is the first page.
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the field `page` when the text is not a whole number from 1.
pub(crate) fn page(text: Option<&str>) -> Result<Page, WebError> {
    let bad = || WebError::BadRequest {
        field: "page",
        reason: FieldReason::Page,
    };
    let Some(text) = text else {
        return Ok(Page {
            number: 1,
            index: 0,
        });
    };
    let number = whole_number(text).ok_or_else(bad)?;
    let index = number.checked_sub(1).ok_or_else(bad)?;
    Ok(Page { number, index })
}

/// Read a flag field such as `show_archived` or `add`. A missing value is false.
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the field `field` and `FieldReason::Flag` for any value but `1`.
pub(crate) fn flag(field: &'static str, text: Option<&str>) -> Result<bool, WebError> {
    match text {
        None => Ok(false),
        Some("1") => Ok(true),
        Some(_other) => Err(WebError::BadRequest {
            field,
            reason: FieldReason::Flag,
        }),
    }
}

/// Unwrap the query string of a `GET` route.
///
/// The function solves one problem: every plant page must answer a bad query string in the same
/// way. A key that the route does not list, a key that appears twice, or a value that is not text
/// gives `422 E-WEB-008` with the field `query`. The answer never holds the typed text, so a typed
/// value cannot reach a message or a log line.
///
/// # Errors
///
/// Returns `422 E-WEB-008` when the rejection says that the query string is bad.
pub(crate) fn query_values<T>(query: Result<Query<T>, QueryRejection>) -> Result<T, WebError> {
    query
        .map(|Query(values)| values)
        .map_err(|_rejection| WebError::BadRequest {
            field: "query",
            reason: FieldReason::Query,
        })
}

/// Unwrap the path values of a route.
///
/// The function solves one problem: axum answers a path value that is not UTF-8 with its own
/// plain-text `400`. The handlers take the rejection as a value, so one mapping gives the `404`
/// for a bad text and for a bad encoding. The answer never holds the path text.
fn path_values<T>(path: Result<Path<T>, PathRejection>) -> Result<T, WebError> {
    path.map(|Path(values)| values)
        .map_err(|_rejection| not_found(Missing::MalformedMachine))
}

/// Read the route `/machines/{code}`.
///
/// # Errors
///
/// Returns `404 E-WEB-005` when the rejection or the code is bad.
pub(crate) fn path_code(
    path: Result<Path<String>, PathRejection>,
) -> Result<MachineCode, WebError> {
    machine_code(&path_values(path)?)
}

/// Read the route `/machines/by-id/{uuid}`.
///
/// # Errors
///
/// Returns `404 E-WEB-005` when the rejection or the identifier is bad.
pub(crate) fn path_machine_id(
    path: Result<Path<String>, PathRejection>,
) -> Result<MachineId, WebError> {
    machine_id(&path_values(path)?)
}

/// Return true when the rejection says that the path value `name` is not UTF-8 after decoding.
fn is_invalid_utf8_in(rejection: &PathRejection, name: &str) -> bool {
    let PathRejection::FailedToDeserializePathParams(failed) = rejection else {
        return false;
    };
    matches!(failed.kind(), ErrorKind::InvalidUtf8InPathParam { key } if key == name)
}

/// Read the route `/machines/{code}/units/{path}`. The code comes first.
///
/// A `{path}` that is not UTF-8 gives `E-PLT-017`, as a malformed text does (`api.md` section 1).
/// The rejection of axum holds the name of the value and not its bytes. So the error shows the
/// replacement character as the lossy text. The rejection holds no other value either, so the
/// code is not known in that case. A malformed code and a `{path}` that is not UTF-8 give
/// `E-PLT-017`. A code that is not UTF-8 still gives `404 E-WEB-005`.
///
/// # Errors
///
/// Returns `404 E-WEB-005` for a bad code or another rejection, else the error of `unit_path`.
pub(crate) fn path_code_unit(
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<(MachineCode, UnitPath), WebError> {
    if path
        .as_ref()
        .err()
        .is_some_and(|rejection| is_invalid_utf8_in(rejection, UNIT_PATH_VALUE))
    {
        return Err(WebError::App(
            PlantError::InvalidUnitPath {
                text: UNDECODABLE_TEXT.to_owned(),
            }
            .into(),
        ));
    }
    let (code, unit) = path_values(path)?;
    Ok((machine_code(&code)?, unit_path(&unit)?))
}

/// Read the route `/machines/{code}/types/{name}`. The code comes first.
///
/// A `{name}` that is not UTF-8 gives `Missing::MalformedType`. The rejection of axum holds the key
/// and no other value, so the code of that request is not known.
///
/// # Errors
///
/// Returns `404 E-WEB-005` for a bad code, a bad name, or a rejection.
pub(crate) fn path_code_type(
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<(MachineCode, TypeName), WebError> {
    if path
        .as_ref()
        .err()
        .is_some_and(|rejection| is_invalid_utf8_in(rejection, TYPE_NAME_VALUE))
    {
        return Err(not_found(Missing::MalformedType));
    }
    let (code, name) = path_values(path)?;
    Ok((machine_code(&code)?, type_name(&name)?))
}

#[cfg(test)]
mod tests;
