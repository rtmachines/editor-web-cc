//! Shared code of the storage tests: the container, the roles, the connections, and fixtures.

pub mod app;
pub mod audit;
mod catalog;
mod constraint;
mod container;
mod database;
mod document;
pub mod fault;
pub mod fixtures;
pub mod gate;
pub mod probe;
mod session;
mod temp_files;
mod trigger;

pub use catalog::missing_names;
pub use constraint::{FOREIGN_KEY_VIOLATION, UNIQUE_VIOLATION, assert_constraint};
pub use container::{Role, server};
pub use database::TestDatabase;
pub use document::{created_names, document_sql, migration_sql, migration_texts};
pub use session::{connect, connect_error};
pub use temp_files::TempMigrations;
pub use trigger::{CHECK_VIOLATION, assert_privilege_denied, assert_trigger};
