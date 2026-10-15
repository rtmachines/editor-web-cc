//! Web layer of the recipe editor.
//!
//! This crate holds the axum routers, the askama templates, the HTMX fragments, and the i18n bundles.
//! It sets the security headers and the session cookie.
//! The error type is `WebError`. It describes the HTTP shape only.
//! This crate does not use `sqlx` or a session library.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/api.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod client;
#[cfg(test)]
mod components_tests;
mod cookies;
mod csrf;
mod error;
mod gates;
mod hex;
mod i18n;
mod lang;
mod layout;
mod limits;
mod pages;
mod pipeline;
mod routes;
mod session;
mod settings;
mod state;
mod static_files;
#[cfg(test)]
mod style_tests;
#[cfg(test)]
mod template_tests;
#[cfg(test)]
mod test_support;

pub use csrf::{PreSession, issue_pre_session};
pub use error::{ByteLimit, FieldReason, RowField, WebError};
pub use i18n::{BundleReason, Bundles, Lang, LangReason};
pub use routes::{Access, router};
pub use settings::{Cidr, CidrList, CidrReason, WebSettings};
pub use state::{ReadOnlyMode, WebState};
pub use static_files::{HTMX_SHA256, HTMX_VERSION};
