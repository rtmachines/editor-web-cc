//! Use cases of the recipe editor.
//!
//! This crate holds the use cases, the ports (traits), the permission checks, and the audit entries.
//! The permission check is the first statement of every use case.
//! The error types are `AppError`, `AuditError`, and `StorageError`.
//! This crate does not use `sqlx` or `axum`.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/domain-model.md` section 5.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod audit;
pub mod auth;
pub mod blocking;
pub mod context;
pub mod error;
pub mod export;
pub mod identity;
#[cfg(any(test, feature = "memory"))]
pub mod memory;
pub mod plant;
pub mod ports;
pub mod recipe;
