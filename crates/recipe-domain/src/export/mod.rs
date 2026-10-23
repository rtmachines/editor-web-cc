//! The module `export` of the Export context (`domain-model.md` section 1, ADR-0019).
//!
//! The module builds the hashed body of an export document and computes its SHA-256 hash.
//! It also holds the fields outside the hash and the time text of the document.

mod body;
mod error;
mod facts;
mod hash;
mod parameters;
mod sha256;
#[cfg(test)]
mod test_rows;
#[cfg(test)]
mod test_support;
mod type_reference;
mod types;

pub use body::{ExportHeader, HashedBody};
pub use error::ExportError;
pub use facts::{DocumentFacts, ExportStatus, SCHEMA_VERSION, utc_text};
pub use parameters::{LimitEntry, OrderedValue, ParameterEntry};
pub use sha256::{Sha256, Sha256Reason};
pub use type_reference::TypeReference;
pub use types::TypeEntry;

/// Serialize a value as the JSON string of its `Display` text.
///
/// A typed field of the hashed body uses this function, so its text equals the text of a `String`
/// field and `content_sha256` does not change.
fn display_text<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: std::fmt::Display,
    S: serde::Serializer,
{
    serializer.collect_str(value)
}

/// Serialize an optional value as the JSON string of its `Display` text, or as `null`.
///
/// An optional typed field of the hashed body uses this function. Its text equals the text of an
/// `Option<String>` field, so `content_sha256` does not change.
fn optional_display_text<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: std::fmt::Display,
    S: serde::Serializer,
{
    match value {
        Some(inner) => serializer.collect_str(inner),
        None => serializer.serialize_none(),
    }
}
