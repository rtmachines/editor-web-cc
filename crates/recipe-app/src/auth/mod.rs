//! Current user, request data, and the operation catalog (design D6 and D7).

mod authorize;
#[cfg(test)]
mod first_statement;
#[cfg(test)]
mod matrix;
#[cfg(test)]
mod matrix_doc;
mod operation;
mod user;

pub use authorize::authorize;
pub use operation::{Operation, Requirement};
pub use user::{CurrentUser, RequestMeta};
