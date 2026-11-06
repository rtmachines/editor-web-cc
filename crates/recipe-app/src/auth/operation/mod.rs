//! The operation catalog (design D7).
//!
//! The catalog solves one problem: a use case without a row in the permission matrix must not compile.
//! The table in `catalog` generates `Operation::requirement` with no wildcard arm, so a new
//! variant forces a new matrix row.

use std::fmt;

mod catalog;
mod purpose;
mod requirement;

pub use catalog::Operation;
pub use purpose::{
    FormPurpose, MachineFormPurpose, ParameterFormPurpose, RecipeFormPurpose, UdtFormPurpose,
    UnitFormPurpose, VersionFormPurpose,
};
pub use requirement::Requirement;

impl Operation {
    /// Return true when the operation still runs in read-only mode (Assumption A24).
    ///
    /// Every query runs. Of the commands, only `Login`, `Logout`, and the two exports run.
    #[must_use]
    pub const fn runs_in_read_only(self) -> bool {
        match self {
            Self::Login
            | Self::Logout
            | Self::ExportApprovedVersion
            | Self::ExportVersionByNumber => true,
            other => !other.is_command(),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests;
