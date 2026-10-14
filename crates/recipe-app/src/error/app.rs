//! The error of every use case (`reliability.md` section 1.1).
//!
//! `Display` gives the user message of `reliability.md` section 1.2.
//! It ends before the sentence "Reference: {request_id}.", because only the request knows the reference (design D4).

use std::fmt;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::ExportError;
use recipe_domain::identity::IdentityError;
use recipe_domain::plant::PlantError;
use recipe_domain::recipe::{DraftError, RecipeError, RecipeErrors, ReferenceNumber};

use super::missing::Missing;
use super::storage::StorageError;
use crate::auth::Operation;

/// The stage of a use case that ran out of time (`E-WEB-009`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeoutStage {
    /// The wait for the Argon2 permit.
    Argon2Permit,
    /// The snapshot validation.
    Validation,
    /// The export.
    Export,
    /// The whole request.
    Request,
}

impl fmt::Display for TimeoutStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Argon2Permit => "password hash permit",
            Self::Validation => "validation",
            Self::Export => "export",
            Self::Request => "request",
        })
    }
}

/// The kind of work that did not complete (`E-WEB-011`).
///
/// The work is a blocking task or the read of random bytes from the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStage {
    /// The Argon2id hash.
    PasswordHash,
    /// The snapshot validation.
    Validation,
    /// The rebuild and the hash of the export body.
    Export,
    /// The read of random bytes for a session token, a CSRF token, or a one-time password.
    Token,
}

impl fmt::Display for TaskStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PasswordHash => "password hash",
            Self::Validation => "validation",
            Self::Export => "export",
            Self::Token => "token generation",
        })
    }
}

/// The error that a use case returns.
///
/// The type solves one problem: `recipe-web` maps one error type to the HTTP answer.
/// Each variant owns one code, or it passes the code of the inner error.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// An error of the Plant Model context.
    #[error(transparent)]
    Plant(#[from] PlantError),
    /// An error of the Recipe Management context.
    #[error(transparent)]
    Recipe(#[from] RecipeError),
    /// A list of errors of the Recipe Management context.
    #[error(transparent)]
    Recipes(#[from] RecipeErrors),
    /// An error of the export.
    #[error(transparent)]
    Export(#[from] ExportError),
    /// An error of the Identity and Access context.
    #[error(transparent)]
    Identity(#[from] IdentityError),
    /// The role does not permit the operation (`E-WEB-002`).
    #[error("Your role does not permit the operation {operation}.")]
    Forbidden {
        /// The operation that the role may not run.
        operation: Operation,
    },
    /// The request has no session or an expired session (`E-WEB-004`).
    #[error("Log in to continue.")]
    Unauthenticated,
    /// A URL value names an entity that does not exist (`E-WEB-005`).
    #[error("{entity} does not exist.")]
    NotFound {
        /// The entity that does not exist.
        entity: Missing,
    },
    /// Another user changed the version first (`E-WEB-001`).
    #[error(
        "Another user changed version {number} of recipe {reference}. Reload the page and apply your change again."
    )]
    ConcurrentEdit {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
        /// The row version that the caller sent.
        expected: i64,
        /// The row version that the version holds.
        actual: i64,
    },
    /// A stage ran out of time (`E-WEB-009`).
    #[error("The request took too long and was stopped. Try again.")]
    Timeout {
        /// The stage that ran out of time.
        stage: TimeoutStage,
    },
    /// A blocking task did not complete (`E-WEB-011`).
    #[error("The operation {stage} did not complete. Try again.")]
    TaskFailed {
        /// The kind of blocking work.
        stage: TaskStage,
    },
    /// The system could not write the audit entry (`E-AUD-001`).
    #[error(
        "The change was not saved. The system could not record the audit entry. Contact the administrator."
    )]
    AuditWriteFailed {
        /// The storage error of the audit insert. The `WebError` mapping logs it.
        #[source]
        cause: StorageError,
    },
    /// An error of the storage ports.
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl AppError {
    /// Return the stable code of the error.
    ///
    /// A domain or storage variant gives the code of the inner error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Plant(error) => error.code(),
            Self::Recipe(error) => error.code(),
            Self::Recipes(errors) => errors.code(),
            Self::Export(error) => error.code(),
            Self::Identity(error) => error.code(),
            Self::Forbidden { .. } => ErrorCode::WEB_002,
            Self::Unauthenticated => ErrorCode::WEB_004,
            Self::NotFound { .. } => ErrorCode::WEB_005,
            Self::ConcurrentEdit { .. } => ErrorCode::WEB_001,
            Self::Timeout { .. } => ErrorCode::WEB_009,
            Self::TaskFailed { .. } => ErrorCode::WEB_011,
            Self::AuditWriteFailed { .. } => ErrorCode::AUD_001,
            Self::Storage(error) => error.code(),
        }
    }
}

/// Unwrap the inner error of a draft command (ADR-0027). The conversion adds no variant.
impl From<DraftError> for AppError {
    fn from(error: DraftError) -> Self {
        match error {
            DraftError::Recipe(inner) => Self::Recipe(inner),
            DraftError::Plant(inner) => Self::Plant(inner),
            DraftError::List(inner) => Self::Recipes(inner),
        }
    }
}

#[cfg(test)]
mod tests;
