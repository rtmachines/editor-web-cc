//! The fault of a parameter value (design D8, ADR-0025).

use std::fmt;

use crate::plant::{LeafPath, LeafSegment};

/// The path of a fault that concerns the whole value.
static WHOLE_VALUE: LeafPath = LeafPath::whole();

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
        /// Short English text of the form that the type needs, for example `true or false`.
        expected: String,
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
    pub(super) fn mismatch(segments: Vec<LeafSegment>, expected: impl Into<String>) -> Self {
        Self::Mismatch {
            leaf: LeafPath::from_segments(segments),
            expected: expected.into(),
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

    /// Return the short text of the form that the type needs.
    #[must_use]
    pub fn expected(&self) -> &str {
        match self {
            Self::Mismatch { expected, .. } => expected,
            Self::WrongTree => "a tree and limits of the root type of the value",
            Self::LimitConflict { .. } => "one limit for each leaf",
            Self::OutOfLimit { .. } => "a value inside the limits",
        }
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
