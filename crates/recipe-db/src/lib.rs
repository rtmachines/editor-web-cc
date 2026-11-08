//! PostgreSQL adapters of the recipe editor.
//!
//! This crate implements the ports of `recipe-app` with `sqlx`.
//! It holds the unit of work, the security event sink, the session store, the login attempt
//! store, the maintenance store and the audit chain reads of the supervised jobs, the operations
//! reads and the readiness probe, and the embedded migrations.
//! It has no public error type. It maps `sqlx::Error` into `StorageError` of `recipe-app`.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/data-model.md`.

#![forbid(unsafe_code)]

mod address;
mod audit_chain;
mod audit_insert;
mod database;
mod error;
mod interval;
mod login_attempts;
mod maintenance;
mod migrator;
mod operations;
mod options;
mod security_events;
mod session_store;
pub mod settings;
mod transaction;
mod trigger_messages;
mod unit_of_work;

#[cfg(feature = "test-support")]
pub use database::SessionFacts;
pub use database::{CloseOutcome, Database};
pub use migrator::migrate;
#[cfg(feature = "test-support")]
pub use migrator::migrate_from_directory;

/// SQL texts that a storage test needs to inspect. The feature `test-support` enables it.
#[cfg(feature = "test-support")]
pub mod statements {
    /// The statement of `RecipeReads::parameter_page`.
    pub const PARAMETER_PAGE: &str = crate::transaction::parameter_reads::SELECT_PAGE;
    /// The statement of `TypeCatalog::lock_derived_types` that orders the class-2 lock keys.
    pub const DERIVED_LOCK_KEYS: &str = crate::transaction::type_rows::DERIVED_LOCK_KEYS;
}
