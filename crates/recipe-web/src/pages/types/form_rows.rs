//! The reader of the body of the UDT form (design D8, ADR-0037 k).
//!
//! The module solves one problem: the member rows have keys such as `members[3].name`, and no
//! serde struct can name them. The reader takes the pairs of the body and returns the name, the
//! flag `add`, and the rows in the order of their index. It accepts the keys `csrf_token`, `name`,
//! `add`, `members[<i>].name`, and `members[<i>].type`. Any other key is a form error.
//!
//! An index is a decimal number of one to four digits with no leading zero, except the index `0`.
//! A row that the form numbers `type.max_members` or higher means that the form holds more rows
//! than the limit. That index gives the error of the member count.

use std::collections::BTreeMap;

use recipe_domain::plant::limits::MAX_MEMBERS;

use crate::error::{FieldReason, RowField, WebError};
use crate::pages::path_values::flag;

/// The key of the session token. The CSRF layer checked it before the handler runs.
const TOKEN_KEY: &str = "csrf_token";
/// The key of the name of the struct.
const NAME_KEY: &str = "name";
/// The key of the button "Add a member".
const ADD_KEY: &str = "add";
/// The start of the key of a member field.
const ROW_PREFIX: &str = "members[";
/// The text between the index and the field name in the key of a member field.
const ROW_SEPARATOR: &str = "].";
/// The largest number of digits of an index.
const INDEX_DIGITS: usize = 4;

/// One member row as the form sent it. The texts are not checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SentRow {
    /// The text of the name input. A key that the body lacks gives an empty text.
    pub(super) name: String,
    /// The text of the type input. A key that the body lacks gives an empty text.
    pub(super) type_text: String,
}

/// The fields of a UDT form body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SentForm {
    /// The name of the struct, or `None` when the body has no `name`.
    pub(super) name: Option<String>,
    /// True when the user pressed the button "Add a member".
    pub(super) add: bool,
    /// The rows in the order of their index. The position in this list is the new row number.
    pub(super) rows: Vec<SentRow>,
}

/// The two inputs of one row. A key that the body lacks leaves its slot empty.
#[derive(Default)]
struct RowParts {
    name: Option<String>,
    type_text: Option<String>,
}

/// Build the error of a body that has a key twice, an unknown key, or a malformed index.
const fn form_error() -> WebError {
    WebError::BadRequest {
        field: "form",
        reason: FieldReason::Form,
    }
}

/// Store `value` in `slot`. A slot that holds a value already means a repeated key.
fn set_once(slot: &mut Option<String>, value: &str) -> Result<(), WebError> {
    if slot.is_some() {
        return Err(form_error());
    }
    *slot = Some(value.to_owned());
    Ok(())
}

/// Read the index of a member key.
///
/// The index is canonical when it has one to four digits and no leading zero, except `0`.
fn parse_index(text: &str) -> Result<usize, WebError> {
    let canonical = (1..=INDEX_DIGITS).contains(&text.len())
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    if !canonical {
        return Err(form_error());
    }
    let index = text.parse::<usize>().map_err(|_error| form_error())?;
    if index >= MAX_MEMBERS {
        return Err(WebError::BadRequest {
            field: "members",
            reason: FieldReason::MemberCount,
        });
    }
    Ok(index)
}

/// Split the key `members[<i>].name` or `members[<i>].type` into its index and its field.
fn parse_row_key(key: &str) -> Result<(usize, RowField), WebError> {
    let rest = key.strip_prefix(ROW_PREFIX).ok_or_else(form_error)?;
    let (index, field) = rest.split_once(ROW_SEPARATOR).ok_or_else(form_error)?;
    let field = match field {
        "name" => RowField::Name,
        "type" => RowField::Type,
        _other => return Err(form_error()),
    };
    Ok((parse_index(index)?, field))
}

/// Put the value of a member key into its row.
fn place_row(rows: &mut BTreeMap<usize, RowParts>, key: &str, value: &str) -> Result<(), WebError> {
    let (index, field) = parse_row_key(key)?;
    let parts = rows.entry(index).or_default();
    let slot = match field {
        RowField::Name => &mut parts.name,
        RowField::Type => &mut parts.type_text,
    };
    set_once(slot, value)
}

/// Read the pairs of the body of a UDT form.
///
/// A key twice, an unknown key, and a malformed index give `422 E-WEB-008` with the reason
/// `Form`. An index of 1024 or more gives the reason `MemberCount`. A value of `add` other than
/// `1` gives the reason `Flag`.
///
/// # Errors
///
/// Returns the `WebError::BadRequest` described above.
pub(super) fn read(pairs: &[(String, String)]) -> Result<SentForm, WebError> {
    let mut token = None;
    let mut name = None;
    let mut add = None;
    let mut rows: BTreeMap<usize, RowParts> = BTreeMap::new();
    for (key, value) in pairs {
        match key.as_str() {
            TOKEN_KEY => set_once(&mut token, value)?,
            NAME_KEY => set_once(&mut name, value)?,
            ADD_KEY => set_once(&mut add, value)?,
            other => place_row(&mut rows, other, value)?,
        }
    }
    let rows = rows
        .into_values()
        .map(|parts| SentRow {
            name: parts.name.unwrap_or_default(),
            type_text: parts.type_text.unwrap_or_default(),
        })
        .collect();
    Ok(SentForm {
        name,
        add: flag("add", add.as_deref())?,
        rows,
    })
}
