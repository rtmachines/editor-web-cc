//! The adapter of the port `AlertWebhook` (ADR-0063, design D3 and D7).
//!
//! The job `alerts` sends one `POST` for each active condition. This module holds the URL of the
//! webhook and the TLS client. The URL is a secret. No error, `Display` text, or `Debug` text in
//! this module holds a part of the URL or of the path of the CA file. The module writes no log
//! line. The job writes the `WARN` line for a fault (ADR-0063 (n)).
//!
//! The client uses `hyper-rustls` with the `ring` provider and `https_only()`. It follows no
//! redirect, so a `3xx` status is a fault of the call.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::{CONTENT_TYPE, HeaderValue};
use hyper::{Method, Request, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use recipe_app::ports::{AlertCondition, AlertWebhook, WebhookFault};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use rustls::{ClientConfig, RootCertStore};
use tokio::task::JoinSet;
use tracing::Instrument;

use crate::config::webhook::WebhookUrl;

/// The time for all `POST` requests of one run (`alerts.webhook_timeout`, `reliability.md`
/// section 7.11).
pub const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(10);

/// The name of the key that holds the path of the CA file. A refusal names this key and no path.
const KEY_WEBHOOK_CA_FILE: &str = "alerts.webhook_ca_file";

/// The value of the header `Content-Type` of every call (ADR-0063 (l)).
const JSON_CONTENT_TYPE: &str = "application/json";

type HttpsClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

/// Why the client of the webhook cannot start.
///
/// The type solves one problem: a refusal of the start must name the key and never the path of
/// the CA file. No variant holds a path or a part of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClientFault {
    /// The CA file cannot be read. The kind comes from the operating system.
    CaUnreadable(ErrorKind),
    /// The CA file holds no certificate.
    CaWithoutCertificate,
    /// The CA file holds a certificate that the TLS client cannot read.
    CaBadCertificate,
    /// The TLS client cannot use the `ring` provider with the safe protocol versions.
    TlsSetup,
}

impl fmt::Display for ClientFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CaUnreadable(kind) => {
                write!(
                    f,
                    "the file of {KEY_WEBHOOK_CA_FILE} cannot be read: {kind}"
                )
            }
            Self::CaWithoutCertificate => {
                write!(f, "the file of {KEY_WEBHOOK_CA_FILE} holds no certificate")
            }
            Self::CaBadCertificate => write!(
                f,
                "the file of {KEY_WEBHOOK_CA_FILE} holds a certificate that cannot be read"
            ),
            Self::TlsSetup => f.write_str("the TLS client of the webhook cannot start"),
        }
    }
}

/// Read the certificates of the CA file into a store of trusted roots.
///
/// The store holds only these certificates. A file without a certificate is a refusal, because a
/// client with no root would fail every call.
fn read_roots(path: &Path) -> Result<RootCertStore, ClientFault> {
    let bytes = std::fs::read(path).map_err(|error| ClientFault::CaUnreadable(error.kind()))?;
    let mut roots = RootCertStore::empty();
    for item in CertificateDer::pem_slice_iter(&bytes) {
        let certificate = item.map_err(|_bad| ClientFault::CaBadCertificate)?;
        roots
            .add(certificate)
            .map_err(|_bad| ClientFault::CaBadCertificate)?;
    }
    if roots.is_empty() {
        return Err(ClientFault::CaWithoutCertificate);
    }
    Ok(roots)
}

/// Build the HTTPS connector (ADR-0063 (j), (k)).
///
/// Without a CA file the roots are the webpki roots. With a CA file the roots are the
/// certificates of the file and nothing else. Both connectors accept only the scheme `https`.
fn build_connector(ca_file: Option<&Path>) -> Result<HttpsConnector<HttpConnector>, ClientFault> {
    let provider: Arc<CryptoProvider> = Arc::new(rustls::crypto::ring::default_provider());
    let builder = HttpsConnectorBuilder::new();
    let with_roots = match ca_file {
        None => builder
            .with_provider_and_webpki_roots(provider)
            .map_err(|_bad| ClientFault::TlsSetup)?,
        Some(path) => {
            let roots = read_roots(path)?;
            let config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .map_err(|_bad| ClientFault::TlsSetup)?
                .with_root_certificates(roots)
                .with_no_client_auth();
            builder.with_tls_config(config)
        }
    };
    Ok(with_roots.https_only().enable_http1().build())
}

/// The adapter of the webhook: one URL, one TLS client, and one timeout for all calls of a run.
///
/// The type solves one problem: the job `alerts` must send to a URL that it never sees. The
/// `Debug` text is written by hand and hides the URL.
#[derive(Clone)]
pub struct WebhookClient {
    client: HttpsClient,
    uri: Uri,
    timeout: Duration,
}

impl WebhookClient {
    /// Build the client for the URL of the webhook file.
    ///
    /// `ca_file` is the path in `alerts.webhook_ca_file`, or `None` for the webpki roots.
    /// `timeout` is `alerts.webhook_timeout`. It bounds all calls of one run together.
    ///
    /// # Errors
    ///
    /// Returns an error that names the key `alerts.webhook_ca_file` and never the path, when the
    /// CA file cannot be read or holds no certificate that the TLS client accepts.
    pub fn new(url: &WebhookUrl, ca_file: Option<&Path>, timeout: Duration) -> Result<Self> {
        Self::from_uri(url.uri().clone(), ca_file, timeout)
    }

    fn from_uri(uri: Uri, ca_file: Option<&Path>, timeout: Duration) -> Result<Self> {
        let connector = build_connector(ca_file).map_err(|fault| anyhow!("{fault}"))?;
        // An alert run comes once for each `alerts.interval`, so an idle connection has no use.
        let client = Client::builder(TokioExecutor::new())
            .pool_max_idle_per_host(0)
            .build(connector);
        Ok(Self {
            client,
            uri,
            timeout,
        })
    }
}

impl fmt::Debug for WebhookClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebhookClient")
            .field("url", &format_args!("<redacted>"))
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl AlertWebhook for WebhookClient {
    async fn send_all(&self, conditions: &[AlertCondition]) -> Vec<Result<(), WebhookFault>> {
        let calls: Vec<_> = conditions
            .iter()
            .map(|condition| post(self.client.clone(), self.uri.clone(), *condition))
            .collect();
        run_bounded(calls, self.timeout).await
    }
}

/// The adapter for a plant with no webhook. Every entry is `Ok` and no call goes out.
///
/// The type solves one problem: the job `alerts` needs a port even when the operator sets no
/// webhook (ADR-0063 (a)).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoWebhook;

#[async_trait]
impl AlertWebhook for NoWebhook {
    async fn send_all(&self, conditions: &[AlertCondition]) -> Vec<Result<(), WebhookFault>> {
        vec![Ok(()); conditions.len()]
    }
}

/// Send one `POST` with the body `{"alert":"<name>"}` and read the status (ADR-0063 (l)).
///
/// The function drops the answer body, which closes the connection.
async fn post(
    client: HttpsClient,
    uri: Uri,
    condition: AlertCondition,
) -> Result<(), WebhookFault> {
    let body = format!(r#"{{"alert":"{}"}}"#, condition.name());
    let mut request = Request::new(Full::new(Bytes::from(body)));
    *request.method_mut() = Method::POST;
    *request.uri_mut() = uri;
    request
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(JSON_CONTENT_TYPE));
    let response = client
        .request(request)
        .await
        .map_err(|error| fault_of(&error))?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        Err(WebhookFault::Status {
            code: status.as_u16(),
        })
    }
}

/// Tell a TLS fault from any other fault of the connection.
///
/// A rustls error can sit behind several layers of `io::Error`. `io::Error::source` skips the
/// error that it wraps, so the loop also reads `get_ref`.
fn fault_of(error: &(dyn Error + 'static)) -> WebhookFault {
    let mut current = error;
    loop {
        if current.is::<rustls::Error>() {
            return WebhookFault::Tls;
        }
        let wrapped = current
            .downcast_ref::<io::Error>()
            .and_then(io::Error::get_ref)
            .map(|inner| inner as &(dyn Error + 'static));
        match wrapped.or_else(|| current.source()) {
            Some(next) => current = next,
            None => return WebhookFault::Connection,
        }
    }
}

/// Run all calls at the same time under one timeout (ADR-0063 (m)).
///
/// The result has one entry for each call, in the order of the argument. A call that is not
/// ready when the timeout ends gives `Timeout`, and the function drops it. A call that panics
/// gives `Connection` for its own entry. The other entries keep their results.
async fn run_bounded<F>(calls: Vec<F>, timeout: Duration) -> Vec<Result<(), WebhookFault>>
where
    F: Future<Output = Result<(), WebhookFault>> + Send + 'static,
{
    let mut results = vec![Err(WebhookFault::Timeout); calls.len()];
    let mut tasks = JoinSet::new();
    let mut slots = HashMap::new();
    for (index, call) in calls.into_iter().enumerate() {
        // A spawned task does not inherit the span. The call keeps the span `job` of the run.
        slots.insert(tasks.spawn(call.in_current_span()).id(), index);
    }
    let collect = async {
        while let Some(joined) = tasks.join_next_with_id().await {
            let (id, result) = match joined {
                Ok((id, result)) => (id, result),
                Err(join_error) => (join_error.id(), Err(WebhookFault::Connection)),
            };
            if let Some(slot) = slots.get(&id).and_then(|index| results.get_mut(*index)) {
                *slot = result;
            }
        }
    };
    // An elapsed timeout drops `collect` and keeps the results that arrived. The drop of the
    // set then aborts the calls that are still open.
    let _elapsed = tokio::time::timeout(timeout, collect).await;
    results
}

#[cfg(test)]
mod tests;
