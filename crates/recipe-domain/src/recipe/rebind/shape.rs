//! The array shapes that a rebind needs (design D13, step 8).

use crate::plant::{ArrayDimensions, PlcTypeId};

/// The shape of an array whose element is the target struct of a rebind.
///
/// The type solves one problem: the use case creates each derived array type once, so the plan
/// lists each shape once. The dimensions run from the outer array to the inner array. The
/// element is the target struct.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArrayShape {
    dimensions: Vec<ArrayDimensions>,
    element: PlcTypeId,
}

impl ArrayShape {
    /// Build a shape from the dimensions of an array chain and the target struct.
    pub(super) const fn new(dimensions: Vec<ArrayDimensions>, element: PlcTypeId) -> Self {
        Self {
            dimensions,
            element,
        }
    }

    /// Return the dimensions of each array level, from the outer to the inner level.
    #[must_use]
    pub fn dimensions(&self) -> &[ArrayDimensions] {
        &self.dimensions
    }

    /// Return the target struct, which is the element of the innermost array.
    #[must_use]
    pub const fn element(&self) -> PlcTypeId {
        self.element
    }
}
