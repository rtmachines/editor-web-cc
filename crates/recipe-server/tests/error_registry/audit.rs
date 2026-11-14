//! Samples of `AuditError`, one for each variant.

use recipe_app::error::AuditError;

use super::sample::Sample;

/// Return the name `Type::Variant` of a sample. The `match` has no wildcard, so a new variant stops the build.
fn name_of(error: &AuditError) -> &'static str {
    match error {
        AuditError::ChainBroken { .. } => "AuditError::ChainBroken",
    }
}

/// Return one sample for each variant of `AuditError`.
pub fn samples() -> Vec<Sample> {
    let errors = [AuditError::ChainBroken { chain_seq: 7 }];
    errors
        .iter()
        .map(|error| Sample {
            name: name_of(error),
            code: error.code(),
        })
        .collect()
}
