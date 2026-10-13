//! In-memory unit of work with a lock journal (design D5).

mod journal;
mod state;
mod store;
#[cfg(test)]
mod test_support;
mod transaction;

pub use journal::{DraftWrite, JournalEntry, Lock, LockMode, WriteKind};
pub use state::MemoryState;
pub use store::MemoryStore;
pub use transaction::MemoryTransaction;

#[cfg(test)]
mod tests;
