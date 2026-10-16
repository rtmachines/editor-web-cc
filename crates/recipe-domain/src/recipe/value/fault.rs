//! The fault of a parameter value (design D8, ADR-0025).

use std::fmt;

use super::expected::Expected;
use crate::plant::{LeafPath, LeafSegment};

/// The path of a fault that concerns the whole value.
static WHOLE_VALUE: LeafPath = LeafPath::whole();

/// The expected form of a value that belongs to another root type than its tree.
static PARAMETER_TYPE: Expected = Expected::ParameterType;

/// The reason why a value does not match its type tree or its limits.
///
/// The enum has no `code()`. The draft adds the tag and wraps the fault (ADR-0025).
/// `Mismatch` becomes `E-RCP-005`. `OutOfLimit` becomes `E-RCP-004`. `LimitConflict` becomes
/// `E-RCP-019`. `WrongTree` becomes `E-RCP-005`. The enum can get more variants, so a caller
/// must not match it exhaustively.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValueFault {
    /// The value at a leaf path has not the form that the type needs.
    Mismatch {
        /// Path of the part that does not match. The empty path names the whole value.
        leaf: LeafPath,
        /// The form that the type needs.
        expected: Expected,
    },
    /// The tree or a limit belongs to another root type than the value.
    WrongTree,
    /// Two limits cover one leaf.
    LimitConflict {
        /// Concrete path of the leaf that two limits cover.
        leaf: LeafPath,
    },
    /// A value is outside the limits of its leaf.
    OutOfLimit {
        /// Concrete path of the leaf.
        leaf: LeafPath,
        /// The value as message text. A string leaf shows its length.
        value: String,
        /// The lower bound as message text. A missing bound shows the bound of the type.
        min: String,
        /// The upper bound as message text. A missing bound shows the bound of the type.
        max: String,
    },
}

impl ValueFault {
    /// Build a mismatch at the given path segments.
    pub(super) fn mismatch(segments: Vec<LeafSegment>, expected: Expected) -> Self {
        Self::Mismatch {
            leaf: LeafPath::from_segments(segments),
            expected,
        }
    }

    /// Return the path of the part that does not match.
    ///
    /// A fault of the whole value returns the empty path.
    #[must_use]
    pub const fn leaf(&self) -> &LeafPath {
        match self {
            Self::Mismatch { leaf, .. }
            | Self::LimitConflict { leaf }
            | Self::OutOfLimit { leaf, .. } => leaf,
            Self::WrongTree => &WHOLE_VALUE,
        }
    }

    /// Return the form that the type needs.
    ///
    /// A fault of the shape returns its form. A fault of a limit returns `None`.
    #[must_use]
    pub const fn expected(&self) -> Option<&Expected> {
        match self {
            Self::Mismatch { expected, .. } => Some(expected),
            Self::WrongTree => Some(&PARAMETER_TYPE),
            Self::LimitConflict { .. } | Self::OutOfLimit { .. } => None,
        }
    }
}

/// One or more faults of one value, in the order of the leaves (ADR-0041).
///
/// The type solves one problem: a draft command reports every fault of a value at once. The
/// list cannot be empty and owns no code. A draft function turns it into `RecipeErrors`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueFaults {
    first: ValueFault,
    rest: Vec<ValueFault>,
}

impl ValueFaults {
    /// Build a list from a vector. Return `None` for an empty vector.
    #[must_use]
    pub fn from_vec(faults: Vec<ValueFault>) -> Option<Self> {
        let mut entries = faults.into_iter();
        let first = entries.next()?;
        Some(Self {
            first,
            rest: entries.collect(),
        })
    }

    /// Return the first fault of the list.
    #[must_use]
    pub const fn first(&self) -> &ValueFault {
        &self.first
    }

    /// Return the number of faults. The count is never zero.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rest.len().saturating_add(1)
    }

    /// Return `false`. A list always holds at least one fault.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Iterate over the faults in order.
    pub fn iter(&self) -> impl Iterator<Item = &ValueFault> {
        std::iter::once(&self.first).chain(self.rest.iter())
    }

    /// Convert the list into a vector of its faults in order.
    #[must_use]
    pub fn into_vec(self) -> Vec<ValueFault> {
        std::iter::once(self.first).chain(self.rest).collect()
    }
}

impl From<ValueFault> for ValueFaults {
    fn from(fault: ValueFault) -> Self {
        Self {
            first: fault,
            rest: Vec::new(),
        }
    }
}

impl fmt::Display for ValueFaults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, fault) in self.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{fault}")?;
        }
        Ok(())
    }
}

impl fmt::Display for ValueFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mismatch { leaf, expected } => write!(
                f,
                "Value at leaf path \"{leaf}\" does not match the type. Expected {expected}."
            ),
            Self::WrongTree => {
                f.write_str("The tree or a limit belongs to another root type than the value.")
            }
            Self::LimitConflict { leaf } => {
                write!(f, "Two limits cover the leaf path \"{leaf}\".")
            }
            Self::OutOfLimit {
                leaf,
                value,
                min,
                max,
            } => write!(
                f,
                "Value {value} at leaf path \"{leaf}\" is outside the limits {min} to {max}."
            ),
        }
    }
}
