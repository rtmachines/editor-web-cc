//! The fault of a process limit (design D9, ADR-0025).

use std::fmt;

use crate::plant::LeafPath;

/// The reason why a limit row does not fit its type tree.
///
/// The enum has no `code()`. The draft wraps the fault into `E-RCP-017`, `E-RCP-019`,
/// `E-RCP-006`, or `E-RCP-018` (ADR-0025).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LimitFault {
    /// The tree has no leaf at the path.
    NoLeaf {
        /// The path of the row.
        leaf: LeafPath,
    },
    /// The leaf class does not accept a field of the row.
    NotApplicable {
        /// The path of the row.
        leaf: LeafPath,
    },
    /// A bound or a length is outside the range of the type.
    OutsideTypeRange {
        /// The path of the row.
        leaf: LeafPath,
    },
    /// The lower bound is greater than the upper bound.
    MinGreaterThanMax {
        /// The path of the row.
        leaf: LeafPath,
        /// The lower bound as message text.
        min: String,
        /// The upper bound as message text.
        max: String,
    },
}

impl LimitFault {
    /// Return the path of the row that failed.
    #[must_use]
    pub const fn leaf(&self) -> &LeafPath {
        match self {
            Self::NoLeaf { leaf }
            | Self::NotApplicable { leaf }
            | Self::OutsideTypeRange { leaf }
            | Self::MinGreaterThanMax { leaf, .. } => leaf,
        }
    }
}

impl fmt::Display for LimitFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLeaf { leaf } => write!(f, "The type has no leaf at \"{leaf}\"."),
            Self::NotApplicable { leaf } => {
                write!(f, "The leaf \"{leaf}\" does not accept this limit.")
            }
            Self::OutsideTypeRange { leaf } => {
                write!(
                    f,
                    "The limit of the leaf \"{leaf}\" is outside the type range."
                )
            }
            Self::MinGreaterThanMax { leaf, min, max } => write!(
                f,
                "The lower bound {min} of the leaf \"{leaf}\" is greater than the upper bound {max}."
            ),
        }
    }
}
