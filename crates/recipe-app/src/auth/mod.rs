//! Current user, request data, and the operation catalog (design D6 and D7).

mod authorize;
#[cfg(test)]
mod first_statement;
#[cfg(test)]
mod matrix;
#[cfg(test)]
mod matrix_doc;
mod operation;
#[cfg(test)]
mod purpose_tests;
mod user;

pub use authorize::authorize;
pub use operation::{
    FormPurpose, MachineFormPurpose, Operation, ParameterFormPurpose, RecipeFormPurpose,
    Requirement, UdtFormPurpose, UnitFormPurpose, VersionFormPurpose,
};
pub use user::{CurrentUser, RequestMeta};
