//! Step 2 of the start: the webhook client (ADR-0063 (a), (j), (k), design D4).
//!
//! The function [`build_webhook`] runs after `serve::load` has read the webhook file. It turns
//! the checked URL and the optional CA file into the port `AlertWebhook`. A refusal names the key
//! `alerts.webhook_ca_file` only. It never shows a path or a part of the URL.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use recipe_app::ports::AlertWebhook;

use crate::config::webhook::{WebhookUrl, parse_webhook_text};
use crate::webhook::{NoWebhook, WEBHOOK_TIMEOUT, WebhookClient};

/// The key of the CA file in the configuration and in every message about it.
const KEY_WEBHOOK_CA_FILE: &str = "alerts.webhook_ca_file";

/// A URL that only serves to test a CA file when the operator sets no webhook URL.
const CHECK_URL: &str = "https://localhost/";

/// Build the port of the webhook for the job `alerts`.
///
/// A CA file without a URL passes with one `WARN` line when the file holds a certificate. The
/// operator then sees the mistake and the server still starts.
///
/// # Errors
///
/// Returns an error that names the key `alerts.webhook_ca_file` when the CA file cannot be read or
/// holds no certificate that the TLS client accepts.
pub fn build_webhook(
    url: Option<&WebhookUrl>,
    ca_file: Option<&Path>,
) -> Result<Arc<dyn AlertWebhook>> {
    match (url, ca_file) {
        (Some(url), ca_file) => Ok(Arc::new(WebhookClient::new(url, ca_file, WEBHOOK_TIMEOUT)?)),
        (None, Some(ca_file)) => {
            check_ca_file(ca_file)?;
            tracing::warn!(
                key = KEY_WEBHOOK_CA_FILE,
                "the CA file is set, but the webhook has no URL: alerts have no webhook"
            );
            Ok(Arc::new(NoWebhook))
        }
        (None, None) => Ok(Arc::new(NoWebhook)),
    }
}

/// Read the CA file with the same code that the client uses.
fn check_ca_file(ca_file: &Path) -> Result<()> {
    let url = parse_webhook_text(CHECK_URL)
        .ok()
        .flatten()
        .ok_or_else(|| anyhow!("the check of {KEY_WEBHOOK_CA_FILE} cannot start"))?;
    WebhookClient::new(&url, Some(ca_file), WEBHOOK_TIMEOUT).map(|_client| ())
}

#[cfg(test)]
mod tests;
