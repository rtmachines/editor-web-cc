//! Samples of `WebError`.
//!
//! A delegate variant passes the code of an inner error. The registry test checks that code with the
//! module of the inner type, so a delegate has no sample here.

use recipe_domain::audit::RequestId;
use recipe_web::{ByteLimit, FieldReason, RowField, WebError};
use uuid::Uuid;

use super::sample::Sample;

/// Return the name `Type::Variant` of an own variant, or `None` for a delegate.
///
/// `App`, `Fields`, and `Conflict` pass the code of an `AppError` or of the first line.
/// The `match` has no wildcard, so a new variant stops the build.
fn name_of(error: &WebError) -> Option<&'static str> {
    match error {
        WebError::App(_) | WebError::Fields { .. } | WebError::Conflict { .. } => None,
        WebError::PayloadTooLarge { .. } => Some("WebError::PayloadTooLarge"),
        WebError::BadRequest { .. } => Some("WebError::BadRequest"),
        WebError::BadQuery { .. } => Some("WebError::BadQuery"),
        WebError::BadRow { .. } => Some("WebError::BadRow"),
        WebError::CsrfRejected => Some("WebError::CsrfRejected"),
        WebError::ReadOnlyMode => Some("WebError::ReadOnlyMode"),
        WebError::MissingBasicCredentials => Some("WebError::MissingBasicCredentials"),
        WebError::Internal { .. } => Some("WebError::Internal"),
    }
}

/// Return one sample for each own variant of `WebError`.
pub fn samples() -> Vec<Sample> {
    let errors = [
        WebError::PayloadTooLarge {
            limit: ByteLimit::from(1024),
        },
        WebError::BadRequest {
            field: "form",
            reason: FieldReason::Form,
        },
        WebError::BadQuery {
            field: "form",
            reason: FieldReason::Form,
        },
        WebError::BadRow {
            row: 0,
            field: RowField::Name,
            reason: FieldReason::Form,
        },
        WebError::CsrfRejected,
        WebError::ReadOnlyMode,
        WebError::MissingBasicCredentials,
        WebError::Internal {
            request_id: RequestId::from(Uuid::nil()),
        },
    ];
    errors
        .iter()
        .filter_map(|error| {
            name_of(error).map(|name| Sample {
                name,
                code: error.code(),
            })
        })
        .collect()
}
