//! The items of a failed snapshot check (design D12, ADR-0028).

use std::fmt;

use crate::error_codes::ErrorCode;
use crate::plant::{PlantError, UnitId};
use crate::recipe::{ParameterTag, RecipeError};

/// The error of one snapshot item.
///
/// The type solves one problem: an item holds an error of the Plant Model or of Recipe
/// Management. The item owns no code of its own.
#[derive(Debug)]
pub enum SnapshotError {
    /// An error of the Plant Model context.
    Plant(PlantError),
    /// An error of the Recipe Management context.
    Recipe(RecipeError),
}

impl SnapshotError {
    /// Return the code of the inner error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Plant(error) => error.code(),
            Self::Recipe(error) => error.code(),
        }
    }
}

impl From<PlantError> for SnapshotError {
    fn from(error: PlantError) -> Self {
        Self::Plant(error)
    }
}

impl From<RecipeError> for SnapshotError {
    fn from(error: RecipeError) -> Self {
        Self::Recipe(error)
    }
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plant(error) => write!(f, "{error}"),
            Self::Recipe(error) => write!(f, "{error}"),
        }
    }
}

/// One problem that the snapshot check found.
///
/// The type solves one problem: the user sees every problem of a version in one list. An item
/// has no code of its own. [`SnapshotItem::code`] returns the code of the inner error.
#[derive(Debug)]
pub enum SnapshotItem {
    /// A problem of one parameter.
    Parameter {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The problem.
        error: SnapshotError,
    },
    /// A problem of the version, for example an archived machine or a deleted recipe.
    Version {
        /// The problem.
        error: SnapshotError,
    },
    /// A parameter uses a unit that the machine does not know.
    ///
    /// The foreign key and the trigger of `data-model.md` section 4.3 make this impossible.
    /// The item protects against corrupt storage.
    UnknownUnit {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The identifier of the unit.
        unit: UnitId,
    },
}

impl SnapshotItem {
    /// Return the code of the inner error. An unknown unit has the code `E-PLT-020`.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Parameter { error, .. } | Self::Version { error } => error.code(),
            Self::UnknownUnit { .. } => ErrorCode::PLT_020,
        }
    }
}

impl fmt::Display for SnapshotItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parameter { error, .. } | Self::Version { error } => write!(f, "{error}"),
            Self::UnknownUnit { tag, unit } => write!(
                f,
                "Parameter {tag} uses the unit {unit}, which does not exist on this machine."
            ),
        }
    }
}
