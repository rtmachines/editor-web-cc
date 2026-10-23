//! Doubles of the port `DocumentWriter` for the export tests on the database (design D11 row T8).
//!
//! `recipe-app` keeps its own doubles in test code, so the database tests write theirs.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use recipe_app::ports::DocumentWriter;
use recipe_domain::export::{DocumentFacts, ExportError, HashedBody};

/// The bytes that [`RecordingWriter`] returns as the document.
const DOCUMENT: &[u8] = b"{\"recorded\":true}";

/// One call of a recording writer: the hashed body and the facts.
pub type Call = (HashedBody, DocumentFacts);

/// A writer that sends each body and each set of facts to a channel and returns [`DOCUMENT`].
///
/// A channel replaces a mutex: the sender is `Sync`, and the test keeps the receiver.
#[derive(Debug)]
pub struct RecordingWriter {
    calls: Sender<Call>,
}

impl RecordingWriter {
    /// Return the writer as a port and the receiver of its calls.
    pub fn port() -> (Arc<dyn DocumentWriter>, Receiver<Call>) {
        let (calls, received) = channel();
        (Arc::new(Self { calls }), received)
    }
}

impl DocumentWriter for RecordingWriter {
    fn write(&self, body: &HashedBody, facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        self.calls
            .send((body.clone(), facts.clone()))
            .expect("the test keeps the receiver");
        Ok(DOCUMENT.to_vec())
    }
}

/// A writer whose every call returns `ExportError::Serialize` (`E-EXP-004`).
#[derive(Clone, Copy, Debug, Default)]
pub struct FailingWriter;

impl FailingWriter {
    /// Return the writer as a port.
    pub fn port() -> Arc<dyn DocumentWriter> {
        Arc::new(Self)
    }
}

impl DocumentWriter for FailingWriter {
    fn write(&self, _body: &HashedBody, _facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        let error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        Err(ExportError::Serialize(error))
    }
}
