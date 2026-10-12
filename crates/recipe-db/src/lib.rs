//! PostgreSQL adapters of the recipe editor.
//!
//! This crate implements the ports of `recipe-app` with `sqlx`.
//! It holds the session store, the unit of work, and the embedded migrations.
//! It has no public error type. It maps `sqlx::Error` into `StorageError` of `recipe-app`.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/data-model.md`.

#![forbid(unsafe_code)]
