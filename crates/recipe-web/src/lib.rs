//! Web layer of the recipe editor.
//!
//! This crate holds the axum routers, the askama templates, the HTMX fragments, and the i18n bundles.
//! It sets the security headers and the session cookie.
//! The error type is `WebError`. It describes the HTTP shape only.
//! This crate does not use `sqlx` or a session library.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/api.md`.

#![forbid(unsafe_code)]
