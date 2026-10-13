//! Audit entries of the use cases (design D9).

#[cfg(test)]
mod coverage;
mod diff;
mod version;

pub use diff::{StateDiff, definition_diff, limit_diff, rebind_diff, translation_diff, value_diff};
pub(crate) use version::version_entry;
