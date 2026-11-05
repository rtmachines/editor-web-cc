//! The error type of the Export context.

use crate::error_codes::ErrorCode;
use crate::plant::UnitPath;
use crate::recipe::ReferenceNumber;

/// Error of the Export context (`reliability.md` section 1.1).
///
/// The enum has one variant for each code in the range `E-EXP-001` to `E-EXP-005`.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The recipe has no approved version (`E-EXP-001`).
    #[error("Recipe {reference} has no approved version.")]
    NoApprovedVersion {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
    },
    /// The version is a draft or was discarded (`E-EXP-003`).
    #[error(
        "Version {number} of recipe {reference} is a draft or was discarded. Only approved and retired versions can be exported."
    )]
    VersionNotExportable {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
    },
    /// The stored hash does not match the content of the version (`E-EXP-002`).
    #[error(
        "Version {number} of recipe {reference} does not match its stored hash. The export is refused. Contact the administrator."
    )]
    ContentHashMismatch {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
    },
    /// The unit filter of a scoped export selects no parameter (`E-EXP-005`).
    #[error("Version {number} of recipe {reference} has no parameter in unit {path} or below it.")]
    EmptyScope {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
        /// The unit of the filter.
        path: UnitPath,
    },
    /// The serializer cannot write the export document (`E-EXP-004`).
    #[error("The server cannot create the export document. Contact the administrator.")]
    Serialize(#[from] serde_json::Error),
}

impl ExportError {
    /// Return the stable code of the error (`reliability.md` section 1.2).
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::NoApprovedVersion { .. } => ErrorCode::EXP_001,
            Self::ContentHashMismatch { .. } => ErrorCode::EXP_002,
            Self::VersionNotExportable { .. } => ErrorCode::EXP_003,
            Self::EmptyScope { .. } => ErrorCode::EXP_005,
            Self::Serialize(_) => ErrorCode::EXP_004,
        }
    }
}

#[cfg(test)]
mod tests;
