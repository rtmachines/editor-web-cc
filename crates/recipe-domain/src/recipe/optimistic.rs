//! The result of a command that compares a row version (ADR-0008, ADR-0028).

/// Result of a command that checks the row version of its target.
///
/// The type solves one problem: two users change the same version at the same time (optimistic concurrency).
/// A stale row version is an expected outcome and not an error, so the command returns it as a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Optimistic<T> {
    /// The row version matched and the command changed the aggregate.
    Applied(T),
    /// The row version did not match. The command changed nothing.
    Conflict {
        /// The row version that the caller sent.
        expected: i64,
        /// The row version that the aggregate holds.
        actual: i64,
    },
}
