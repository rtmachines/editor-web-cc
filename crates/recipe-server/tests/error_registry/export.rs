//! Samples of `ExportError`, one for each variant.

use recipe_domain::export::ExportError;
use recipe_domain::plant::UnitPath;
use recipe_domain::recipe::ReferenceNumber;

use super::sample::Sample;

/// Return the name `Type::Variant` of a sample. The `match` has no wildcard, so a new variant stops the build.
fn name_of(error: &ExportError) -> &'static str {
    match error {
        ExportError::NoApprovedVersion { .. } => "ExportError::NoApprovedVersion",
        ExportError::VersionNotExportable { .. } => "ExportError::VersionNotExportable",
        ExportError::ContentHashMismatch { .. } => "ExportError::ContentHashMismatch",
        ExportError::EmptyScope { .. } => "ExportError::EmptyScope",
        ExportError::Serialize(_) => "ExportError::Serialize",
    }
}

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("RCP-1").unwrap()
}

/// Return one sample for each variant of `ExportError`.
pub fn samples() -> Vec<Sample> {
    let errors = [
        ExportError::NoApprovedVersion {
            reference: reference(),
        },
        ExportError::VersionNotExportable {
            reference: reference(),
            number: 1,
        },
        ExportError::ContentHashMismatch {
            reference: reference(),
            number: 1,
        },
        ExportError::EmptyScope {
            reference: reference(),
            number: 1,
            path: UnitPath::parse("U1").unwrap(),
        },
        ExportError::Serialize(serde_json::from_str::<u8>("").unwrap_err()),
    ];
    errors
        .iter()
        .map(|error| Sample {
            name: name_of(error),
            code: error.code(),
        })
        .collect()
}
