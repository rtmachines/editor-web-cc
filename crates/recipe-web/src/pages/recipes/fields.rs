//! The field errors of one form or one query string (ADR-0041 (a)).
//!
//! The module solves one problem: a form answer must list every bad field at once. A reader of a
//! field returns `WebError::BadRequest`. [`FieldLines`] turns each of these errors into a line and
//! keeps the other errors, so the handler reads every field before it answers.

use recipe_app::ports::NonEmpty;

use crate::error::{FieldError, FieldReason, WebError};

/// The lines of the bad fields of one form, in the order of the form.
#[derive(Default)]
pub(crate) struct FieldLines(Vec<FieldError>);

impl FieldLines {
    /// Return the value of `result`, or `default` after a field error.
    ///
    /// A field error (`WebError::BadRequest`) becomes a line. The default value only lets the
    /// handler read the next field, because [`FieldLines::into_lines`] then returns the lines.
    ///
    /// # Errors
    ///
    /// Returns every error that is not a field error at once, for example `404 E-WEB-005`.
    pub(crate) fn take_or<T>(
        &mut self,
        result: Result<T, WebError>,
        default: T,
    ) -> Result<T, WebError> {
        match result {
            Ok(value) => Ok(value),
            Err(WebError::BadRequest { field, reason }) => {
                self.push(field, reason);
                Ok(default)
            }
            Err(other) => Err(other),
        }
    }

    /// Add the line `E-WEB-008` of the field `field`.
    pub(crate) fn push(&mut self, field: &'static str, reason: FieldReason) {
        self.0.push(FieldError::bad_request(field, reason));
    }

    /// Return the value of a parser, or `None` after the line `E-WEB-008` of `field`.
    ///
    /// The parser error holds the typed text, so the line keeps only the field and the reason.
    pub(crate) fn field<T, E>(
        &mut self,
        result: Result<T, E>,
        field: &'static str,
        reason: FieldReason,
    ) -> Option<T> {
        result.map_err(|_error| self.push(field, reason)).ok()
    }

    /// Return the lines, or `None` when every field is valid.
    pub(crate) fn into_lines(self) -> Option<NonEmpty<FieldError>> {
        NonEmpty::try_from(self.0).ok()
    }

    /// Return the input of the use case when every field is valid, else every line.
    ///
    /// `input` is `None` exactly when a field failed. The last arm only keeps the function total:
    /// it gives the line of the whole form.
    ///
    /// # Errors
    ///
    /// Returns the lines of the bad fields.
    pub(crate) fn finish<T>(self, input: Option<T>) -> Result<T, NonEmpty<FieldError>> {
        match (self.into_lines(), input) {
            (None, Some(input)) => Ok(input),
            (Some(lines), _input) => Err(lines),
            (None, None) => Err(NonEmpty::new(FieldError::bad_request(
                "form",
                FieldReason::Form,
            ))),
        }
    }

    /// Return `Ok` when every field is valid, else the error with every line and no form.
    ///
    /// # Errors
    ///
    /// Returns `WebError::Fields` (`E-WEB-008`) with each line of a bad field.
    pub(crate) fn check(self) -> Result<(), WebError> {
        self.into_lines().map_or(Ok(()), |lines| {
            Err(WebError::Fields {
                lines,
                form: None,
                failure: None,
            })
        })
    }
}
