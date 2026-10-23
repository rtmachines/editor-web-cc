//! Test doubles of the port `DocumentWriter` (design D11 rows T6 and T7).

use std::sync::Arc;

use parking_lot::Mutex;
use recipe_domain::export::{DocumentFacts, ExportError, HashedBody};

use crate::ports::DocumentWriter;

/// The bytes that [`RecordingWriter`] returns as the document.
pub(super) const DOCUMENT: &[u8] = b"{\"recorded\":true}";

/// A writer that records each body and each set of facts and returns [`DOCUMENT`].
///
/// The type solves one problem: a test must see the facts that the flow gives to the writer.
#[derive(Debug, Default)]
pub(super) struct RecordingWriter {
    calls: Mutex<Vec<(HashedBody, DocumentFacts)>>,
}

impl RecordingWriter {
    /// Return a new writer in an `Arc`, so the test keeps a handle after it gives the port away.
    pub(super) fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Return every recorded call, the first call first.
    pub(super) fn calls(&self) -> Vec<(HashedBody, DocumentFacts)> {
        self.calls.lock().clone()
    }

    /// Return the last recorded call.
    pub(super) fn last(&self) -> (HashedBody, DocumentFacts) {
        self.calls().last().cloned().unwrap()
    }
}

impl DocumentWriter for RecordingWriter {
    fn write(&self, body: &HashedBody, facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        self.calls.lock().push((body.clone(), facts.clone()));
        Ok(DOCUMENT.to_vec())
    }
}

/// Return the port view of a recording writer.
pub(super) fn port(writer: &Arc<RecordingWriter>) -> Arc<dyn DocumentWriter> {
    Arc::<RecordingWriter>::clone(writer)
}

/// A writer whose every call returns `E-EXP-004`.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FailingWriter;

impl DocumentWriter for FailingWriter {
    fn write(&self, _body: &HashedBody, _facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        let error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        Err(ExportError::Serialize(error))
    }
}

/// The payload of the panic of [`PanickingWriter`].
#[derive(Clone, Copy, Debug)]
struct WriterPanic;

/// A writer whose every call panics.
///
/// The type solves one problem: a test must prove that a panic in the blocking task of the export
/// gives `E-WEB-011`. The panic unwinds without a message on stderr.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct PanickingWriter;

impl DocumentWriter for PanickingWriter {
    fn write(&self, _body: &HashedBody, _facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        std::panic::resume_unwind(Box::new(WriterPanic))
    }
}
