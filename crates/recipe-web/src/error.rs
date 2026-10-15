//! `WebError`, the error of the HTTP layer (`reliability.md` section 1.1, `api.md` section 5).
//!
//! A handler returns `Err(WebError)`. The layer [`render_errors`] turns it into the answer.

use std::fmt;
use std::sync::Arc;

use axum::extract::rejection::FormRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use recipe_app::error::AppError;
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;

mod answer;
mod layer;
mod log;
mod plant;
mod reason;
mod retry;
mod status;
mod values;
mod view;

pub(crate) use layer::render_errors;
pub use reason::FieldReason;
pub(crate) use retry::Retry;
pub(crate) use view::ErrorView;

/// Bytes in one KiB.
const KIB: usize = 1024;
/// Bytes in one MiB.
const MIB: usize = 1024 * 1024;

/// A body limit in bytes. `Display` writes the unit of `reliability.md` section 7.11.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteLimit(usize);

impl From<usize> for ByteLimit {
    fn from(bytes: usize) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for ByteLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0;
        if bytes > 0 && bytes.is_multiple_of(MIB) {
            write!(f, "{} MiB", bytes / MIB)
        } else if bytes > 0 && bytes.is_multiple_of(KIB) {
            write!(f, "{} KiB", bytes / KIB)
        } else {
            write!(f, "{bytes} B")
        }
    }
}

/// The input of a member row that a field error names (`WebError::BadRow`).
///
/// The type solves one problem: the error line of a row must name the row and link to the right
/// input. A row has two inputs, so the error holds which one failed. The label comes from the key
/// `udt-member-name` or `udt-member-type`. The link goes to `#members-<row>-name` or
/// `#members-<row>-type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowField {
    /// The name of the member.
    Name,
    /// The type of the member.
    Type,
}

impl RowField {
    /// Return the key of the label in the bundles. The label takes the number of the row.
    pub(crate) const fn label_key(self) -> &'static str {
        match self {
            Self::Name => "udt-member-name",
            Self::Type => "udt-member-type",
        }
    }

    /// Return the last part of the `id` of the input: `name` or `type`.
    pub(crate) const fn id_suffix(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Type => "type",
        }
    }
}

impl fmt::Display for RowField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id_suffix())
    }
}

/// The error of a handler or a layer of `recipe-web`.
///
/// The type solves one problem: one error type maps to the HTTP answer, so no handler builds an
/// error page by hand. `Display` gives the English message of `reliability.md` section 1.2
/// without the sentence "Reference: {request_id}.". No variant carries a secret.
#[derive(Debug, thiserror::Error)]
pub enum WebError {
    /// An error of a use case.
    #[error(transparent)]
    App(#[from] AppError),
    /// The body is larger than the limit of its route (`E-WEB-003`).
    #[error("The request is larger than {limit}.")]
    PayloadTooLarge {
        /// The limit of the route.
        limit: ByteLimit,
    },
    /// A request field is missing or not valid (`E-WEB-008`).
    #[error("Field {field} is not valid: {reason}.")]
    BadRequest {
        /// The name of the field. It is a fixed name in the code, never typed text.
        field: &'static str,
        /// Why the field is not valid. The page shows the text of its key in the UI language.
        reason: FieldReason,
    },
    /// A field of one member row is not valid (`E-WEB-008`).
    ///
    /// The error line shows the label of the field with the number of the row. It links to the
    /// input of that row.
    #[error("Field {field} of member row {row} is not valid: {reason}.")]
    BadRow {
        /// The index of the row, counted from 0, as in `members[<row>].name`.
        row: u16,
        /// The input of the row that is not valid.
        field: RowField,
        /// Why the input is not valid. The page shows the text of its key in the UI language.
        reason: FieldReason,
    },
    /// The CSRF token or the origin of a mutating request is not valid (`E-WEB-007`).
    #[error("The form is no longer valid. Reload the page and try again.")]
    CsrfRejected,
    /// The system is in read-only mode (`E-WEB-006`).
    #[error(
        "The system is in read-only mode because disk space is low. Contact the administrator."
    )]
    ReadOnlyMode,
    /// A defect of the server, for example a route without an operation (`E-WEB-010`).
    #[error("An internal error occurred. Contact the administrator.")]
    Internal {
        /// The id of the request that failed.
        request_id: RequestId,
    },
}

impl WebError {
    /// Return the stable code of the error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::App(error) => error.code(),
            Self::PayloadTooLarge { .. } => ErrorCode::WEB_003,
            Self::BadRequest { .. } | Self::BadRow { .. } => ErrorCode::WEB_008,
            Self::CsrfRejected => ErrorCode::WEB_007,
            Self::ReadOnlyMode => ErrorCode::WEB_006,
            Self::Internal { .. } => ErrorCode::WEB_010,
        }
    }
}

impl From<FormRejection> for WebError {
    /// A form that cannot be read is a field error: a missing field, an unknown field, or a
    /// content type that is not `application/x-www-form-urlencoded`.
    fn from(_rejection: FormRejection) -> Self {
        Self::BadRequest {
            field: "form",
            reason: FieldReason::Form,
        }
    }
}

impl IntoResponse for WebError {
    /// Store the error in the response. The layer `render_errors` builds the answer.
    fn into_response(self) -> Response {
        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
        response.extensions_mut().insert(Arc::new(self));
        response
    }
}

#[cfg(test)]
pub(crate) use view::FieldLine;
#[cfg(test)]
mod fragment_tests;
#[cfg(test)]
mod plant_args_tests;
#[cfg(test)]
mod push_url_tests;
#[cfg(test)]
mod row_field_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod value_tests;
