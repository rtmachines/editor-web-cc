//! Mapping of an optimistic outcome and of a storage error to `AppError` (design D4).
//!
//! The constraint names come from `reliability.md` section 1.1 and `data-model.md` sections 3 and 6.
//! The adapters of `recipe-db` use the same constants.

use recipe_domain::recipe::{Optimistic, ReferenceNumber};

use super::{AppError, StorageError};

/// Unique index of the live machine codes.
pub const MACHINE_CODE_LIVE_UNIQUE: &str = "machine_code_live_unique";
/// Unique constraint of the recipe reference numbers.
pub const RECIPE_REFERENCE_UNIQUE: &str = "recipe_reference_unique";
/// Unique constraint of the unit paths of one machine.
pub const UNIT_PATH_UNIQUE: &str = "unit_path_unique";
/// Unique constraint of the user logins.
pub const APP_USER_LOGIN_UNIQUE: &str = "app_user_login_unique";
/// Unique constraint of the parameter tags of one version.
pub const PARAMETER_TAG_UNIQUE: &str = "parameter_tag_unique";
/// Unique constraint of the named type revisions of one machine.
pub const PLC_TYPE_NAMED_UNIQUE: &str = "plc_type_named_unique";
/// Unique constraint of the member names of one struct.
pub const PLC_TYPE_MEMBER_NAME_UNIQUE: &str = "plc_type_member_name_unique";
/// Unique index of the one live draft of a recipe.
pub const RECIPE_VERSION_ONE_LIVE_DRAFT: &str = "recipe_version_one_live_draft";
/// Foreign key from a unit to its parent unit.
pub const UNIT_PARENT_EXISTS: &str = "unit_parent_exists";

/// Turn an optimistic outcome into a result.
///
/// The function solves one problem: one place maps a stale row version to `ConcurrentEdit` (`E-WEB-001`).
///
/// # Errors
///
/// Returns [`AppError::ConcurrentEdit`] with both row versions on `Optimistic::Conflict`.
pub fn conflict<T>(
    outcome: Optimistic<T>,
    reference: ReferenceNumber,
    number: u32,
) -> Result<T, AppError> {
    match outcome {
        Optimistic::Applied(value) => Ok(value),
        Optimistic::Conflict { expected, actual } => Err(AppError::ConcurrentEdit {
            reference,
            number,
            expected,
            actual,
        }),
    }
}

/// Map a storage error to the domain error of the command.
///
/// The function solves one problem: the database is the second line behind the domain check.
/// A `Duplicate` or `MissingReference` error calls `known` with the constraint name.
/// `known` returns the domain error for a name that the command knows, because the command knows the field values.
/// Every other case stays [`AppError::Storage`] with the original code.
/// An unknown name therefore gives `E-DB-006` or `E-DB-007`.
#[must_use]
pub fn map_storage(error: StorageError, known: impl FnOnce(&str) -> Option<AppError>) -> AppError {
    let mapped = match &error {
        StorageError::Duplicate { constraint } | StorageError::MissingReference { constraint } => {
            known(constraint)
        }
        _ => None,
    };
    mapped.unwrap_or(AppError::Storage(error))
}

#[cfg(test)]
mod tests;
