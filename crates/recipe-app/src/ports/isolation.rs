//! Isolation level of a transaction.

/// The isolation of a transaction that `UnitOfWork::begin` opens.
///
/// The type solves one problem: a use case names the isolation that its row in
/// `data-model.md` section 6 requires, and the adapter does not guess it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Isolation {
    /// The default level of a command (`READ COMMITTED`, no retry). `SetUserRole` and
    /// `DeactivateUser` use it after the guard lock of the last Administrator (ADR-0096).
    ReadCommitted,
    /// The level of `bootstrap_administrator` (`SERIALIZABLE`, one more run, ADR-0035 (h)).
    Serializable,
    /// The read before the transaction of a command. It takes no lock (design D5).
    ReadOnly,
    /// The level of a draft query: one snapshot, no lock, no retry (`REPEATABLE READ`, ADR-0042).
    RepeatableRead,
}
