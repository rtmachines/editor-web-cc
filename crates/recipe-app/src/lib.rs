//! Use cases of the recipe editor.
//!
//! This crate holds the use cases, the ports (traits), the permission checks, and the audit entries.
//! The permission check is the first statement of every use case.
//! The error types are `AppError`, `AuditError`, and `StorageError`.
//! This crate does not use `sqlx` or `axum`.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/domain-model.md` section 5.

#![forbid(unsafe_code)]
