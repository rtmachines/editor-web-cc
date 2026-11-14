//! Samples of `CheckResultFault`, one for each variant.

use std::io;

use recipe_app::ports::{CheckResultFault, ScheduledCheck};

use super::sample::Sample;

/// Return the name `Type::Variant` of a sample. The `match` has no wildcard, so a new variant stops the build.
fn name_of(error: &CheckResultFault) -> &'static str {
    match error {
        CheckResultFault::Unreadable { .. } => "CheckResultFault::Unreadable",
        CheckResultFault::Malformed { .. } => "CheckResultFault::Malformed",
    }
}

/// Return one sample for each variant of `CheckResultFault`.
pub fn samples() -> Vec<Sample> {
    let errors = [
        CheckResultFault::Unreadable {
            check: ScheduledCheck::RestoreTest,
            source: io::Error::from(io::ErrorKind::NotFound),
        },
        CheckResultFault::Malformed {
            check: ScheduledCheck::BaseBackup,
        },
    ];
    errors
        .iter()
        .map(|error| Sample {
            name: name_of(error),
            code: error.code(),
        })
        .collect()
}
