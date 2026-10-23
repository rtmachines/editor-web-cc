//! Export document of the recipe editor.
//!
//! This crate holds the export JSON Schema, the JSON writer of the HTTP response, and the golden tests.
//! It has no error type of its own. It returns `ExportError` of `recipe-domain`.
//! Only `recipe-web` uses this crate.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/api.md` section 6.

#![forbid(unsafe_code)]

mod document;
mod error_body;
mod file_name;

pub use document::write_document;
pub use error_body::error_body;
pub use file_name::file_name;
