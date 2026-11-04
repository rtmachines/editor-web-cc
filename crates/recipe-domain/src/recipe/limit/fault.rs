//! The fault of a process limit (design D9, ADR-0025).

use std::fmt;

use crate::plant::{LeafPath, Primitive};
use crate::recipe::PrecisionField;

/// The reason why a limit row does not fit its type tree.
///
/// The enum has no `code()`. The draft wraps the fault into `E-RCP-017`, `E-RCP-019`,
/// `E-RCP-006`, `E-RCP-018`, or `E-RCP-055` (ADR-0025, ADR-0117).
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
    /// A bound of a `REAL` or `LREAL` leaf has more significant digits than the type holds.
    PrecisionExceeded {
        /// The path of the row.
        leaf: LeafPath,
        /// The bound with too many digits: `LimitMin` or `LimitMax`.
        field: PrecisionField,
        /// The type of the leaf.
        type_name: Primitive,
        /// The largest permitted count of significant digits.
        max_digits: u8,
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
            | Self::MinGreaterThanMax { leaf, .. }
            | Self::PrecisionExceeded { leaf, .. } => leaf,
        }
    }
}

/// The faults of one limit row, never empty.
///
/// Most rows give one fault. A `REAL` or `LREAL` row with too many digits in both bounds gives
/// one fault for each bound, so a form can show a line under each bound (ADR-0041, ADR-0117).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::recipe) struct LimitFaults {
    first: LimitFault,
    rest: Vec<LimitFault>,
}

impl LimitFaults {
    /// Build the list from the first fault and the faults that follow it.
    pub(in crate::recipe) const fn new(first: LimitFault, rest: Vec<LimitFault>) -> Self {
        Self { first, rest }
    }

    /// Return the first fault and drop the others.
    pub(in crate::recipe) fn into_first(self) -> LimitFault {
        self.first
    }

    /// Return every fault, the first one first.
    pub(in crate::recipe) fn into_vec(self) -> Vec<LimitFault> {
        std::iter::once(self.first).chain(self.rest).collect()
    }
}

impl From<LimitFault> for LimitFaults {
    fn from(fault: LimitFault) -> Self {
        Self::new(fault, Vec::new())
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
            Self::PrecisionExceeded {
                leaf,
                type_name,
                max_digits,
                ..
            } => write!(
                f,
                "A bound of the leaf \"{leaf}\" has too many significant digits for {type_name}. Use at most {max_digits} digits."
            ),
        }
    }
}
