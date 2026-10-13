//! A list with at least one element.

use std::iter;

/// A list that always holds at least one element.
///
/// The type solves one problem: `UnitOfWork::commit` must not accept a command
/// without an audit entry (I13). The constructors give no way to build an empty list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmpty<T> {
    /// Build a list with one element.
    #[must_use]
    pub const fn new(first: T) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    /// Build a list that holds `first_items` in order, then `last`.
    ///
    /// The result is never empty, because `last` is always present.
    #[must_use]
    pub fn with_last(first_items: Vec<T>, last: T) -> Self {
        let mut items = first_items.into_iter();
        match items.next() {
            Some(first) => Self {
                first,
                rest: items.chain(iter::once(last)).collect(),
            },
            None => Self::new(last),
        }
    }

    /// Add an element at the end of the list.
    pub fn push(&mut self, item: T) {
        self.rest.push(item);
    }

    /// Return the number of elements. The number is at least 1.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rest.len().saturating_add(1)
    }

    /// Return `false`. The list is never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Return the elements in order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        iter::once(&self.first).chain(self.rest.iter())
    }

    /// Return the first element.
    #[must_use]
    pub const fn first(&self) -> &T {
        &self.first
    }
}

/// The error of `NonEmpty::try_from` for an empty vector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptyVec;

impl<T> TryFrom<Vec<T>> for NonEmpty<T> {
    type Error = EmptyVec;

    /// Build a list from a vector. An empty vector gives `EmptyVec`.
    fn try_from(items: Vec<T>) -> Result<Self, Self::Error> {
        let mut items = items.into_iter();
        let first = items.next().ok_or(EmptyVec)?;
        Ok(Self {
            first,
            rest: items.collect(),
        })
    }
}

impl<T> From<NonEmpty<T>> for Vec<T> {
    /// Return the elements as a vector, in order.
    fn from(list: NonEmpty<T>) -> Self {
        let mut items = Vec::with_capacity(list.len());
        items.push(list.first);
        items.extend(list.rest);
        items
    }
}

#[cfg(test)]
mod tests;
