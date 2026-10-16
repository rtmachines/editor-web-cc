//! The definition fields of a parameter form: the reader and its lines (ADR-0041 (a), design D6).
//!
//! The module solves one problem: a form answer lists every bad field at once. The reader parses
//! each definition field, keeps a line for each bad one, and still parses the next field. A value
//! object with its own code gives that code, for example `E-PLT-017` for a unit path.

use recipe_domain::plant::{S7Address, TypeExpr, UnitPath};
use recipe_domain::recipe::{Ordinal, ParameterTag};
use recipe_domain::text::{LongText, Text};

use super::fields::SentFields;
use crate::error::{FieldError, FieldReason};

/// The definition fields of a parameter form after their check.
#[derive(Clone, Debug)]
pub(crate) struct Definition {
    /// The `row_version` that the form sent.
    pub(crate) row_version: i64,
    /// The path of the unit of the parameter.
    pub(crate) unit_path: UnitPath,
    /// The tag of the parameter.
    pub(crate) tag: ParameterTag,
    /// The English name.
    pub(crate) name_en: Text,
    /// The English description.
    pub(crate) description_en: LongText,
    /// The type of the parameter.
    pub(crate) type_expr: TypeExpr,
    /// The S7 address, or `None` for an empty field.
    pub(crate) s7_address: Option<S7Address>,
    /// The order of the parameter in its unit.
    pub(crate) ordinal: Ordinal,
}

/// The result of the check of the definition fields.
pub(crate) struct Checked {
    /// The fields, or `None` when a field is bad.
    pub(crate) definition: Option<Definition>,
    /// The type of the field `type`, or `None` when it does not parse.
    pub(crate) type_expr: Option<TypeExpr>,
    /// One line for each bad field, in the order of the form.
    pub(crate) lines: Vec<FieldError>,
}

/// Return the trimmed text of the plain field `name`, or an empty text when the form lacks it.
fn trimmed<'a>(sent: &'a SentFields, name: &str) -> &'a str {
    sent.text(name).unwrap_or_default().trim()
}

/// Read a whole number in the canonical form: ASCII digits only and no zero in front.
fn whole_number<T: std::str::FromStr>(text: &str) -> Option<T> {
    let canonical = !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical.then(|| text.parse().ok()).flatten()
}

/// Read the hidden field `row_version`.
///
/// # Errors
///
/// Returns the line `E-WEB-008` with the reason `Reload` for a missing or malformed text.
pub(crate) fn row_version(sent: &SentFields) -> Result<i64, FieldError> {
    whole_number(trimmed(sent, "row_version"))
        .ok_or_else(|| FieldError::bad_request("row_version", FieldReason::Reload))
}

/// The lines of a form, collected while the reader parses each field.
#[derive(Default)]
struct Lines(Vec<FieldError>);

impl Lines {
    /// Return the value of `result`, or `None` after the line `E-WEB-008` of `field`.
    fn field<T, E>(
        &mut self,
        result: Result<T, E>,
        field: &'static str,
        reason: FieldReason,
    ) -> Option<T> {
        result
            .map_err(|_error| self.0.push(FieldError::bad_request(field, reason)))
            .ok()
    }

    /// Return the value of `result`, or `None` after the line of its own code under `input`.
    fn plant<T>(
        &mut self,
        result: Result<T, recipe_domain::plant::PlantError>,
        input: &str,
    ) -> Option<T> {
        result
            .map_err(|error| {
                self.0
                    .push(FieldError::plant(&error, Some(input.to_owned())))
            })
            .ok()
    }

    /// Keep a line of a field that the caller checked.
    fn push(&mut self, line: FieldError) {
        self.0.push(line);
    }
}

/// Read the S7 address. An empty field gives `None`.
fn s7_address(text: &str) -> Result<Option<S7Address>, ()> {
    if text.is_empty() {
        return Ok(None);
    }
    S7Address::parse(text).map(Some).map_err(|_reason| ())
}

/// Read the order of the parameter in its unit: a whole number that `Ordinal` accepts.
fn ordinal(text: &str) -> Result<Ordinal, ()> {
    let number: u32 = whole_number(text).ok_or(())?;
    Ordinal::parse(number).map_err(|_reason| ())
}

/// Check every definition field of a parameter form (ADR-0041 (a)).
pub(crate) fn check(sent: &SentFields) -> Checked {
    let mut lines = Lines::default();
    let version = row_version(sent).map_err(|line| lines.push(line)).ok();
    let unit_path = lines.plant(UnitPath::parse(trimmed(sent, "unit_path")), "unit_path");
    let tag = lines.field(
        ParameterTag::parse(trimmed(sent, "tag")),
        "tag",
        FieldReason::Tag,
    );
    let name_en = lines.field(
        Text::parse(sent.text("name_en").unwrap_or_default()),
        "name_en",
        FieldReason::Name,
    );
    let description_en = lines.field(
        LongText::parse(sent.text("description_en").unwrap_or_default()),
        "description_en",
        FieldReason::Description,
    );
    let type_expr = lines.plant(TypeExpr::parse(trimmed(sent, "type")), "type");
    let s7 = lines.field(
        s7_address(trimmed(sent, "s7_address")),
        "s7_address",
        FieldReason::S7Address,
    );
    let order = lines.field(
        ordinal(trimmed(sent, "ordinal")),
        "ordinal",
        FieldReason::Ordinal,
    );
    let definition = (|| {
        Some(Definition {
            row_version: version?,
            unit_path: unit_path?,
            tag: tag?,
            name_en: name_en?,
            description_en: description_en?,
            type_expr: type_expr.clone()?,
            s7_address: s7?,
            ordinal: order?,
        })
    })();
    Checked {
        definition,
        type_expr,
        lines: lines.0,
    }
}
