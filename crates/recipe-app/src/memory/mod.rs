//! In-memory adapters of the ports (design D2). This module is not test code.
//!
//! It follows every lint of `reliability.md` section 2. `recipe-server` never wires it.

mod audit;
mod clock;
mod core;
mod draft;
mod fault;
#[cfg(test)]
pub(crate) mod fixtures;
mod identity;
mod images;
mod plant;
mod recipe;
mod rows;
mod sink;
mod types;

pub use self::core::{
    DraftWrite, JournalEntry, Lock, LockMode, MemoryState, MemoryStore, MemoryTransaction,
    WriteKind,
};
pub use clock::MemoryClock;
pub use fault::{Fault, FaultPoint};
pub use identity::LOGIN_LOCK_TABLE;
pub use sink::MemorySink;
pub use types::{DERIVED_TYPE_LOCK_TABLE, seed_type_id};
