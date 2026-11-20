//! Doubles of the port `DocumentWriter` for the export tests on the database (design D11 row T8).
//!
//! `recipe-app` keeps its own doubles in test code, so the database tests write theirs.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use recipe_app::ports::DocumentWriter;
use recipe_domain::export::{
    DocumentFacts, ExportError, HashedBody, ScopeBody, ScopeFacts, Sha256,
};

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

    fn write_scoped(&self, _body: &ScopeBody, _facts: &ScopeFacts) -> Result<Vec<u8>, ExportError> {
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

/// Return the `E-EXP-004` error that the failing writer gives.
fn serialize_error() -> ExportError {
    let error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    ExportError::Serialize(error)
}

impl DocumentWriter for FailingWriter {
    fn write(&self, _body: &HashedBody, _facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        Err(serialize_error())
    }

    fn write_scoped(&self, _body: &ScopeBody, _facts: &ScopeFacts) -> Result<Vec<u8>, ExportError> {
        Err(serialize_error())
    }
}

/// A writer that writes the hashed body and every fact of the document, except the export instant.
///
/// The type solves one problem: `recipe-db` must not depend on `recipe-export` (`overview.md`
/// section 5, `deny.toml`), so no production writer reaches the database tests. The real writer
/// is a pure function of the body and the facts. This writer keeps all of them, so a test sees a
/// change of `loadable`, the status, the stored hash, or any name. The instant `exported_at`
/// changes in each export, so the bytes leave it out.
///
/// Each document holds the canonical bytes of the body (`canonical_body`) and its hash
/// (`body_sha256`). A test compares these two values before and after an upgrade, so a change of
/// the body is visible byte for byte. The document of a scope export also holds `scope_sha256`.
#[derive(Clone, Copy, Debug, Default)]
pub struct DocumentFactsWriter;

impl DocumentFactsWriter {
    /// Return the writer as a port.
    pub fn port() -> Arc<dyn DocumentWriter> {
        Arc::new(Self)
    }
}

/// The fields that the document of a whole version and the document of a scope share.
fn shared_fields(
    facts: &DocumentFacts,
    canonical_body: String,
    body_sha256: Sha256,
) -> serde_json::Map<String, serde_json::Value> {
    let names: Vec<(String, String)> = facts
        .names
        .iter()
        .map(|(key, name)| (format!("{key:?}"), name.to_string()))
        .collect();
    let document = serde_json::json!({
        "loadable": facts.loadable(),
        "content_sha256": facts.content_sha256.to_string(),
        "status": format!("{:?}", facts.status),
        "has_rule_fault": facts.has_rule_fault,
        "exported_by": format!("{:?}", facts.exported_by),
        "approved_at": format!("{:?}", facts.approved_at),
        "approved_by": format!("{:?}", facts.approved_by),
        "machine_name": facts.machine_name.to_string(),
        "recipe_name": facts.recipe_name.to_string(),
        "names": names,
        "canonical_body": canonical_body,
        "body_sha256": body_sha256.to_string(),
    });
    document
        .as_object()
        .cloned()
        .expect("the json! macro of an object gives an object")
}

impl DocumentWriter for DocumentFactsWriter {
    fn write(&self, body: &HashedBody, facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        let mut fields = shared_fields(facts, body.canonical_text()?, body.sha256()?);
        fields.insert("body".to_owned(), serde_json::to_value(body)?);
        Ok(serde_json::to_vec_pretty(&fields)?)
    }

    fn write_scoped(&self, body: &ScopeBody, facts: &ScopeFacts) -> Result<Vec<u8>, ExportError> {
        let mut fields = shared_fields(&facts.document, body.canonical_text()?, body.sha256()?);
        fields.insert(
            "scope_sha256".to_owned(),
            serde_json::Value::String(facts.scope_sha256.to_string()),
        );
        fields.insert("body".to_owned(), serde_json::to_value(body)?);
        Ok(serde_json::to_vec_pretty(&fields)?)
    }
}
