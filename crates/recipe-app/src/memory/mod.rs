//! In-memory adapters of the ports (design D2). This module is not test code.
//!
//! It follows every lint of `reliability.md` section 2. `recipe-server` never wires it.

mod audit;
mod audit_chain;
mod backups;
mod call_faults;
mod clock;
mod core;
mod disk;
mod document_writer;
mod draft;
mod fault;
#[cfg(test)]
pub(crate) mod fixtures;
mod identity;
mod images;
mod instants;
#[cfg(test)]
mod job_adapters_tests;
mod maintenance;
mod operations;
mod plant;
mod read_only;
mod readiness;
mod recipe;
mod rows;
mod sink;
#[cfg(test)]
mod testkit;
mod types;
mod webhook;

pub use self::core::{
    DraftWrite, JournalEntry, Lock, LockMode, MemoryState, MemoryStore, MemoryTransaction,
    WriteKind,
};
pub use audit_chain::{AuditChainMethod, ChainBreak, MemoryAuditChain};
pub use backups::MemoryBackupDirectory;
pub use clock::MemoryClock;
pub use disk::{DiskFailure, DiskReading, MemoryDiskSpace};
pub use document_writer::BodyWriter;
pub use fault::{Fault, FaultPoint};
pub use identity::LOGIN_LOCK_TABLE;
pub use maintenance::{MaintenanceMethod, MemoryMaintenance, SECURITY_PURGE_BATCH};
pub use operations::{MemoryOperations, OperationsMethod};
pub use read_only::MemoryReadOnly;
pub use readiness::MemoryReadiness;
pub use sink::MemorySink;
pub use types::{DERIVED_TYPE_LOCK_TABLE, seed_type_id};
pub use webhook::MemoryWebhook;
