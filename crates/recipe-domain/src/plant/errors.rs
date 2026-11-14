//! The carrier of several Plant Model errors (ADR-0027, ADR-0154).

use std::fmt;

use super::PlantError;
use crate::error_codes::ErrorCode;

/// One or more errors of the Plant Model context, in a fixed order (ADR-0027).
///
/// One connection request lists every field fault at once (ADR-0041).
/// The first entry is stored apart, so the list cannot be empty.
/// The first entry is boxed, so a `Result` of this type stays small (`clippy::result_large_err`).
/// The list owns no row of its own. `code` passes on the code of the first entry, as in `RecipeErrors` (ADR-0165).
#[derive(Debug)]
pub struct PlantErrors {
    first: Box<PlantError>,
    rest: Vec<PlantError>,
}

impl PlantErrors {
    /// Append an error after the existing entries.
    pub fn push(&mut self, error: PlantError) {
        self.rest.push(error);
    }

    /// Return the first entry of the list.
    #[must_use]
    pub fn first(&self) -> &PlantError {
        &self.first
    }

    /// Return the code of the first entry (`reliability.md` section 1.2).
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.first.code()
    }

    /// Return the number of entries. The count is never zero.
    #[must_use]
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Return `false`. A list always holds at least one entry.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Iterate over the entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &PlantError> {
        std::iter::once(&*self.first).chain(self.rest.iter())
    }

    /// Convert the list into a vector of its entries in insertion order.
    #[must_use]
    pub fn into_vec(self) -> Vec<PlantError> {
        std::iter::once(*self.first).chain(self.rest).collect()
    }

    /// Build a list from a vector. Return `None` for an empty vector.
    #[must_use]
    pub fn from_vec(errors: Vec<PlantError>) -> Option<Self> {
        let mut entries = errors.into_iter();
        let first = entries.next()?;
        Some(Self {
            first: Box::new(first),
            rest: entries.collect(),
        })
    }
}

impl From<PlantError> for PlantErrors {
    fn from(error: PlantError) -> Self {
        Self {
            first: Box::new(error),
            rest: Vec::new(),
        }
    }
}

impl Extend<PlantError> for PlantErrors {
    /// Append errors after the existing entries, in their order.
    fn extend<I: IntoIterator<Item = PlantError>>(&mut self, errors: I) {
        self.rest.extend(errors);
    }
}

impl fmt::Display for PlantErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for PlantErrors {}

#[cfg(test)]
mod tests;
