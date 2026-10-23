//! In-memory adapter of the port `DocumentWriter` (design D2 of the change `export-v1`).

use recipe_domain::export::{DocumentFacts, ExportError, HashedBody};

use crate::ports::DocumentWriter;

/// A writer that writes only the hashed body as JSON and ignores the facts.
///
/// The type solves one problem: a test of a use case needs a writer, but `recipe-app` must not
/// depend on `recipe-export`. The bytes are not an export document of `api.md` section 6.1.
#[derive(Clone, Copy, Debug, Default)]
pub struct BodyWriter;

impl DocumentWriter for BodyWriter {
    fn write(&self, body: &HashedBody, _facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        Ok(serde_json::to_vec(body)?)
    }
}
