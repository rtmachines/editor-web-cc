//! PostgreSQL adapters of the recipe editor.
//!
//! This crate implements the ports of `recipe-app` with `sqlx`.
//! It holds the unit of work, the security event sink, the session store, the login attempt
//! store, and the embedded migrations.
//! It has no public error type. It maps `sqlx::Error` into `StorageError` of `recipe-app`.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/data-model.md`.

#![forbid(unsafe_code)]

mod address;
mod audit_insert;
mod database;
mod error;
mod interval;
mod login_attempts;
mod migrator;
mod options;
mod security_events;
mod session_store;
pub mod settings;
mod transaction;
mod trigger_messages;
mod unit_of_work;

pub use database::Database;
pub use migrator::migrate;
#[cfg(feature = "test-support")]
pub use migrator::migrate_from_directory;
