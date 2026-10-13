//! Port of the current time.

use recipe_domain::timestamp::Timestamp;

/// Source of the current instant.
///
/// The port solves one problem: a use case that reads the system clock cannot be
/// tested at a fixed time. A domain method takes the instant as an argument.
pub trait Clock: Send + Sync {
    /// Return the current instant.
    fn now(&self) -> Timestamp;
}
