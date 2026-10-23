//! Write port of the export document (design D6 (a) of the change `export-v1`).

use recipe_domain::export::{DocumentFacts, ExportError, HashedBody};

/// Writes the bytes of the export document from the hashed body and the facts outside the hash.
///
/// The port solves one problem: the export flow must give the document bytes, but `recipe-app`
/// must not depend on `recipe-export` and its output rules (`overview.md` section 5).
/// `recipe-web` gives an adapter over `recipe_export::write_document`. The flow calls the port
/// inside its blocking task, after the hash check.
pub trait DocumentWriter: Send + Sync {
    /// Write the export document of `body` with the values of `facts` (`api.md` section 6.1).
    ///
    /// # Errors
    ///
    /// Returns `ExportError::Serialize` (`E-EXP-004`) when the writer cannot write the document.
    fn write(&self, body: &HashedBody, facts: &DocumentFacts) -> Result<Vec<u8>, ExportError>;
}
