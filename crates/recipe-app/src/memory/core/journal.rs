//! Lock journal of the in-memory store.

use recipe_domain::recipe::ParameterKey;

use crate::ports::Isolation;

/// The strength of a lock that a use case takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockMode {
    /// `FOR UPDATE`.
    ForUpdate,
    /// `FOR NO KEY UPDATE`.
    ForNoKeyUpdate,
    /// `FOR SHARE`.
    ForShare,
    /// `FOR KEY SHARE`.
    ForKeyShare,
    /// `pg_advisory_xact_lock(class, key)`.
    Advisory,
}

/// One lock call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lock {
    /// The table that the lock names, or the lock class for an advisory lock.
    pub table: &'static str,
    /// The strength of the lock.
    pub mode: LockMode,
    /// The key of the locked row, in a text form that the caller chooses.
    pub key: String,
}

/// One line of the lock journal.
///
/// The journal solves one problem: the in-memory adapter has no real locks, but a
/// test must still check the lock order of ADR-0015.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalEntry {
    /// A transaction opened with this isolation.
    Begin(Isolation),
    /// A lock call.
    Lock(Lock),
}

/// The group of columns or rows that a draft write statement changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteKind {
    /// `UPDATE parameter` of the definition columns (`update_definition`).
    Definition,
    /// `UPDATE parameter SET value` (`set_value`).
    Value,
    /// `DELETE parameter_limit` and `INSERT parameter_limit` (`replace_limits`).
    Limits,
}

/// One write statement of the draft adapter on one parameter.
///
/// The log solves one problem: the state after a write cannot show that a statement did not
/// run when the new data equals the old data. A test of "writes only the definition columns"
/// reads this log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraftWrite {
    /// The statement group.
    pub kind: WriteKind,
    /// The key of the written parameter.
    pub key: ParameterKey,
}
