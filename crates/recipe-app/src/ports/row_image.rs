//! Row images for the audit entries (design D5 and D9, ADR-0029).

use serde_json::Value;

/// The full row of an entity as JSON, with the column names of `data-model.md` section 3.
///
/// The type solves one problem: PostgreSQL 16 has no `RETURNING OLD`, but an audit entry needs
/// the row before and after a change. A lock method gives the image before the change and
/// a write method gives the image after it. The storage adapter builds an image with
/// `to_jsonb`, and the in-memory adapter builds the same keys by hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowImage(Value);

impl RowImage {
    /// Wrap a JSON object that an adapter built from a row.
    #[must_use]
    pub const fn new(value: Value) -> Self {
        Self(value)
    }

    /// Return the JSON of the image.
    #[must_use]
    pub const fn as_value(&self) -> &Value {
        &self.0
    }

    /// Give the JSON of the image to the caller.
    #[must_use]
    pub fn into_value(self) -> Value {
        self.0
    }
}

/// A locked state together with the row image before any change.
///
/// The type solves one problem: a lock statement must return the state for the domain
/// method and the row for the audit entry in one round trip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locked<T> {
    /// The state that the lock statement read. The domain method runs on it.
    pub state: T,
    /// The row image before the command changes it.
    pub before: RowImage,
}
