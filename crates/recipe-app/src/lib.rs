//! Use cases of the recipe editor.
//!
//! This crate holds the use cases, the ports (traits), the permission checks, and the audit entries.
//! The permission check is the first statement of every use case. The supervised jobs are
//! internal operations without a permission (ADR-0060 (t)).
//! The error types are `AppError`, `AuditError`, and `StorageError`. `WebhookFault`,
//! `DiskFault`, and `BackupFault` are reason types without a code (ADR-0073 (b)).
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
pub mod jobs;
#[cfg(any(test, feature = "memory"))]
pub mod memory;
pub mod operations;
pub mod plant;
pub mod ports;
pub mod recipe;
