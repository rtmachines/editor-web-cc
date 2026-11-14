//! One sample value of an error type, reduced to what the registry test compares.

use recipe_domain::error_codes::ErrorCode;

/// The name of a variant and the code that one value of it returns.
///
/// The name has the form `Type::Variant`, like the owner cell of a row in `reliability.md` section 1.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sample {
    pub name: &'static str,
    pub code: ErrorCode,
}
